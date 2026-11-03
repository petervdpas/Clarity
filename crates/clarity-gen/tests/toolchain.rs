//! Each stack builds its generated project with its own tools: Run and Build
//! in Clarity run the command `toolchain` gives and start the program it
//! names.

use std::path::PathBuf;

use clarity_project::{Backend, Ui};

/// A project named shop on `backend`: a new project's settings, not the
/// shop example, which is generated for its own stack only.
fn shop(backend: Backend, ui: Ui) -> clarity_project::Settings {
    let mut settings = clarity_project::starter("Shop", "shop");
    settings.stack.backend = backend;
    settings.stack.ui = ui;
    settings
}

fn exe(path: &str) -> PathBuf {
    if cfg!(windows) { PathBuf::from(format!("{path}.exe")) } else { PathBuf::from(path) }
}

#[test]
fn rust_builds_with_cargo() {
    let settings = shop(Backend::Rust, Ui::Clarity);
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
    let settings = shop(Backend::Csharp, Ui::Clarity);
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
    let settings = shop(Backend::Go, Ui::Clarity);
    let debug = clarity_gen::toolchain(&settings, false).unwrap();
    assert_eq!(debug.tool, "go");
    // `go build -o` writes the name it is given, so the program is built
    // under the name Run starts: `.exe` on Windows.
    let built = exe("bin/shop").to_string_lossy().into_owned();
    // -v names each package as it is built, so a long first build shows progress.
    assert_eq!(debug.args, ["build", "-v", "-o", built.as_str(), "."]);
    assert_eq!(debug.program, exe("bin/shop"));
    let release = clarity_gen::toolchain(&settings, true).unwrap();
    // Without debug information.
    assert_eq!(release.args, ["build", "-v", "-trimpath", "-ldflags=-s -w", "-o", built.as_str(), "."]);
    assert!(debug.missing.contains("Go"), "{}", debug.missing);
}

/// A Go desktop program links Clarity.UI's library through cgo, from the
/// folder CLARITY_UI names; it finds the shared library beside it, else in
/// that folder (DESIGN 7.9).
#[test]
fn go_takes_clarity_ui_from_the_folder_clarity_ui_names() {
    use clarity_gen::go_cgo_env;
    assert!(go_cgo_env(None, "linux").is_empty());
    let folder = std::path::Path::new("/opt/clarity-ui");
    let linux = go_cgo_env(Some(folder), "linux");
    let value = |env: &[(&str, String)], name: &str| env.iter().find(|(n, _)| *n == name).map(|(_, v)| v.clone()).unwrap_or_default();
    assert_eq!(value(&linux, "CGO_ENABLED"), "1");
    assert_eq!(value(&linux, "CGO_CFLAGS"), format!("-I{}", folder.join("include").display()));
    let lib = folder.join("lib").display().to_string();
    assert_eq!(value(&linux, "CGO_LDFLAGS"), format!("-L{lib} -Wl,-rpath,$ORIGIN -Wl,-rpath,{lib}"));
    assert_eq!(value(&go_cgo_env(Some(folder), "macos"), "CGO_LDFLAGS"), format!("-L{lib} -Wl,-rpath,@executable_path -Wl,-rpath,{lib}"));
    assert_eq!(value(&go_cgo_env(Some(folder), "windows"), "CGO_LDFLAGS"), format!("-L{lib}"));
}

#[test]
fn c_and_cpp_build_with_a_cmake_workflow() {
    for backend in [Backend::C, Backend::Cpp] {
        let settings = shop(backend, Ui::Clarity);
        let debug = clarity_gen::toolchain(&settings, false).unwrap();
        assert_eq!(debug.tool, "cmake");
        assert_eq!(debug.args, ["--workflow", "--preset", "debug"]);
        assert_eq!(debug.program, exe("build/debug/shop"));
        let release = clarity_gen::toolchain(&settings, true).unwrap();
        assert_eq!(release.args, ["--workflow", "--preset", "release"]);
        assert_eq!(release.program, exe("build/release/shop"));
    }
}

#[test]
fn stacks_without_a_generator_have_no_toolchain() {
    assert!(clarity_gen::toolchain(&shop(Backend::Csharp, Ui::Webview), false).is_none());
}

/// The command-line program (DESIGN 5.20) builds in `out/cli` with the same
/// tools, into a program named as it is typed.
#[test]
fn the_command_line_program_builds_with_the_same_tools() {
    let rust = clarity_gen::cli_toolchain(&shop(Backend::Rust, Ui::Clarity), false).unwrap();
    assert_eq!((rust.tool, rust.program.clone()), ("cargo", exe("target/debug/shop")));
    let go = clarity_gen::cli_toolchain(&shop(Backend::Go, Ui::Clarity), true).unwrap();
    assert_eq!(go.tool, "go");
    assert_eq!(go.program, exe("bin/shop"));
    assert!(!go.args.contains(&"production".to_string()), "{:?}", go.args);
    let csharp = clarity_gen::cli_toolchain(&shop(Backend::Csharp, Ui::Clarity), false).unwrap();
    assert_eq!((csharp.tool, csharp.program.clone()), ("dotnet", exe("bin/Debug/net10.0/shop")));
    for backend in [Backend::C, Backend::Cpp] {
        let release = clarity_gen::cli_toolchain(&shop(backend, Ui::Clarity), true).unwrap();
        assert_eq!((release.tool, release.program.clone()), ("cmake", exe("build/release/shop")));
    }
}
