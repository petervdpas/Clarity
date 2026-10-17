//! The build tools of each stack (P3). Clarity is installed as a finished
//! program, so a machine may lack a stack's tools: cargo, the .NET SDK, Go,
//! a C or C++ compiler. Clarity looks for them and says what to install;
//! designing, previewing and generating work without them, and only Build
//! and Run need them, so the generated source can be built elsewhere.

use std::ffi::OsStr;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use clarity_project::{Backend, Ui};

/// How Clarity looks at the machine: where a program is, and what it says.
pub trait Probe {
    fn find(&self, program: &str) -> Option<PathBuf>;
    /// What `program args` prints, when it runs and succeeds.
    fn run(&self, program: &str, args: &[&str]) -> Option<String>;
}

/// The machine Clarity runs on.
pub struct System;

impl Probe for System {
    fn find(&self, program: &str) -> Option<PathBuf> {
        find_on_path(program, &std::env::var_os("PATH")?)
    }

    fn run(&self, program: &str, args: &[&str]) -> Option<String> {
        let path = self.find(program)?;
        let out = Command::new(path).args(args).stdin(Stdio::null()).stderr(Stdio::null()).output().ok()?;
        out.status.success().then(|| String::from_utf8_lossy(&out.stdout).into_owned())
    }
}

/// The file of `program` in the folders of `path` (a PATH value), with
/// Windows' `.exe`.
pub fn find_on_path(program: &str, path: &OsStr) -> Option<PathBuf> {
    let name = if cfg!(windows) { format!("{program}.exe") } else { program.to_string() };
    std::env::split_paths(path).map(|dir| dir.join(&name)).find(|file| is_program(file))
}

fn is_program(file: &std::path::Path) -> bool {
    let Ok(meta) = std::fs::metadata(file) else { return false };
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        meta.is_file() && meta.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        meta.is_file()
    }
}

/// What a stack's tools are on this machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackTools {
    pub backend: Backend,
    pub ui: Ui,
    /// Whether Clarity generates this stack already.
    pub generated: bool,
    /// The tools found, as they name themselves: `cargo 1.91.0`.
    pub found: Vec<String>,
    /// What to install, with where to get it.
    pub missing: Vec<String>,
}

impl StackTools {
    /// Whether Build and Run can work.
    pub fn ready(&self) -> bool {
        self.generated && self.missing.is_empty()
    }
}

/// Whether Clarity generates `backend` with `ui`.
pub fn generates(backend: Backend, ui: Ui) -> bool {
    matches!((backend, ui), (Backend::Rust, Ui::Web) | (Backend::Csharp, Ui::Avalonia))
}

/// The first line a tool prints, as it names itself.
fn first_line(text: &str) -> String {
    text.lines().next().unwrap_or_default().trim().to_string()
}

/// The newest SDK of `major` in what `dotnet --list-sdks` prints.
fn dotnet_sdk(listed: &str, major: u32) -> Option<String> {
    listed
        .lines()
        .filter_map(|l| l.split_whitespace().next())
        .rfind(|v| v.split('.').next().and_then(|m| m.parse::<u32>().ok()) == Some(major))
        .map(str::to_string)
}

/// The .NET major version generated projects target: 10 for `net10.0`.
fn dotnet_major() -> u32 {
    crate::csharp::TARGET_FRAMEWORK.trim_start_matches("net").split('.').next().and_then(|m| m.parse().ok()).unwrap_or(10)
}

/// The first of `compilers` that answers `--version`.
fn compiler(probe: &dyn Probe, compilers: &[&str]) -> Option<String> {
    compilers.iter().find_map(|c| probe.run(c, &["--version"]).map(|v| first_line(&v)))
}

/// What to install for each tool, with where to get it: the words of
/// `check`, `clarity tools`, the app and every generated README.md.
const RUST: &str = "Rust with cargo, from https://rustup.rs";
const WEBKIT: &str = "the WebKitGTK development files and pkg-config (Debian and Ubuntu: libwebkit2gtk-4.1-dev; see https://tauri.app/start/prerequisites/)";
const GO: &str = "Go, from https://go.dev/dl";
const CMAKE: &str = "CMake, from https://cmake.org/download";
const MAKE: &str = "make or Ninja, to build what CMake writes (Ninja: https://ninja-build.org)";

