//! The system's light or dark choice (DESIGN 7.9), for a desktop where the
//! window system does not say it (winit on X11 answers nothing), read as the
//! C# version reads it: on Linux `GTK_THEME` (as GTK programs honour it), the
//! desktop portal (org.freedesktop.appearance color-scheme), else GNOME's
//! settings; on Windows the registry's AppsUseLightTheme; on macOS the
//! defaults' AppleInterfaceStyle. A program that is missing, slow or says
//! nothing leaves light, so reading it never stops the application.

use std::process::{Command, Stdio};
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
