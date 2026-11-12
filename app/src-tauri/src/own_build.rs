//! The IDE started by `cargo run` (./dev.sh) has its own build folder on the
//! system's library search path: cargo puts `target/debug` and
//! `target/debug/deps` there. Every program the IDE builds and runs would
//! inherit it, and on Linux that path comes before the library beside the
//! program, so a Run loaded an old debug build of Clarity.UI from
//! `target/debug` instead of the one the program was built with. At its
//! start, before any thread or child, the IDE takes its own build folder out
//! of the search path, so nothing it starts (programs, build tools, engines,
//! agents) ever sees it. An installed IDE has nothing to take out.

use std::ffi::{OsStr, OsString};
use std::path::Path;

/// The variables that hold the library search path cargo adds to, on `os`.
pub fn search_path_variables(os: &str) -> &'static [&'static str] {
    match os {
        "windows" => &["PATH"],
        "macos" => &["DYLD_FALLBACK_LIBRARY_PATH", "DYLD_LIBRARY_PATH"],
        _ => &["LD_LIBRARY_PATH"],
    }
}

/// `value`, a search path, without the folders in `folder` (the folder itself
/// and those below it); None when nothing is left.
pub fn without_folder(value: &OsStr, folder: &Path) -> Option<OsString> {
    let kept: Vec<_> = std::env::split_paths(value).filter(|p| !p.as_os_str().is_empty() && !p.starts_with(folder)).collect();
    if kept.is_empty() {
        return None;
    }
    std::env::join_paths(kept).ok()
}

/// The build folder (`target`) the running IDE was built in, when cargo
/// started it (cargo names itself in CARGO); None for an installed IDE.
fn own_build_folder() -> Option<std::path::PathBuf> {
    std::env::var_os("CARGO")?;
    let exe = std::env::current_exe().ok()?;
    // target/debug/clarity-preview: the build folder is two up.
    exe.parent()?.parent().map(Path::to_path_buf)
}

/// Takes the IDE's own build folder out of its library search path. Called
/// first in `run`, while the IDE has one thread and has started nothing.
pub fn leave_out_own_build() {
    let Some(folder) = own_build_folder() else { return };
    for name in search_path_variables(std::env::consts::OS) {
        let Some(value) = std::env::var_os(name) else { continue };
        let cleaned = without_folder(&value, &folder);
        if cleaned.as_deref() == Some(value.as_os_str()) {
            continue;
        }
        // SAFETY: called at the very start of `run`, before Tauri or anything
        // else starts a thread, so nothing reads the environment meanwhile.
        unsafe {
            match cleaned {
                Some(cleaned) => std::env::set_var(name, cleaned),
                None => std::env::remove_var(name),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn joined(parts: &[&str]) -> OsString {
        std::env::join_paths(parts.iter().map(PathBuf::from)).unwrap()
    }

    #[test]
    fn the_own_build_folder_and_what_is_below_it_go_and_the_rest_stays() {
        let value = joined(&["/repo/target/debug/deps", "/repo/target/debug", "/home/me/.rustup/toolchains/stable/lib", "/opt/lib"]);
        let cleaned = without_folder(&value, Path::new("/repo/target"));
        assert_eq!(cleaned, Some(joined(&["/home/me/.rustup/toolchains/stable/lib", "/opt/lib"])));
    }

    #[test]
    fn a_folder_that_only_starts_with_the_same_letters_stays() {
        let value = joined(&["/repo/target-other/lib", "/repo/target/debug"]);
        assert_eq!(without_folder(&value, Path::new("/repo/target")), Some(joined(&["/repo/target-other/lib"])));
    }

    #[test]
    fn a_path_of_only_the_own_build_folder_is_left_out_whole() {
        let value = joined(&["/repo/target/debug/deps", "/repo/target/debug"]);
        assert_eq!(without_folder(&value, Path::new("/repo/target")), None);
    }

    #[test]
    fn a_path_without_it_is_as_it_was() {
        let value = joined(&["/usr/local/lib", "/opt/lib"]);
        assert_eq!(without_folder(&value, Path::new("/repo/target")), Some(value));
    }

    #[test]
    fn each_system_names_its_own_search_path() {
        assert_eq!(search_path_variables("linux"), &["LD_LIBRARY_PATH"]);
        assert_eq!(search_path_variables("macos"), &["DYLD_FALLBACK_LIBRARY_PATH", "DYLD_LIBRARY_PATH"]);
        assert_eq!(search_path_variables("windows"), &["PATH"]);
    }
}
