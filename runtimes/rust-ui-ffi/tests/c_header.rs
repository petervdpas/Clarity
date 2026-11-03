//! The hand-written header against the library: every function the library
//! exports is declared in it and every declared one is exported, and a C
//! program built with the system's C compiler against the header and the
//! library runs (tests/c/clarity_ui_test.c). Build the library before
//! these tests: `cargo build -p clarity-ui-ffi && cargo test -p clarity-ui-ffi`.

use std::path::{Path, PathBuf};
use std::process::Command;

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The names of the functions the library's sources export.
fn exported() -> Vec<String> {
    let mut names = Vec::new();
    for entry in std::fs::read_dir(crate_dir().join("src")).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "rs") {
            let source = std::fs::read_to_string(&path).unwrap();
            let mut lines = source.lines();
            while let Some(line) = lines.next() {
                if line.trim() == "#[unsafe(no_mangle)]" {
                    let signature = lines.next().unwrap();
                    let name = signature.split("fn ").nth(1).unwrap().split('(').next().unwrap();
                    names.push(name.to_string());
                }
            }
        }
    }
    names.sort();
    names
}

/// The names of the functions the header declares.
fn declared() -> Vec<String> {
    let header = std::fs::read_to_string(crate_dir().join("include/clarity_ui.h")).unwrap();
    let mut names: Vec<String> = header
        .lines()
        .filter_map(|l| l.strip_prefix("CLARITY_API "))
        .map(|l| l.split('(').next().unwrap().rsplit([' ', '*']).next().unwrap().to_string())
        .collect();
    names.sort();
    names
}

#[test]
fn the_header_declares_what_the_library_exports() {
    assert_eq!(declared(), exported());
}

/// The library as `cargo build -p clarity-ui-ffi` built it, in the profile's
/// folder above the test's `deps` folder. Cargo does not build a cdylib for
/// tests, so the build comes first.
fn library() -> Option<PathBuf> {
    let deps = std::env::current_exe().ok()?.parent()?.to_path_buf();
    let name = if cfg!(target_os = "macos") { "libclarity_ui.dylib" } else { "libclarity_ui.so" };
    [deps.parent()?.join(name), deps.join(name)].into_iter().find(|p| p.exists())
}

#[test]
fn a_c_program_builds_against_the_header_and_runs() {
    if cfg!(windows) {
        eprintln!("the C program is built on Linux and macOS, with cc");
        return;
    }
    let library = library().expect("the library is built first: cargo build -p clarity-ui-ffi");
    let folder = library.parent().unwrap();
    let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join("clarity_ui_test");
    let built = Command::new("cc")
        .args(["-std=c11", "-Wall", "-Wextra", "-Werror", "-pedantic"])
        .arg("-I")
        .arg(crate_dir().join("include"))
        .arg(crate_dir().join("tests/c/clarity_ui_test.c"))
        .arg("-L")
        .arg(folder)
        .arg("-lclarity_ui")
        .arg(format!("-Wl,-rpath,{}", folder.display()))
        .arg("-o")
        .arg(&out)
        .output()
        .expect("the system has a C compiler, cc");
    assert!(built.status.success(), "cc failed:\n{}", String::from_utf8_lossy(&built.stderr));
    let ran = Command::new(&out).output().unwrap();
    let said = String::from_utf8_lossy(&ran.stdout);
    assert!(ran.status.success() && said.trim() == "ok", "the C program failed:\n{said}\n{}", String::from_utf8_lossy(&ran.stderr));
    let _ = std::fs::remove_file(out);
}
