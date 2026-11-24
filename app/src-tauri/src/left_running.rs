//! The program Run started, remembered in Clarity's own folder while it
//! runs, so that one left running when Clarity ended without stopping it (a
//! crash, a restart while developing) is ended when Clarity starts again: a
//! web program would otherwise keep its port. Only that same program is
//! ended: its process must still be the program that was started.

use std::path::{Path, PathBuf};

/// The file in Clarity's folder that names the running program: its process id, then its path.
const FILE: &str = "running.txt";

/// Remembers the program Run started.
pub fn remember(home: &Path, pid: u32, program: &Path) {
    let _ = std::fs::create_dir_all(home);
    let _ = std::fs::write(home.join(FILE), format!("{pid}\n{}\n", program.display()));
}

/// The program Run started has ended: nothing is remembered.
pub fn forget(home: &Path) {
    let _ = std::fs::remove_file(home.join(FILE));
}

/// The process to end from the remembered `text`: its id, when that process
/// still runs the remembered program (`program_of` gives what a process runs).
pub fn left(text: &str, program_of: &dyn Fn(u32) -> Option<PathBuf>) -> Option<u32> {
    let mut lines = text.lines();
    let pid: u32 = lines.next()?.trim().parse().ok()?;
    let program = PathBuf::from(lines.next()?.trim());
    let same = |a: &Path, b: &Path| a.canonicalize().ok().zip(b.canonicalize().ok()).is_some_and(|(a, b)| a == b);
    program_of(pid).filter(|running| same(running, &program)).map(|_| pid)
}

/// Ends the program a Clarity before this one left running, if it still
/// runs; forgets it either way. Gives the process id it ended.
pub fn end_left(home: &Path) -> Option<u32> {
    let text = std::fs::read_to_string(home.join(FILE)).ok()?;
    forget(home);
    let mut system = sysinfo::System::new();
    system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    let program_of = |pid: u32| system.process(sysinfo::Pid::from_u32(pid)).and_then(|p| p.exe()).map(Path::to_path_buf);
    let pid = left(&text, &program_of)?;
    system.process(sysinfo::Pid::from_u32(pid)).filter(|p| p.kill()).map(|_| pid)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Only the remembered program is ended: a process id now held by
    /// another program, a process that is gone, or a file that says nothing are left alone.
    #[test]
    fn only_the_same_program_still_running_is_left_over() {
        let me = std::env::current_exe().unwrap();
        let text = format!("4242\n{}\n", me.display());
        assert_eq!(left(&text, &|pid| (pid == 4242).then(|| me.clone())), Some(4242));
        assert_eq!(left(&text, &|_| Some(PathBuf::from("/bin/sh"))), None);
        assert_eq!(left(&text, &|_| None), None);
        assert_eq!(left("nothing", &|_| Some(me.clone())), None);
    }

    /// A program a Clarity before left running is ended when Clarity starts.
    #[cfg(unix)]
    #[test]
    fn a_program_left_running_is_ended() {
        let home = std::env::temp_dir().join(format!("clarity-left-running-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        let sleep = which("sleep");
        let mut child = std::process::Command::new(&sleep).arg("60").spawn().unwrap();
        remember(&home, child.id(), &sleep);
        assert_eq!(end_left(&home), Some(child.id()));
        assert!(child.wait().is_ok_and(|status| !status.success()), "the program was ended");
        assert!(!home.join(FILE).exists());
        // Nothing remembered: nothing ended.
        assert_eq!(end_left(&home), None);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[cfg(unix)]
    fn which(program: &str) -> PathBuf {
        std::env::var_os("PATH").and_then(|path| std::env::split_paths(&path).map(|dir| dir.join(program)).find(|file| file.is_file())).unwrap()
    }
}
