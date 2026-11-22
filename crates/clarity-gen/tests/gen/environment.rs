//! Clarity's environment: where the build tools of the five stacks are on
//! this computer, found by a scan or chosen by the person, kept in
//! `~/.clarity/environment.json` for every project, and used first by every
//! build Clarity runs.

use std::path::{Path, PathBuf};

use clarity_gen::tools::environment::{self, Environment, TOOLS};

/// A folder with fake programs, as a scan finds them.
fn folder(name: &str, programs: &[&str]) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("environment-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    for program in programs {
        let file = dir.join(if cfg!(windows) { format!("{program}.exe") } else { program.to_string() });
        std::fs::write(&file, "#!/bin/sh\necho fake 1.0\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
    }
    dir
}

fn path_of(folders: &[&Path]) -> std::ffi::OsString {
    std::env::join_paths(folders).unwrap()
}

#[test]
fn every_stack_has_its_tools_listed() {
    let programs: Vec<&str> = TOOLS.iter().map(|t| t.program).collect();
    for program in ["cargo", "go", "dotnet", "cmake", "cc", "c++", "make", "ninja"] {
        assert!(programs.contains(&program), "{program}");
    }
    for backend in [clarity_project::Backend::Rust, clarity_project::Backend::Go, clarity_project::Backend::Csharp, clarity_project::Backend::C, clarity_project::Backend::Cpp] {
        assert!(TOOLS.iter().any(|t| t.stacks.contains(&backend)), "{backend:?}");
    }
}

#[test]
fn a_scan_finds_every_copy_of_a_tool() {
    let first = folder("scan-first", &["go"]);
    let second = folder("scan-second", &["go", "cargo"]);
    let path = path_of(&[&first, &second]);
    let found = environment::candidates("go", &path);
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(found[0].starts_with(&first) && found[1].starts_with(&second));
}

#[test]
fn a_chosen_tool_comes_before_the_path() {
    let first = folder("chosen-first", &["go"]);
    let second = folder("chosen-second", &["go"]);
    let path = path_of(&[&first, &second]);
    let mut env = Environment::default();
    assert!(environment::find("go", &env, &path).unwrap().starts_with(&first));
    let chosen = environment::candidates("go", &path)[1].clone();
    env.choose("go", Some(chosen.clone()));
    assert_eq!(environment::find("go", &env, &path), Some(chosen.clone()));
    // Builds see the chosen tool's folder first, so the tools it starts are its own.
    let searched: Vec<PathBuf> = std::env::split_paths(&environment::search_path(&env, &path)).collect();
    assert_eq!(searched[0], second);
    // A chosen tool that is gone falls back to the path.
    env.choose("go", Some(second.join("gone")));
    assert!(environment::find("go", &env, &path).unwrap().starts_with(&first));
}

#[test]
fn chosen_compilers_reach_cmake() {
    let mut env = Environment::default();
    assert!(environment::compilers(&env).is_empty());
    env.choose("cc", Some(PathBuf::from("/opt/llvm/bin/clang")));
    env.choose("c++", Some(PathBuf::from("/opt/llvm/bin/clang++")));
    assert_eq!(
        environment::compilers(&env),
        [("CC".to_string(), "/opt/llvm/bin/clang".to_string()), ("CXX".to_string(), "/opt/llvm/bin/clang++".to_string())]
    );
}

#[test]
fn the_environment_is_kept_in_a_file_for_every_project() {
    let dir = folder("file", &[]);
    let file = dir.join("environment.json");
    assert_eq!(Environment::load(&file), Environment::default(), "no file is no choice");
    let mut env = Environment::default();
    env.choose("dotnet", Some(PathBuf::from("/usr/share/dotnet/dotnet")));
    env.save(&file).unwrap();
    assert_eq!(Environment::load(&file), env);
    env.choose("dotnet", None);
    assert_eq!(env.chosen("dotnet"), None);
    // The file sits in Clarity's own folder, beside the vault.
    assert!(environment::file().ends_with("environment.json"));
}
