//! Every program Clarity starts (a tool asked its version, a build, the
//! program built) runs as a background process. On Windows a console
//! program started from a window program opens a console window of its own,
//! which flashes up and closes, unless it is started with no window
//! (CREATE_NO_WINDOW); its output still reaches Clarity through its pipes.
//! Guarded by tests/background.rs: nothing else calls `Command::new`.

use std::ffi::OsStr;
use std::process::Command;

/// Windows' CREATE_NO_WINDOW: the process gets no console window.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// A command for `program` that runs in the background: on Windows with no
/// console window of its own; elsewhere a plain command.
pub fn background_command(program: impl AsRef<OsStr>) -> Command {
    #[allow(unused_mut)]
    let mut command = Command::new(program);
    no_window(&mut command);
    command
}

/// Starts `command` with no console window on Windows.
#[allow(unused_variables)]
pub fn no_window(command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);
    }
}

/// The creation flags of a background process, for a command of another
/// kind (tokio's) that takes them itself: CREATE_NO_WINDOW on Windows.
#[cfg(windows)]
pub const BACKGROUND_FLAGS: u32 = CREATE_NO_WINDOW;
