//! The system's own file dialogs on the desktop (DESIGN 7.9): choose a file
//! to open, where a copy is saved, a folder; and a file opened in the
//! program the system uses for its kind. Each system its own way, with no
//! crate of its own: Linux asks the desktop portal over D-Bus
//! (org.freedesktop.portal.FileChooser), then zenity, then kdialog; macOS
//! asks osascript (AppleScript's choose file, choose file name, choose
//! folder); Windows shows the common item dialog through COM. Each call
//! waits for the person, so parts ask through `Cx::later`. Each dialog
//! belongs to the window the person works in, which takes no input while it
//! is open.

#[cfg(unix)]
pub mod portal;
#[cfg(windows)]
mod windows;

use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use crate::files::{safe_name, ChosenFile, FileFilter, FilePicker};

/// What a dialog chooses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Open,
    Save,
    Folder,
}

/// A dialog to show: what it chooses, its title, the name it suggests (saving) and the kinds of file it offers (opening).
#[derive(Debug, Clone, PartialEq)]
pub struct Ask {
    pub kind: Kind,
    pub title: String,
    pub name: Option<String>,
    pub filter: Option<FileFilter>,
}

/// The systems Clarity knows the dialogs of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    Linux,
    MacOs,
    Windows,
}

impl Os {
    /// The system this runs on.
    pub fn current() -> Os {
        if cfg!(windows) {
            Os::Windows
        } else if cfg!(target_os = "macos") {
            Os::MacOs
        } else {
            Os::Linux
        }
    }
}

/// A program to start: its name and arguments.
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub program: String,
    pub args: Vec<String>,
}

impl Program {
    fn new(program: &str, args: Vec<String>) -> Program {
        Program { program: program.to_string(), args }
    }
}

/// A desktop window as the system knows it, which a file dialog belongs to:
/// its X11 window, or its Win32 window handle; zero for none.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SystemWindow {
    pub x11: u64,
    pub win32: isize,
}

impl SystemWindow {
    /// The window as the desktop portal names a parent: `x11:` and its number in hex; empty for none.
    pub fn portal(&self) -> String {
        if self.x11 != 0 { format!("x11:{:x}", self.x11) } else { String::new() }
    }
}

/// How a system shows its dialogs, in the order they are tried.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Way {
    Portal,
    Commands,
    CommonItemDialog,
}

/// The ways `os` shows its dialogs; the first that answers wins.
pub fn ways(os: Os) -> &'static [Way] {
    match os {
        Os::Windows => &[Way::CommonItemDialog],
        Os::MacOs => &[Way::Commands],
        Os::Linux => &[Way::Portal, Way::Commands],
    }
}

/// What shows a dialog and gives the path chosen: the system's own, or a test's.
pub type Chooser = Box<dyn Fn(&Ask, Option<SystemWindow>) -> Option<String> + Send + Sync>;

/// What starts a program and lets it run.
pub type Starter = Box<dyn Fn(&Program) + Send + Sync>;

/// The file dialogs of the system this runs on. Each dialog belongs to the
/// window the person works in (its owner, where the system lets a dialog
/// name one), and while one is open the windows take no input, as under a
/// dialog of their own (`showing`). A dialog waits off the screen's thread
/// (the part asks through `Cx::later`), so the windows go on drawing.
pub struct SystemFilePicker {
    choose: Chooser,
    run: Starter,
    owner: Mutex<Option<SystemWindow>>,
    showing: AtomicUsize,
}

impl Default for SystemFilePicker {
    fn default() -> SystemFilePicker {
        SystemFilePicker::with(Box::new(|ask, owner| choose(Os::current(), ask, owner)), Box::new(start))
    }
}

/// A dialog open: the count of open dialogs goes down when it ends, however it ends.
struct Open<'a>(&'a AtomicUsize);

impl Drop for Open<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

impl SystemFilePicker {
    /// A picker whose dialogs `choose` answers and whose programs `run` starts: a test's.
    pub fn with(choose: Chooser, run: Starter) -> SystemFilePicker {
        SystemFilePicker { choose, run, owner: Mutex::new(None), showing: AtomicUsize::new(0) }
    }

    /// The window the person works in, which a dialog belongs to; none while there is none.
    pub fn set_owner(&self, owner: Option<SystemWindow>) {
        if let Ok(mut now) = self.owner.lock() {
            *now = owner;
        }
    }

