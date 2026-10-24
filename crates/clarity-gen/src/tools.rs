//! The build tools of each stack (P3). Clarity is installed as a finished
//! program, so a machine may lack a stack's tools: cargo, the .NET SDK, Go,
//! a C or C++ compiler. Clarity looks for them and says what to install;
//! designing, previewing and generating work without them, and only Build
//! and Run need them, so the generated source can be built elsewhere.

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use clarity_project::{Backend, Ui};

pub mod environment;

/// How Clarity looks at the machine: where a program is, and what it says.
pub trait Probe {
    fn find(&self, program: &str) -> Option<PathBuf>;
    /// What `program args` prints, when it runs and succeeds.
    fn run(&self, program: &str, args: &[&str]) -> Option<String>;
    /// What `program args` says, on its output or its error output, however
    /// it ends: Visual Studio's `cl` names itself only that way.
    fn says(&self, program: &str, args: &[&str]) -> Option<String> {
        self.run(program, args)
    }
}

/// The machine Clarity runs on.
pub struct System;

impl Probe for System {
    fn find(&self, program: &str) -> Option<PathBuf> {
        environment::find(program, &environment::Environment::current(), &tool_path())
    }

    fn run(&self, program: &str, args: &[&str]) -> Option<String> {
        let path = self.find(program)?;
        let out = Command::new(path).args(args).stdin(Stdio::null()).stderr(Stdio::null()).output().ok()?;
        out.status.success().then(|| String::from_utf8_lossy(&out.stdout).into_owned())
    }

    fn says(&self, program: &str, args: &[&str]) -> Option<String> {
        let path = self.find(program)?;
        let out = Command::new(path).args(args).stdin(Stdio::null()).output().ok()?;
        let text = if out.stdout.iter().all(u8::is_ascii_whitespace) { out.stderr } else { out.stdout };
        let text = String::from_utf8_lossy(&text).into_owned();
        (!text.trim().is_empty()).then_some(text)
    }
}

/// Where Clarity looks for build tools on this machine: its PATH, then the
/// folders the official installers use. Build commands run with it as
/// their PATH, so the tools they start themselves are found too.
pub fn tool_path() -> OsString {
    environment::search_path(&environment::Environment::current(), &installers_path())
}

/// The PATH and the installers' folders, without the person's choices: where a scan looks.
pub fn installers_path() -> OsString {
    let home = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).map(PathBuf::from);
    search_path(std::env::var_os("PATH").as_deref(), home.as_deref())
}

/// The variables a build gets from the person's choices: the chosen C and C++ compilers for CMake.
pub fn tool_env() -> Vec<(String, String)> {
    environment::compilers(&environment::Environment::current())
}

/// `path` (a PATH value), followed by the installers' folders it lacks. A
/// program started from the desktop menu gets a shorter PATH than a
/// terminal, without what a shell profile adds: rustup's ~/.cargo/bin,
/// Go's /usr/local/go/bin, the .NET script's ~/.dotnet.
pub fn search_path(path: Option<&OsStr>, home: Option<&Path>) -> OsString {
    let mut folders: Vec<PathBuf> = path.map(|p| std::env::split_paths(p).collect()).unwrap_or_default();
    let mut extra = Vec::new();
    for (variable, sub) in [("CARGO_HOME", "bin"), ("GOROOT", "bin"), ("GOBIN", ""), ("DOTNET_ROOT", "")] {
        if let Some(dir) = std::env::var_os(variable) {
            extra.push(PathBuf::from(dir).join(sub));
        }
    }
    if let Some(home) = home {
        extra.extend([home.join(".cargo").join("bin"), home.join(".dotnet"), home.join("go").join("bin")]);
        if !cfg!(windows) {
            extra.push(home.join(".local/bin"));
        }
    }
    if cfg!(windows) {
        extra.extend(windows_folders(&|name| std::env::var_os(name), home));
        // Visual Studio installed somewhere else, as its own locator knows.
        for installation in vswhere_installations() {
            extra.extend(visual_studio_folders(&installation));
        }
    } else {
        extra.extend(
            ["/usr/local/go/bin", "/usr/local/bin", "/usr/local/share/dotnet", "/usr/share/dotnet", "/opt/homebrew/bin", "/snap/bin"]
                .map(PathBuf::from),
        );
    }
    for dir in extra {
        let dir = PathBuf::from(dir.to_string_lossy().trim_end_matches(['/', '\\']));
        if !dir.as_os_str().is_empty() && !folders.contains(&dir) {
            folders.push(dir);
        }
    }
    std::env::join_paths(folders).unwrap_or_default()
}