fn dotnet() -> String {
    format!("the .NET {} SDK, from https://dotnet.microsoft.com/download", dotnet_major())
}

fn compiler_needed(backend: Backend) -> String {
    let language = if backend == Backend::C { "C" } else { "C++" };
    format!("a {language} compiler: GCC or Clang (on Windows, Visual Studio's Build Tools)")
}

/// Everything `backend` builds with, on any system: what a README lists.
pub fn requirements(backend: Backend) -> Vec<String> {
    match backend {
        Backend::Rust => vec![RUST.to_string(), format!("on Linux, {WEBKIT}")],
        Backend::Csharp => vec![dotnet()],
        Backend::Go => vec![GO.to_string()],
        Backend::C | Backend::Cpp => vec![compiler_needed(backend), CMAKE.to_string(), MAKE.to_string()],
    }
}

/// Looks for the tools `backend` with `ui` builds with.
pub fn check(backend: Backend, ui: Ui, probe: &dyn Probe) -> StackTools {
    let mut found = Vec::new();
    let mut missing = Vec::new();
    match backend {
        Backend::Rust => {
            match probe.run("cargo", &["--version"]) {
                Some(version) => found.push(first_line(&version)),
                None => missing.push(RUST.to_string()),
            }
            // Tauri draws with WebKitGTK on Linux, and builds against its development files.
            if cfg!(target_os = "linux") && probe.run("pkg-config", &["--exists", "webkit2gtk-4.1"]).is_none() {
                missing.push(WEBKIT.to_string());
            }
        }
        Backend::Csharp => {
            match probe.run("dotnet", &["--list-sdks"]).and_then(|listed| dotnet_sdk(&listed, dotnet_major())) {
                Some(version) => found.push(format!(".NET SDK {version}")),
                None => missing.push(dotnet()),
            }
        }
        Backend::Go => match probe.run("go", &["version"]) {
            Some(version) => found.push(first_line(&version)),
            None => missing.push(GO.to_string()),
        },
        Backend::C | Backend::Cpp => {
            let compilers = if backend == Backend::C { ["cc", "gcc", "clang"] } else { ["c++", "g++", "clang++"] };
            match compiler(probe, &compilers) {
                Some(version) => found.push(version),
                None => missing.push(compiler_needed(backend)),
            }
            // The generated project is a CMake project (CMakeLists.txt), built with make or Ninja.
            match probe.run("cmake", &["--version"]) {
                Some(version) => found.push(first_line(&version)),
                None => missing.push(CMAKE.to_string()),
            }
            if let Some(version) = probe.run("make", &["--version"]) {
                found.push(first_line(&version));
            } else if let Some(version) = probe.run("ninja", &["--version"]) {
                found.push(format!("ninja {}", first_line(&version)));
            } else {
                missing.push(MAKE.to_string());
            }
        }
    }
    StackTools {
        backend,
        ui,
        generated: generates(backend, ui),
        found,
        missing,
    }
}

/// Every stack the catalog offers, checked.
pub fn check_all(probe: &dyn Probe) -> Vec<StackTools> {
    clarity_project::catalog()
        .backends
        .iter()
        .flat_map(|b| b.uis.iter().map(move |ui| (b.id, *ui)))
        .map(|(backend, ui)| check(backend, ui, probe))
        .collect()
}

/// A stack as people read it: `C# with Avalonia`.
pub fn stack_name(backend: Backend, ui: Ui) -> String {
    let catalog = clarity_project::catalog();
    let language = catalog.backends.iter().find(|b| b.id == backend).map_or("?", |b| b.label);
    let renderer = match ui {
        Ui::Web => "the web renderer",
        Ui::Avalonia => "Avalonia",
    };
    format!("{language} with {renderer}")
}

/// A line per stack: ready with the tools found, or what is missing.
pub fn report(stacks: &[StackTools]) -> String {
    let mut out = String::new();
    for s in stacks {
        let mut said = Vec::new();
        if !s.generated {
            said.push("not generated yet".to_string());
        }
        if s.missing.is_empty() {
            if s.generated {
                said.push(format!("ready ({})", s.found.join(", ")));
            } else {
                said.push(format!("tools found ({})", s.found.join(", ")));
            }
        } else {
            said.push(format!("missing {}", s.missing.join("; and ")));
        }
        out.push_str(&format!("{}: {}\n", stack_name(s.backend, s.ui), said.join("; ")));
    }
    out
}