    /// Whether a dialog is open: the windows take no input meanwhile.
    pub fn showing(&self) -> bool {
        self.showing.load(Ordering::SeqCst) > 0
    }

    /// The path chosen in a dialog over the owner, counted as open while it shows.
    fn ask(&self, ask: Ask) -> Option<String> {
        let owner = self.owner.lock().ok().and_then(|o| *o);
        self.showing.fetch_add(1, Ordering::SeqCst);
        let _open = Open(&self.showing);
        (self.choose)(&ask, owner)
    }
}

impl FilePicker for SystemFilePicker {
    fn picks_folders(&self) -> bool {
        true
    }

    fn open_file(&self, title: &str, filter: Option<&FileFilter>) -> Option<ChosenFile> {
        let path = self.ask(Ask { kind: Kind::Open, title: title.to_string(), name: None, filter: filter.cloned() })?;
        let bytes = std::fs::read(&path).map_err(|e| eprintln!("{path}: {e}")).ok()?;
        let name = std::path::Path::new(&path).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        Some(ChosenFile { name, bytes })
    }

    fn save_file(&self, title: &str, name: &str, bytes: &[u8]) -> Option<String> {
        let path = self.ask(Ask { kind: Kind::Save, title: title.to_string(), name: Some(name.to_string()), filter: None })?;
        match std::fs::write(&path, bytes) {
            Ok(()) => Some(path),
            Err(e) => {
                eprintln!("{path}: {e}");
                None
            }
        }
    }

    fn choose_folder(&self, title: &str) -> Option<String> {
        self.ask(Ask { kind: Kind::Folder, title: title.to_string(), name: None, filter: None })
    }

    /// Opens a copy of the file, in a folder of its own among the system's temporary files, in the system's program for its kind.
    fn show(&self, name: &str, bytes: &[u8]) {
        let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or_default();
        let folder = std::env::temp_dir().join("clarity-shown").join(format!("{}-{stamp}", std::process::id()));
        let path = folder.join(safe_name(name));
        let written = std::fs::create_dir_all(&folder).and_then(|_| std::fs::write(&path, bytes));
        if let Err(e) = written {
            eprintln!("{name} could not be shown: {e}");
            return;
        }
        (self.run)(&show_command(Os::current(), &path.to_string_lossy()));
    }
}

/// What opens the file at `path` in the system's program for its kind.
pub fn show_command(os: Os, path: &str) -> Program {
    match os {
        Os::Windows => Program::new("cmd", vec!["/C".into(), "start".into(), String::new(), path.into()]),
        Os::MacOs => Program::new("open", vec![path.into()]),
        Os::Linux => Program::new("xdg-open", vec![path.into()]),
    }
}

/// Starts a program and lets it run; one that cannot start is said.
fn start(program: &Program) {
    if let Err(e) = Command::new(&program.program).args(&program.args).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn() {
        eprintln!("{} could not be started: {e}", program.program);
    }
}

/// The path the person chose in `os`'s dialog over `owner`; none when they chose none, or no dialog could be shown.
pub fn choose(os: Os, ask: &Ask, owner: Option<SystemWindow>) -> Option<String> {
    for way in ways(os) {
        let answered = match way {
            #[cfg(windows)]
            Way::CommonItemDialog => match windows::choose(ask, owner) {
                Ok(path) => Some(path),
                Err(why) => {
                    eprintln!("the file dialog could not be shown: {why}");
                    None
                }
            },
            #[cfg(unix)]
            Way::Portal => match portal::choose(ask, owner) {
                Ok(path) => Some(path),
                // No session bus, or no portal on it: the next way is tried.
                Err(why) => {
                    eprintln!("the desktop portal showed no file dialog: {why}");
                    None
                }
            },
            Way::Commands => first_command(os, ask, owner),
            #[allow(unreachable_patterns)]
            _ => None,
        };
        if let Some(path) = answered {
            return path;
        }
    }
    eprintln!("no file dialog could be shown on this system");
    None
}

/// The first program that shows the dialog: Some with its path, or Some(None) when the person chose none; None when none could show.
fn first_command(os: Os, ask: &Ask, owner: Option<SystemWindow>) -> Option<Option<String>> {
    for program in commands_for(os, ask, owner) {
        let Ok(output) = Command::new(&program.program).args(&program.args).stdin(Stdio::null()).stderr(Stdio::null()).output() else {
            // The program is not installed: the next one is asked.
            continue;
        };
        // 0 is a choice, 1 is Cancel; anything else is a program that could not show its dialog.
        match output.status.code() {
            Some(0) => return Some(answer(&String::from_utf8_lossy(&output.stdout))),
            Some(1) => return Some(None),
            _ => {}
        }
    }
    None
}