/// The folders the usual Windows installers put build tools in, by the
/// environment variables `var` gives: Go, .NET, CMake and LLVM in Program
/// Files; Visual Studio's compiler, CMake and Ninja; MSYS2's MinGW and
/// Clang; Scoop's, Chocolatey's and WinGet's links.
pub fn windows_folders(var: &dyn Fn(&str) -> Option<OsString>, home: Option<&Path>) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let program_files: Vec<PathBuf> = ["ProgramFiles", "ProgramFiles(x86)"].iter().filter_map(|v| var(v)).map(PathBuf::from).collect();
    for dir in &program_files {
        out.extend([dir.join("Go").join("bin"), dir.join("dotnet"), dir.join("CMake").join("bin"), dir.join("LLVM").join("bin")]);
    }
    for installation in visual_studio_installations(&program_files) {
        out.extend(visual_studio_folders(&installation));
    }
    let drive = var("SystemDrive").map(|d| d.to_string_lossy().into_owned()).unwrap_or_else(|| "C:".into());
    let msys = PathBuf::from(format!("{drive}{}", std::path::MAIN_SEPARATOR)).join("msys64");
    for shell in ["ucrt64", "mingw64", "clang64", "usr"] {
        out.push(msys.join(shell).join("bin"));
    }
    if let Some(home) = home {
        out.push(home.join("scoop").join("shims"));
    }
    let chocolatey = var("ChocolateyInstall").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(format!("{drive}{}", std::path::MAIN_SEPARATOR)).join("ProgramData").join("chocolatey"));
    out.push(chocolatey.join("bin"));
    if let Some(local) = var("LOCALAPPDATA") {
        out.push(PathBuf::from(local).join("Microsoft").join("WinGet").join("Links"));
    }
    out
}

/// The Visual Studio installations with a C and C++ compiler under the
/// Program Files folders `roots`: `Microsoft Visual Studio/<year>/<edition>`, newest year first.
pub fn visual_studio_installations(roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for root in roots {
        let Ok(years) = std::fs::read_dir(root.join("Microsoft Visual Studio")) else { continue };
        let mut years: Vec<PathBuf> = years.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
        years.sort();
        years.reverse();
        for year in years {
            let Ok(editions) = std::fs::read_dir(&year) else { continue };
            let mut editions: Vec<PathBuf> = editions.flatten().map(|e| e.path()).filter(|p| p.join("VC").join("Tools").join("MSVC").is_dir()).collect();
            editions.sort();
            found.extend(editions);
        }
    }
    found
}

/// The folders of a Visual Studio installation with its build tools: the
/// 64-bit compiler (`cl`) of its default tool version, else the newest, and
/// the CMake and Ninja it brings.
pub fn visual_studio_folders(installation: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let msvc = installation.join("VC").join("Tools").join("MSVC");
    let default = std::fs::read_to_string(installation.join("VC").join("Auxiliary").join("Build").join("Microsoft.VCToolsVersion.default.txt"))
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| msvc.join(v).is_dir());
    let newest = || {
        let mut versions: Vec<String> = std::fs::read_dir(&msvc).ok()?.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
        versions.sort_by_key(|v| v.split('.').map(|n| n.parse::<u32>().unwrap_or(0)).collect::<Vec<_>>());
        versions.pop()
    };
    if let Some(version) = default.or_else(newest) {
        out.push(msvc.join(version).join("bin").join("Hostx64").join("x64"));
    }
    let bundled = installation.join("Common7").join("IDE").join("CommonExtensions").join("Microsoft").join("CMake");
    out.extend([bundled.join("CMake").join("bin"), bundled.join("Ninja")]);
    out
}

