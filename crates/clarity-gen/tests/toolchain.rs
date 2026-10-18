//! Each stack builds its generated project with its own tools: Run and Build
//! in Clarity run the command `toolchain` gives and start the program it
//! names.

use std::path::{Path, PathBuf};

use clarity_project::{Backend, Ui};

fn shop(backend: Backend, ui: Ui) -> clarity_project::Settings {
    let file = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/shop.clarity");
    let mut settings = clarity_project::load(&file).settings.expect("the example has a project file");
    settings.stack.backend = backend;
    settings.stack.ui = ui;
    settings
}

fn exe(path: &str) -> PathBuf {
    if cfg!(windows) { PathBuf::from(format!("{path}.exe")) } else { PathBuf::from(path) }
}

#[test]
fn rust_builds_with_cargo() {
    let settings = shop(Backend::Rust, Ui::Web);
    let debug = clarity_gen::toolchain(&settings, false).unwrap();
    assert_eq!(debug.tool, "cargo");
    assert_eq!(debug.args, ["build", "--color", "never"]);
    assert_eq!(debug.program, exe("target/debug/shop"));
    let release = clarity_gen::toolchain(&settings, true).unwrap();
    assert_eq!(release.args, ["build", "--color", "never", "--release"]);
    assert_eq!(release.program, exe("target/release/shop"));
}

#[test]
fn csharp_builds_with_dotnet() {
    let settings = shop(Backend::Csharp, Ui::Avalonia);
    let debug = clarity_gen::toolchain(&settings, false).unwrap();
    assert_eq!(debug.tool, "dotnet");
    assert_eq!(debug.args, ["build", "--nologo", "-tl:off", "-c", "Debug"]);
    // The program dotnet writes next to Shop.dll.
    assert_eq!(debug.program, exe("bin/Debug/net10.0/Shop"));
    let release = clarity_gen::toolchain(&settings, true).unwrap();
    assert_eq!(release.args, ["build", "--nologo", "-tl:off", "-c", "Release"]);
    assert_eq!(release.program, exe("bin/Release/net10.0/Shop"));
    assert!(debug.missing.contains(".NET SDK"), "{}", debug.missing);
}

#[test]
fn go_builds_with_go_build() {
    let settings = shop(Backend::Go, Ui::Web);
    let debug = clarity_gen::toolchain(&settings, false).unwrap();
    assert_eq!(debug.tool, "go");
    assert_eq!(debug.args, ["build", "-o", "bin/shop", "."]);
    assert_eq!(debug.program, exe("bin/shop"));
    let release = clarity_gen::toolchain(&settings, true).unwrap();
    // Wails' production build, without debug information.
    assert_eq!(release.args, ["build", "-tags", "production", "-trimpath", "-ldflags=-s -w", "-o", "bin/shop", "."]);
    assert!(debug.missing.contains("Go"), "{}", debug.missing);
}

#[test]
fn stacks_without_a_generator_have_no_toolchain() {
    assert!(clarity_gen::toolchain(&shop(Backend::C, Ui::Web), false).is_none());
}
