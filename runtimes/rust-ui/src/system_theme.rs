//! The system's light or dark choice (DESIGN 7.9), for a desktop where the
//! window system does not say it (winit on X11 answers nothing), read as the
//! C# version reads it: on Linux `GTK_THEME` (as GTK programs honour it), the
//! desktop portal (org.freedesktop.appearance color-scheme), else GNOME's
//! settings; on Windows the registry's AppsUseLightTheme; on macOS the
//! defaults' AppleInterfaceStyle. A program that is missing, slow or says
//! nothing leaves light, so reading it never stops the application. A
//! change is followed by the system's own event where it gives one (the
//! desktop host listens: the portal's SettingChanged on Linux, the registry's
//! change on Windows), else by reading it every few seconds (`follow`).

use std::process::{Command, Stdio};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

/// The system a program runs on, as far as its light or dark is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum System {
    Linux,
    Windows,
    Macos,
    Other,
}

impl System {
    /// The system this runs on.
    pub fn here() -> System {
        if cfg!(target_os = "linux") {
            System::Linux
        } else if cfg!(windows) {
            System::Windows
        } else if cfg!(target_os = "macos") {
            System::Macos
        } else {
            System::Other
        }
    }
}

/// Whether the system shows dark: `gtk_theme_env` is the environment's
/// GTK_THEME, `run` runs a program and gives what it printed (none when it
/// is missing, fails or is slow). Light when nothing answers.
pub fn dark(system: System, gtk_theme_env: Option<&str>, run: &dyn Fn(&str, &[&str]) -> Option<String>) -> bool {
    match system {
        System::Linux => {
            if let Some(theme) = gtk_theme_env.filter(|t| !t.is_empty()) {
                return theme.to_lowercase().contains("dark");
            }
            run("gdbus", &["call", "--session", "--dest", "org.freedesktop.portal.Desktop", "--object-path", "/org/freedesktop/portal/desktop", "--method", "org.freedesktop.portal.Settings.Read", "org.freedesktop.appearance", "color-scheme"])
                .and_then(|s| portal(&s))
                .or_else(|| run("gsettings", &["get", "org.gnome.desktop.interface", "color-scheme"]).and_then(|s| color_scheme(&s)))
                .or_else(|| run("gsettings", &["get", "org.gnome.desktop.interface", "gtk-theme"]).and_then(|s| gtk_theme(&s)))
                .unwrap_or(false)
        }
        System::Windows => run("reg", &["query", r"HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize", "/v", "AppsUseLightTheme"]).and_then(|s| registry(&s)).unwrap_or(false),
        System::Macos => run("defaults", &["read", "-g", "AppleInterfaceStyle"]).is_some_and(|s| s.trim() == "Dark"),
        System::Other => false,
    }
}

/// Whether the system this runs on shows dark now.
pub fn dark_here() -> bool {
    dark(System::here(), std::env::var("GTK_THEME").ok().as_deref(), &run)
}

/// The portal's color-scheme (`(<<uint32 1>>,)`): 1 prefers dark, 2 prefers light, 0 says nothing.
pub fn portal(said: &str) -> Option<bool> {
    let at = said.find("uint32 ")? + "uint32 ".len();
    let digits: String = said[at..].chars().take_while(char::is_ascii_digit).collect();
    match digits.as_str() {
        "1" => Some(true),
        "2" => Some(false),
        _ => None,
    }
}

/// GNOME's color-scheme: `'prefer-dark'`, `'prefer-light'` or `'default'`.
pub fn color_scheme(said: &str) -> Option<bool> {
    match said.trim().trim_matches('\'') {
        "prefer-dark" => Some(true),
        "prefer-light" => Some(false),
        _ => None,
    }
}

/// A GTK theme named dark (`'Adwaita-dark'`) is dark; another says nothing.
pub fn gtk_theme(said: &str) -> Option<bool> {
    said.to_lowercase().contains("dark").then_some(true)
}

/// Windows' AppsUseLightTheme: 0 is dark, 1 light; none when the value is not there.
pub fn registry(said: &str) -> Option<bool> {
    let at = said.find("AppsUseLightTheme")?;
    let rest = &said[at..];
    let hex = rest.find("0x")? + 2;
    let digits: String = rest[hex..].chars().take_while(char::is_ascii_hexdigit).collect();
    if digits.is_empty() { None } else { Some(digits != "1") }
}

/// What a program prints when it ends well within two seconds; none when it is missing, fails or is slow.
fn run(program: &str, args: &[&str]) -> Option<String> {
    let mut child = Command::new(program).args(args).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().ok()?;
    let end = Instant::now() + Duration::from_secs(2);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let mut out = String::new();
                std::io::Read::read_to_string(child.stdout.as_mut()?, &mut out).ok()?;
                return status.success().then_some(out);
            }
            Ok(None) if Instant::now() < end => std::thread::sleep(Duration::from_millis(20)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
}

/// A watch over the system's light or dark (see [`watch`]); dropping it stops it.
pub struct Watch {
    stop: std::sync::mpsc::Sender<()>,
}

impl Drop for Watch {
    fn drop(&mut self) {
        let _ = self.stop.send(());
    }
}

/// Asks the system whether it shows dark every `every` on a thread of its
/// own, as C#'s SystemTheme.Watch does where the window system tells no
/// change (X11); `changed` hears it when it differs from `now`, once per change.
pub fn watch(now: bool, every: Duration, changed: impl Fn(bool) + Send + 'static) -> Watch {
    watch_with(now, every, dark_here, changed)
}