/// The Visual Studio installations Microsoft's own locator (`vswhere`) knows, on Windows.
fn vswhere_installations() -> Vec<PathBuf> {
    let Some(x86) = std::env::var_os("ProgramFiles(x86)") else { return Vec::new() };
    let vswhere = PathBuf::from(x86).join("Microsoft Visual Studio").join("Installer").join("vswhere.exe");
    let Ok(out) = Command::new(vswhere).args(["-all", "-products", "*", "-property", "installationPath"]).stdin(Stdio::null()).stderr(Stdio::null()).output() else {
        return Vec::new();
    };
    String::from_utf8_lossy(&out.stdout).lines().map(str::trim).filter(|l| !l.is_empty()).map(PathBuf::from).collect()
}

/// The file of `program` in the folders of `path` (a PATH value), with
/// Windows' `.exe`.
pub fn find_on_path(program: &str, path: &OsStr) -> Option<PathBuf> {
    let name = if cfg!(windows) { format!("{program}.exe") } else { program.to_string() };
    std::env::split_paths(path).map(|dir| dir.join(&name)).find(|file| is_program(file))
}

pub(crate) fn is_program(file: &std::path::Path) -> bool {
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
    /// For each of `missing`, in the same order, the heading of the manual's
    /// page `SETUP_TOPIC` that tells how to install it on this system.
    pub guides: Vec<&'static str>,
}

impl StackTools {
    /// Whether Build and Run can work.
    pub fn ready(&self) -> bool {
        self.generated && self.missing.is_empty()
    }
}

/// Whether Clarity generates `backend` with `ui`.
pub fn generates(backend: Backend, ui: Ui) -> bool {
    matches!(
        (backend, ui),
        (Backend::Rust, Ui::Web) | (Backend::Go, Ui::Web) | (Backend::Csharp, Ui::Avalonia) | (Backend::C, Ui::Web) | (Backend::Cpp, Ui::Web)
    )
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

/// The first of `compilers` that answers `--version`, or Visual Studio's
/// `cl`, which names itself when run alone.
fn compiler(probe: &dyn Probe, compilers: &[&str]) -> Option<String> {
    compilers.iter().find_map(|c| probe.run(c, &["--version"]).map(|v| first_line(&v))).or_else(|| {
        probe.says("cl", &[]).map(|v| first_line(&v)).filter(|v| v.contains("Microsoft"))
    })
}

/// The manual's page on installing the build tools (docs/manual/setup.tools.md).
pub const SETUP_TOPIC: &str = "setup.tools";

/// Where that page tells how to install something: a heading's anchor per
/// system, as the manual's window makes them.
#[derive(Clone, Copy)]
struct Guide {
    windows: &'static str,
    linux: &'static str,
    macos: &'static str,
}

impl Guide {
    /// The heading for the system Clarity runs on.
    fn here(self) -> &'static str {
        if cfg!(windows) {
            self.windows
        } else if cfg!(target_os = "macos") {
            self.macos
        } else {
            self.linux
        }
    }
}

const RUST_GUIDE: Guide = Guide { windows: "rust-on-windows", linux: "rust-on-linux", macos: "rust-on-macos" };
const DOTNET_GUIDE: Guide = Guide { windows: "net-on-windows", linux: "net-on-linux", macos: "net-on-macos" };
const GO_GUIDE: Guide = Guide { windows: "go-on-windows", linux: "go-on-linux", macos: "go-on-macos" };
/// The compilers, CMake, make or Ninja, and the C and C++ libraries.
const NATIVE_GUIDE: Guide = Guide { windows: "c-and-c-on-windows", linux: "c-and-c-on-linux", macos: "c-and-c-on-macos" };
/// WebKitGTK is Linux's alone.
const WEBKIT_GUIDE: Guide = Guide { windows: "webview2-on-windows", linux: "webkitgtk-on-linux", macos: "webkitgtk-on-linux" };

/// Every heading of the setup page the check can point at, on any system.
pub fn guide_headings() -> Vec<&'static str> {
    [RUST_GUIDE, DOTNET_GUIDE, GO_GUIDE, NATIVE_GUIDE, WEBKIT_GUIDE].iter().flat_map(|g| [g.windows, g.linux, g.macos]).collect()
}

