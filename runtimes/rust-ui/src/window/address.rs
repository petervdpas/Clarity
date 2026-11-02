//! Opening an address in the system's own program for it (a link's web page
//! in the browser, a `mailto:` in the mail program), as the desktop host
//! does: `xdg-open` on Linux, `open` on macOS, the shell's handler of
//! addresses on Windows. Only addresses a link opens are passed on: web pages, mail and
//! phone numbers. The host gives the opener (`set_opener`); a host that
//! gives none opens nothing and says so, so a test never starts a program.

use std::cell::RefCell;
use std::rc::Rc;

/// What opens an address: the host's, or the system's (`open`).
pub type Opener = Rc<dyn Fn(&str)>;

thread_local! {
    static OPENER: RefCell<Option<Opener>> = const { RefCell::new(None) };
}

/// The host gives what opens addresses for the parts on this thread; the desktop host gives `open`.
pub fn set_opener(opener: Option<Opener>) {
    OPENER.with(|o| *o.borrow_mut() = opener);
}

/// A part asked to open `address`: the host's opener does, else it is only said.
pub fn open_with_host(address: &str) {
    match OPENER.with(|o| o.borrow().clone()) {
        Some(opener) => opener(address),
        None => eprintln!("{address} is not opened: the host opens no addresses"),
    }
}

/// The system a command is made for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    Linux,
    MacOs,
    Windows,
}

impl Os {
    /// The system this runs on.
    pub fn current() -> Os {
        if cfg!(target_os = "windows") {
            Os::Windows
        } else if cfg!(target_os = "macos") {
            Os::MacOs
        } else {
            Os::Linux
        }
    }
}

/// What runs to open an address: a program and its arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    pub program: String,
    pub arguments: Vec<String>,
}

/// The schemes an address may have to be opened.
const SCHEMES: [&str; 4] = ["http:", "https:", "mailto:", "tel:"];

/// Whether `address` is one the system is asked to open: a web page, mail or a phone number, with no control characters.
pub fn opens(address: &str) -> bool {
    let lower = address.to_ascii_lowercase();
    SCHEMES.iter().any(|s| lower.starts_with(s)) && !address.chars().any(char::is_control)
}

/// What opens `address` on `os`; none for an address it is not asked to open.
pub fn command_for(os: Os, address: &str) -> Option<Command> {
    if !opens(address) {
        return None;
    }
    let (program, arguments) = match os {
        // The shell's own handler of addresses, as `start` uses, without cmd reading `&` in the address.
        Os::Windows => ("rundll32", vec!["url.dll,FileProtocolHandler".to_string(), address.to_string()]),
        Os::MacOs => ("open", vec![address.to_string()]),
        Os::Linux => ("xdg-open", vec![address.to_string()]),
    };
    Some(Command { program: program.to_string(), arguments })
}

/// Opens `address` in the system's program for it; one it may not open, or a program that fails, is only said.
pub fn open(address: &str) {
    let Some(command) = command_for(Os::current(), address) else {
        eprintln!("an address that is opened has no scheme of a web page, mail or phone: {address}");
        return;
    };
    if let Err(why) = std::process::Command::new(&command.program).args(&command.arguments).spawn() {
        eprintln!("{address} could not be opened: {why}");
    }
}