/// [`watch`] with what asks the system given: for tests.
pub fn watch_with(now: bool, every: Duration, ask: impl Fn() -> bool + Send + 'static, changed: impl Fn(bool) + Send + 'static) -> Watch {
    let (stop, stopped) = std::sync::mpsc::channel::<()>();
    std::thread::spawn(move || {
        let mut last = now;
        // Ends when the watch is dropped (a message, or the sender gone).
        while let Err(std::sync::mpsc::RecvTimeoutError::Timeout) = stopped.recv_timeout(every) {
            let dark = ask();
            if dark != last {
                last = dark;
                changed(dark);
            }
        }
    });
    Watch { stop }
}

/// What the system's change event calls each time its light or dark may have changed.
pub type Heard = Arc<dyn Fn() + Send + Sync>;

/// Listening to the system's change event; dropping it stops listening.
pub type Listening = Box<dyn Send>;

/// Starts listening to the system's change event, calling what it is given
/// each time; none where the system has none, an error where it cannot be
/// listened to now (no bus).
pub type Listen = Box<dyn FnOnce(Heard) -> Result<Option<Listening>, String> + Send>;

/// The system's choice followed (see [`follow`]): by its event or, where it
/// has none, by reading it every so often. Dropping it stops.
pub struct Following {
    shared: Arc<FollowShared>,
}

struct FollowShared {
    state: Mutex<FollowState>,
    started: Condvar,
    ask: Box<dyn Fn() -> bool + Send + Sync>,
    changed: Box<dyn Fn(bool) + Send + Sync>,
}

#[derive(Default)]
struct FollowState {
    last: bool,
    started: bool,
    stopped: bool,
    by_event: bool,
    listening: Option<Listening>,
    watching: Option<Watch>,
}

impl FollowShared {
    /// The choice read again differs from the last one: it is told once.
    fn changed(&self, dark: bool) {
        {
            let Ok(mut state) = self.state.lock() else { return };
            if state.stopped || dark == state.last {
                return;
            }
            state.last = dark;
        }
        (self.changed)(dark);
    }
}

impl Following {
    /// Waits until it listens to the event or reads every so often, at most `timeout`; whether it started.
    pub fn wait_started(&self, timeout: Duration) -> bool {
        let Ok(state) = self.shared.state.lock() else { return false };
        match self.shared.started.wait_timeout_while(state, timeout, |s| !s.started) {
            Ok((state, _)) => state.started,
            Err(_) => false,
        }
    }

    /// Whether it follows the system's event; false while it reads every so often (or has not started).
    pub fn by_event(&self) -> bool {
        self.shared.state.lock().map(|s| s.by_event).unwrap_or(false)
    }
}

impl Drop for Following {
    fn drop(&mut self) {
        let (listening, watching) = match self.shared.state.lock() {
            Ok(mut state) => {
                state.stopped = true;
                (state.listening.take(), state.watching.take())
            }
            Err(_) => (None, None),
        };
        drop(listening);
        drop(watching);
    }
}

/// Follows the system's choice by its change event: each time `listen`'s
/// event comes, the choice is read again (`ask`, off the thread that said
/// it) and `changed` hears it when it differs from the last. Where the
/// system gives no event (`listen` gives none, or fails) the choice is read
/// every `every` instead. Listening starts on a thread of its own, so a slow
/// system never holds the screen.
pub fn follow(now: bool, ask: impl Fn() -> bool + Send + Sync + 'static, listen: Listen, every: Duration, changed: impl Fn(bool) + Send + Sync + 'static) -> Following {
    let shared = Arc::new(FollowShared { state: Mutex::new(FollowState { last: now, ..FollowState::default() }), started: Condvar::new(), ask: Box::new(ask), changed: Box::new(changed) });
    let starting = shared.clone();
    std::thread::spawn(move || {
        let weak = Arc::downgrade(&starting);
        let heard: Heard = Arc::new(move || {
            let Some(shared) = weak.upgrade() else { return };
            // Read again off the thread that said it, which may be the bus's.
            std::thread::spawn(move || {
                let dark = (shared.ask)();
                shared.changed(dark);
            });
        });
        let listening = match listen(heard) {
            Ok(listening) => listening,
            Err(why) => {
                // Fail safe: an event that cannot be listened to leaves reading every so often.
                eprintln!("{}", crate::locale::fill(&crate::locale::translate("systemThemeNotHeard", "the system's light or dark change cannot be heard: {why}"), "why", &why));
                None
            }
        };
        let Ok(mut state) = starting.state.lock() else { return };
        state.by_event = listening.is_some();
        match listening {
            Some(listening) => state.listening = Some(listening),
            None => {
                let weak = Arc::downgrade(&starting);
                let asking = Arc::downgrade(&starting);
                let last = state.last;
                state.watching = Some(watch_with(last, every, move || asking.upgrade().map(|s| (s.ask)()).unwrap_or(last), move |dark| {
                    if let Some(shared) = weak.upgrade() {
                        shared.changed(dark);
                    }
                }));
            }
        }
        if state.stopped {
            state.listening = None;
            state.watching = None;
        }
        state.started = true;
        starting.started.notify_all();
    });
    Following { shared }
}