/// What to install for each tool, with where to get it: the words of
/// `check`, `clarity tools`, the app and every generated README.md.
const RUST: &str = "Rust with cargo, from https://rustup.rs";
const WEBKIT: &str = "the WebKitGTK development files and pkg-config (Debian and Ubuntu: libwebkit2gtk-4.1-dev; see https://tauri.app/start/prerequisites/)";
const GO: &str = "Go, from https://go.dev/dl";
const CMAKE: &str = "CMake, from https://cmake.org/download (Windows, in MSYS2's UCRT64 shell: pacman -S mingw-w64-ucrt-x86_64-cmake)";
const MAKE: &str = "make or Ninja, to build what CMake writes (Ninja: https://ninja-build.org; Windows: pacman -S mingw-w64-ucrt-x86_64-ninja)";
const SQLITE: &str = "the SQLite development files (Debian and Ubuntu: libsqlite3-dev; Windows, in MSYS2's UCRT64 shell: pacman -S mingw-w64-ucrt-x86_64-sqlite3; elsewhere https://sqlite.org/download.html)";
const WEBKIT_GTK: &str = "the WebKitGTK and GTK 3 development files and pkg-config (Debian and Ubuntu: libwebkit2gtk-4.1-dev libgtk-3-dev)";
const NLOHMANN: &str = "nlohmann-json (Debian and Ubuntu: nlohmann-json3-dev; Windows, in MSYS2's UCRT64 shell: pacman -S mingw-w64-ucrt-x86_64-nlohmann-json; elsewhere https://github.com/nlohmann/json)";
/// The webview draws with WebView2 on Windows, whose SDK headers the shell
/// builds against; MSYS2 does not package them.
const WEBVIEW2: &str = "the WebView2 SDK, from https://developer.microsoft.com/microsoft-edge/webview2 (the Microsoft.Web.WebView2 NuGet package); the runtime itself ships with Windows 11";
/// HTTP calls from C and C++ code (DESIGN 5.23) go through libcurl.
pub const LIBCURL: &str = "the libcurl development files, as the code calls web APIs (Debian and Ubuntu: libcurl4-openssl-dev; Windows, in MSYS2's UCRT64 shell: pacman -S mingw-w64-ucrt-x86_64-curl; macOS brings it)";
/// The webview of a C program is one C++ file.
const WEBVIEW_CXX: &str = "a C++ compiler for the webview's one C++ file: GCC (g++) or Clang (clang++)";

fn dotnet() -> String {
    format!("the .NET {} SDK, from https://dotnet.microsoft.com/download", dotnet_major())
}

fn compiler_needed(backend: Backend) -> String {
    let language = if backend == Backend::C { "C" } else { "C++" };
    format!(
        "a {language} compiler: GCC or Clang (on Windows, MSYS2's UCRT64 shell: pacman -S mingw-w64-ucrt-x86_64-gcc, or Visual Studio's Build Tools; take every C and C++ tool from one of them, never a mix)"
    )
}

/// Everything `backend` builds with, on any system: what a README lists.
pub fn requirements(backend: Backend) -> Vec<String> {
    match backend {
        Backend::Rust => vec![RUST.to_string(), format!("on Linux, {WEBKIT}")],
        Backend::Csharp => vec![dotnet()],
        Backend::Go => vec![GO.to_string(), format!("on Linux, {WEBKIT}")],
        Backend::C => vec![
            compiler_needed(backend),
            WEBVIEW_CXX.to_string(),
            CMAKE.to_string(),
            MAKE.to_string(),
            SQLITE.to_string(),
            format!("on Linux, {WEBKIT_GTK}"),
            format!("on Windows, {WEBVIEW2}"),
        ],
        Backend::Cpp => vec![
            compiler_needed(backend),
            CMAKE.to_string(),
            MAKE.to_string(),
            SQLITE.to_string(),
            NLOHMANN.to_string(),
            format!("on Linux, {WEBKIT_GTK}"),
            format!("on Windows, {WEBVIEW2}"),
        ],
    }
}