/// A program's answer as a path: its first line, a folder without the slash it ends with; none for nothing.
pub fn answer(output: &str) -> Option<String> {
    let mut line = output.split('\n').next().unwrap_or("").trim_end_matches('\r').to_string();
    if line.len() > 1 && line.ends_with('/') {
        line.pop();
    }
    (!line.is_empty()).then_some(line)
}

/// The programs that show the dialog on `os`, in the order they are tried: zenity and kdialog on Linux, osascript on macOS; none on Windows.
pub fn commands(os: Os, ask: &Ask) -> Vec<Program> {
    commands_for(os, ask, None)
}

/// The programs that show the dialog on `os` over the window `owner`, which takes no input meanwhile.
pub fn commands_for(os: Os, ask: &Ask, owner: Option<SystemWindow>) -> Vec<Program> {
    let x11 = owner.map(|o| o.x11).filter(|x| *x != 0);
    match os {
        Os::Windows => Vec::new(),
        Os::MacOs => vec![Program::new("osascript", vec!["-e".into(), apple_script(ask)])],
        Os::Linux => vec![zenity(ask, x11), kdialog(ask, x11)],
    }
}

fn zenity(ask: &Ask, owner: Option<u64>) -> Program {
    let mut args = vec!["--file-selection".to_string(), format!("--title={}", ask.title)];
    // Over the window it belongs to, which takes no input meanwhile.
    if let Some(owner) = owner {
        args.extend(["--modal".to_string(), format!("--attach={owner}")]);
    }
    match ask.kind {
        Kind::Save => args.extend(["--save".to_string(), "--confirm-overwrite".to_string(), format!("--filename={}", ask.name.clone().unwrap_or_default())]),
        Kind::Folder => args.push("--directory".to_string()),
        Kind::Open => {
            if let Some(filter) = &ask.filter {
                args.push(format!("--file-filter={} | {}", filter.name, filter.patterns.join(" ")));
            }
        }
    }
    Program::new("zenity", args)
}

fn kdialog(ask: &Ask, owner: Option<u64>) -> Program {
    let home = std::env::var("HOME").unwrap_or_default();
    let mut args = vec!["--title".to_string(), ask.title.clone()];
    if let Some(owner) = owner {
        args.extend(["--attach".to_string(), owner.to_string()]);
    }
    match ask.kind {
        Kind::Save => args.extend(["--getsavefilename".to_string(), std::path::Path::new(&home).join(ask.name.clone().unwrap_or_default()).to_string_lossy().into_owned()]),
        Kind::Folder => args.extend(["--getexistingdirectory".to_string(), home]),
        Kind::Open => {
            args.extend(["--getopenfilename".to_string(), home]);
            if let Some(filter) = &ask.filter {
                args.push(format!("{} ({})", filter.name, filter.patterns.join(" ")));
            }
        }
    }
    Program::new("kdialog", args)
}

/// The AppleScript of the dialog, its texts quoted as AppleScript strings.
pub fn apple_script(ask: &Ask) -> String {
    let prompt = format!("with prompt {}", quoted(&ask.title));
    match ask.kind {
        Kind::Save => format!("POSIX path of (choose file name {prompt} default name {})", quoted(ask.name.as_deref().unwrap_or(""))),
        Kind::Folder => format!("POSIX path of (choose folder {prompt})"),
        Kind::Open => {
            let kinds = extensions(ask.filter.as_ref());
            if kinds.is_empty() {
                format!("POSIX path of (choose file {prompt})")
            } else {
                format!("POSIX path of (choose file {prompt} of type {{{}}})", kinds.iter().map(|k| quoted(k)).collect::<Vec<_>>().join(", "))
            }
        }
    }
}

fn quoted(text: &str) -> String {
    format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
}

/// The extensions of a filter's patterns: `*.png` is png; a pattern of every file gives none.
pub fn extensions(filter: Option<&FileFilter>) -> Vec<String> {
    let Some(filter) = filter else { return Vec::new() };
    if filter.patterns.iter().any(|p| p == "*" || p == "*.*") {
        return Vec::new();
    }
    filter.patterns.iter().filter_map(|p| p.strip_prefix("*.")).filter(|e| !e.is_empty()).map(str::to_string).collect()
}