/// Looks for the tools `backend` with `ui` builds with.
pub fn check(backend: Backend, ui: Ui, probe: &dyn Probe) -> StackTools {
    let mut found = Vec::new();
    // What to install, each with where the manual tells how.
    let mut missing: Vec<(String, Guide)> = Vec::new();
    match backend {
        Backend::Rust => {
            match probe.run("cargo", &["--version"]) {
                Some(version) => found.push(first_line(&version)),
                None => missing.push((RUST.to_string(), RUST_GUIDE)),
            }
            // Tauri draws with WebKitGTK on Linux, and builds against its development files.
            if cfg!(target_os = "linux") && probe.run("pkg-config", &["--exists", "webkit2gtk-4.1"]).is_none() {
                missing.push((WEBKIT.to_string(), WEBKIT_GUIDE));
            }
        }
        Backend::Csharp => {
            match probe.run("dotnet", &["--list-sdks"]).and_then(|listed| dotnet_sdk(&listed, dotnet_major())) {
                Some(version) => found.push(format!(".NET SDK {version}")),
                None => missing.push((dotnet(), DOTNET_GUIDE)),
            }
        }
        Backend::Go => {
            match probe.run("go", &["version"]) {
                Some(version) => found.push(first_line(&version)),
                None => missing.push((GO.to_string(), GO_GUIDE)),
            }
            // Wails draws with WebKitGTK on Linux too, through cgo.
            if cfg!(target_os = "linux") && probe.run("pkg-config", &["--exists", "webkit2gtk-4.1"]).is_none() {
                missing.push((WEBKIT.to_string(), WEBKIT_GUIDE));
            }
        }
        Backend::C | Backend::Cpp => {
            let compilers = if backend == Backend::C { ["cc", "gcc", "clang"] } else { ["c++", "g++", "clang++"] };
            match compiler(probe, &compilers) {
                Some(version) => found.push(version),
                None => missing.push((compiler_needed(backend), NATIVE_GUIDE)),
            }
            if backend == Backend::C {
                match compiler(probe, &["c++", "g++", "clang++"]) {
                    Some(version) => found.push(version),
                    None => missing.push((WEBVIEW_CXX.to_string(), NATIVE_GUIDE)),
                }
            }
            // The generated project is a CMake project (CMakeLists.txt), built with make or Ninja.
            match probe.run("cmake", &["--version"]) {
                Some(version) => found.push(first_line(&version)),
                None => missing.push((CMAKE.to_string(), NATIVE_GUIDE)),
            }
            if let Some(version) = probe.run("make", &["--version"]).or_else(|| probe.run("mingw32-make", &["--version"])) {
                found.push(first_line(&version));
            } else if let Some(version) = probe.run("ninja", &["--version"]) {
                found.push(format!("ninja {}", first_line(&version)));
            } else {
                missing.push((MAKE.to_string(), NATIVE_GUIDE));
            }
            // The libraries CMake finds, through pkg-config on Linux.
            if cfg!(target_os = "linux") {
                let mut libraries = vec![("sqlite3", SQLITE, NATIVE_GUIDE), ("webkit2gtk-4.1", WEBKIT_GTK, WEBKIT_GUIDE)];
                if backend == Backend::Cpp {
                    libraries.push(("nlohmann_json", NLOHMANN, NATIVE_GUIDE));
                }
                for (package, install, guide) in libraries {
                    if probe.run("pkg-config", &["--exists", package]).is_none() {
                        missing.push((install.to_string(), guide));
                    }
                }
            }
        }
    }
    StackTools {
        backend,
        ui,
        generated: generates(backend, ui),
        found,
        guides: missing.iter().map(|(_, guide)| guide.here()).collect(),
        missing: missing.into_iter().map(|(what, _)| what).collect(),
    }
}

const WASM_TARGET: &str = "Rust's WebAssembly target: run rustup target add wasm32-unknown-unknown";

/// What to install to build engines from their Rust source: cargo, and
/// Rust's WebAssembly target. Empty when nothing is missing.
pub fn engine_tools(probe: &dyn Probe) -> Vec<String> {
    if probe.run("cargo", &["--version"]).is_none() {
        return vec![RUST.to_string()];
    }
    let target = clarity_project::engines::TARGET;
    let installed = match probe.run("rustup", &["target", "list", "--installed"]) {
        Some(listed) => listed.lines().any(|l| l.trim() == target),
        // Rust without rustup, as a system package: the target is in its sysroot or not.
        None => probe
            .run("rustc", &["--print", "sysroot"])
            .is_some_and(|root| std::path::Path::new(root.trim()).join("lib/rustlib").join(target).is_dir()),
    };
    if installed { Vec::new() } else { vec![WASM_TARGET.to_string()] }
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
    if stacks.iter().any(|s| !s.missing.is_empty()) {
        out.push_str("How to install each: the manual's page Setting up the build tools (Manual in Clarity, or docs/manual/setup.tools.md).\n");
    }
    out
}
