//! The build tools of each stack (P3). Clarity is installed as a finished
//! program, so a machine may lack cargo or the .NET SDK; Clarity looks for
//! them and says what to install, while designing and generating keep
//! working. The probe is faked here, so the tests need no tools.

use std::collections::HashMap;
use std::path::PathBuf;

use clarity_gen::tools::{Probe, check, engine_tools, find_on_path, search_path};
use clarity_gen::clarity_ui::Library;
use clarity_project::{Backend, Ui};

/// A machine with these programs, each answering one command line.
#[derive(Default)]
struct Machine {
    programs: HashMap<&'static str, HashMap<String, &'static str>>,
    /// Where Clarity.UI's library comes from; by default fetched on the first build.
    library: Option<Library>,
}

impl Machine {
    fn library(mut self, library: Library) -> Self {
        self.library = Some(library);
        self
    }

    fn with(mut self, program: &'static str, args: &str, answer: &'static str) -> Self {
        self.programs.entry(program).or_default().insert(args.to_string(), answer);
        self
    }
}

impl Probe for Machine {
    fn find(&self, program: &str) -> Option<PathBuf> {
        self.programs.contains_key(program).then(|| PathBuf::from(format!("/usr/bin/{program}")))
    }

    fn run(&self, program: &str, args: &[&str]) -> Option<String> {
        self.programs.get(program)?.get(&args.join(" ")).map(|s| s.to_string())
    }

    fn clarity_ui(&self) -> Library {
        self.library.clone().unwrap_or(Library::ToFetch { folder: PathBuf::from("/cache/clarity/clarity-ui/0.1.0/linux-x64"), version: "0.1.0".into(), rid: "linux-x64".into() })
    }
}

/// What the tools say of Clarity.UI's library when it is fetched on the first build.
const FETCHED_ON_FIRST_BUILD: &str = "Clarity.UI's library 0.1.0, fetched on first build";

/// Rust draws with Clarity.UI only, on Skia, whose prebuilt library cargo
/// downloads: it needs cargo and no WebKitGTK, Tauri CLI or Node; its web
/// program serves Clarity.UI's browser files and needs nothing more.
#[test]
fn rust_on_clarity_ui_needs_only_cargo() {
    let bare = check(Backend::Rust, Ui::Clarity, &Machine::default());
    assert!(bare.generated);
    assert_eq!(bare.missing.len(), 1, "{:?}", bare.missing);
    assert!(bare.missing[0].contains("https://rustup.rs"), "{:?}", bare.missing);
    let ready = check(Backend::Rust, Ui::Clarity, &Machine::default().with("cargo", "--version", "cargo 1.91.0\n"));
    assert!(ready.ready(), "{:?}", ready.missing);
    assert_eq!(ready.found, ["cargo 1.91.0"]);
    if cfg!(target_os = "linux") {
        assert!(!bare.missing.iter().any(|m| m.contains("WebKitGTK")), "{:?}", bare.missing);
    }
    assert!(!clarity_gen::tools::requirements(Backend::Rust).iter().any(|m| m.contains("WebKitGTK")));
    assert!(!clarity_gen::tools::generates(Backend::Rust, Ui::Webview));
    let all = clarity_gen::tools::check_all(&Machine::default());
    assert!(all.iter().any(|s| s.backend == Backend::Rust && s.ui == Ui::Clarity));
    assert!(clarity_gen::tools::report(&all).contains("Rust with Clarity.UI: missing Rust with cargo"));
}

#[test]
fn csharp_needs_the_dotnet_sdk_the_project_targets() {
    let old = Machine::default().with("dotnet", "--list-sdks", "8.0.426 [/usr/share/dotnet/sdk]\n9.0.319 [/usr/share/dotnet/sdk]\n");
    let check_old = check(Backend::Csharp, Ui::Clarity, &old);
    assert!(!check_old.ready());
    assert!(check_old.missing[0].contains(".NET 10 SDK"), "{:?}", check_old.missing);
    // The runtime alone, without an SDK, cannot build.
    let runtime_only = Machine::default().with("dotnet", "--list-sdks", "");
    assert!(!check(Backend::Csharp, Ui::Clarity, &runtime_only).ready());

    let new = Machine::default().with("dotnet", "--list-sdks", "9.0.319 [/usr/share/dotnet/sdk]\n10.0.402 [/usr/share/dotnet/sdk]\n");
    let ready = check(Backend::Csharp, Ui::Clarity, &new);
    assert!(ready.ready(), "{:?}", ready.missing);
    assert_eq!(ready.found, [".NET SDK 10.0.402"]);
}

/// Go draws with Clarity.UI's library through cgo (DESIGN 7.9): Go and a C
/// compiler, no WebKitGTK; the library is the folder CLARITY_UI names.
#[test]
fn go_needs_go_and_a_c_compiler_for_cgo() {
    let go = check(Backend::Go, Ui::Clarity, &Machine::default());
    assert!(go.generated);
    assert!(go.missing[0].contains("https://go.dev/dl"), "{:?}", go.missing);
    assert!(go.missing.iter().any(|m| m.contains("cgo")), "{:?}", go.missing);
    let ready = check(Backend::Go, Ui::Clarity, &Machine::default().with("go", "version", "go version go1.25.7 linux/amd64\n").with("cc", "--version", "cc (Debian 14.2.0-19) 14.2.0\n"));
    // Clarity.UI's library is no tool to install: Clarity fetches it on the first build.
    assert_eq!(ready.found, ["go version go1.25.7 linux/amd64", "cc (Debian 14.2.0-19) 14.2.0", FETCHED_ON_FIRST_BUILD]);
    assert!(ready.missing.is_empty(), "{:?}", ready.missing);
    let needs = clarity_gen::tools::requirements(Backend::Go);
    assert!(needs.iter().any(|r| r.contains("CLARITY_UI")) && !needs.iter().any(|r| r.contains("WebKit")), "{needs:?}");
}

/// What the C and C++ projects build with on Linux besides the tools: the
/// libraries CMake finds through pkg-config.
fn with_libraries(machine: Machine) -> Machine {
    machine
        .with("pkg-config", "--exists sqlite3", "")
        .with("pkg-config", "--exists webkit2gtk-4.1", "")
        .with("pkg-config", "--exists nlohmann_json", "")
}

#[test]
fn c_and_cpp_need_a_compiler_cmake_and_their_libraries() {
    // C and C++ build with CMake (the generated project has a CMakeLists.txt) and make or Ninja.
    // C draws with Clarity.UI's library (DESIGN 7.9): no C++ compiler, no WebKitGTK.
    let c = check(
        Backend::C,
        Ui::Clarity,
        &with_libraries(
            Machine::default()
                .with("clang", "--version", "clang version 19.1.7\nTarget: x86_64\n")
                .with("clang++", "--version", "clang version 19.1.7\nTarget: x86_64\n")
                .with("cmake", "--version", "cmake version 3.31.6\n\nCMake suite maintained by Kitware\n")
                .with("make", "--version", "GNU Make 4.4.1\n")
                ,
        ),
    );
    assert!(c.generated);
    assert_eq!(c.found, ["clang version 19.1.7", "cmake version 3.31.6", "GNU Make 4.4.1", FETCHED_ON_FIRST_BUILD]);
    assert!(c.missing.is_empty(), "{:?}", c.missing);
    let cpp = check(Backend::Cpp, Ui::Clarity, &with_libraries(Machine::default().with("g++", "--version", "g++ (Debian 14.2.0-19) 14.2.0\n").with("ninja", "--version", "1.12.1\n")));
    assert!(cpp.generated);
    assert_eq!(cpp.found, ["g++ (Debian 14.2.0-19) 14.2.0", "ninja 1.12.1", FETCHED_ON_FIRST_BUILD]);
    assert_eq!(cpp.missing.len(), 1);
    assert!(cpp.missing[0].contains("CMake") && cpp.missing[0].contains("https://cmake.org/download"), "{:?}", cpp.missing);
    // C++ draws with Clarity.UI's library too: no WebKitGTK, no WebView2.
    let needs = clarity_gen::tools::requirements(Backend::Cpp);
    assert!(needs.iter().any(|r| r.contains("CLARITY_UI")) && !needs.iter().any(|r| r.contains("WebKit") || r.contains("WebView2")), "{needs:?}");
    let bare = check(Backend::C, Ui::Clarity, &Machine::default().with("cc", "--version", "cc (Debian 14.2.0-19) 14.2.0\n").with("cmake", "--version", "cmake version 3.31.6\n"));
    assert!(!bare.missing.iter().any(|m| m.contains("C++ compiler") || m.contains("WebKitGTK")), "{:?}", bare.missing);
    assert!(clarity_gen::tools::requirements(Backend::C).iter().any(|r| r.contains("CLARITY_UI")));
    assert!(bare.missing.iter().any(|m| m.contains("make or Ninja")), "{:?}", bare.missing);
    if cfg!(target_os = "linux") {
        assert!(bare.missing.iter().any(|m| m.contains("libsqlite3-dev")), "{:?}", bare.missing);
        let no_json = check(Backend::Cpp, Ui::Clarity, &Machine::default().with("g++", "--version", "g++ 14\n"));
        assert!(no_json.missing.iter().any(|m| m.contains("nlohmann-json3-dev")), "{:?}", no_json.missing);
    }
}

/// Clarity.UI's library for Go, C and C++: the folder CLARITY_UI names wins;
/// else the one fetched before; it is missing only where no release fits
/// the system, or the named folder lacks the header.
#[test]
fn clarity_ui_is_named_fetched_or_fetched_on_first_build() {
    let ready = |m: Machine| m.with("go", "version", "go1.25\n").with("cc", "--version", "cc 14\n");
    let dir = std::env::temp_dir().join(format!("clarity-tools-ui-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("include")).unwrap();
    std::fs::write(dir.join("include/clarity_ui.h"), "").unwrap();
    let named = check(Backend::Go, Ui::Clarity, &ready(Machine::default().library(Library::Named(dir.clone()))));
    assert!(named.missing.is_empty(), "{:?}", named.missing);
    assert_eq!(named.found[2], format!("Clarity.UI's library in {} (CLARITY_UI)", dir.display()));
    let wrong = check(Backend::Go, Ui::Clarity, &ready(Machine::default().library(Library::Named(dir.join("nothing")))));
    assert!(wrong.missing[0].contains("has no include/clarity_ui.h"), "{:?}", wrong.missing);
    let bundled = check(Backend::Go, Ui::Clarity, &ready(Machine::default().library(Library::Bundled(dir.clone()))));
    assert_eq!(bundled.found[2], format!("Clarity.UI's library {}, carried by Clarity", clarity_gen::clarity_ui::version()));
    let fetched = check(Backend::Go, Ui::Clarity, &ready(Machine::default().library(Library::Fetched(dir.clone()))));
    assert_eq!(fetched.found[2], format!("Clarity.UI's library {}", clarity_gen::clarity_ui::version()));
    for backend in [Backend::Go, Backend::C, Backend::Cpp] {
        let none = check(backend, Ui::Clarity, &Machine::default().library(Library::Unavailable));
        assert!(none.missing.iter().any(|m| m == clarity_gen::tools::CLARITY_UI_LIBRARY), "{:?}", none.missing);
    }
    // Rust and C# bring the library as a package.
    assert!(!check(Backend::Rust, Ui::Clarity, &Machine::default().library(Library::Unavailable)).missing.iter().any(|m| m.contains("CLARITY_UI")));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_program_is_found_on_the_path() {
    let dir = std::env::temp_dir().join(format!("clarity-tools-{}", std::process::id()));
    let bin = dir.join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    let name = if cfg!(windows) { "cargo.exe" } else { "cargo" };
    std::fs::write(bin.join(name), "").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(bin.join(name), std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let path = std::env::join_paths([dir.join("nothing"), bin.clone()]).unwrap();
    assert_eq!(find_on_path("cargo", &path), Some(bin.join(name)));
    assert_eq!(find_on_path("dotnet", &path), None);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// A program started from the desktop menu gets a shorter PATH than a
/// terminal: no ~/.cargo/bin, no /usr/local/go/bin. Clarity also looks in
/// the folders the official installers use.
#[test]
fn the_installers_folders_are_searched_after_the_path() {
    let home = std::path::Path::new("/home/someone");
    let path = std::env::join_paths(["/usr/bin", "/bin"]).unwrap();
    let searched: Vec<PathBuf> = std::env::split_paths(&search_path(Some(&path), Some(home))).collect();
    assert_eq!(searched[..2], [PathBuf::from("/usr/bin"), PathBuf::from("/bin")]);
    let installers: Vec<PathBuf> = if cfg!(windows) {
        vec![home.join(".cargo").join("bin"), home.join(".dotnet")]
    } else {
        vec![home.join(".cargo/bin"), home.join(".dotnet"), home.join("go/bin"), PathBuf::from("/usr/local/go/bin"), PathBuf::from("/usr/local/bin")]
    };
    for folder in installers {
        assert!(searched.contains(&folder), "{} not in {searched:?}", folder.display());
    }
    let mut unique = searched.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), searched.len(), "searched twice: {searched:?}");
}

#[test]
fn a_folder_already_on_the_path_is_not_searched_twice() {
    let home = std::path::Path::new("/home/someone");
    let path = std::env::join_paths([home.join(".cargo/bin"), PathBuf::from("/usr/bin")]).unwrap();
    let searched: Vec<PathBuf> = std::env::split_paths(&search_path(Some(&path), Some(home))).collect();
    assert_eq!(searched.iter().filter(|f| **f == home.join(".cargo/bin")).count(), 1);
    assert_eq!(searched[0], home.join(".cargo/bin"));
}

#[test]
fn every_stack_of_the_catalog_is_checked_and_reported() {
    let machine = Machine::default().with("dotnet", "--list-sdks", "10.0.402 [/usr/share/dotnet/sdk]\n");
    let all = clarity_gen::tools::check_all(&machine);
    assert!(all.iter().any(|s| s.backend == Backend::Rust && s.ui == Ui::Clarity));
    assert!(!all.iter().any(|s| s.backend == Backend::Rust && s.ui == Ui::Webview));
    assert!(all.iter().any(|s| s.backend == Backend::Csharp && s.ui == Ui::Clarity));
    let report = clarity_gen::tools::report(&all);
    assert!(report.contains("C# with Clarity.UI: ready (.NET SDK 10.0.402)\n"), "{report}");
    assert!(report.contains("Rust with Clarity.UI: missing Rust with cargo, from https://rustup.rs"), "{report}");
    assert!(report.contains("Go with Clarity.UI: missing Go, from https://go.dev/dl"), "{report}");
    // Where to read how, once something is missing.
    assert!(report.ends_with("How to install each: the manual's page Setting up the build tools (Manual in Clarity, or docs/manual/setup.tools.md).\n"), "{report}");
    let ready = clarity_gen::tools::report(&all.into_iter().filter(|s| s.missing.is_empty()).collect::<Vec<_>>());
    assert!(!ready.contains("How to install"), "{ready}");
}

/// Building an engine from its Rust source needs cargo and Rust's
/// WebAssembly target, which rustup adds.
#[test]
fn engines_need_cargo_and_the_webassembly_target() {
    let missing = engine_tools(&Machine::default());
    assert!(missing[0].contains("https://rustup.rs"), "{missing:?}");
    let without = Machine::default()
        .with("cargo", "--version", "cargo 1.91.0\n")
        .with("rustup", "target list --installed", "x86_64-unknown-linux-gnu\n");
    assert_eq!(engine_tools(&without), ["Rust's WebAssembly target: run rustup target add wasm32-unknown-unknown"]);
    let with = Machine::default()
        .with("cargo", "--version", "cargo 1.91.0\n")
        .with("rustup", "target list --installed", "x86_64-unknown-linux-gnu\nwasm32-unknown-unknown\n");
    assert!(engine_tools(&with).is_empty());
}

/// A folder tree under the test's own temporary folder, with these files.
fn tree(name: &str, files: &[&str]) -> PathBuf {
    let root = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("tools-{name}"));
    let _ = std::fs::remove_dir_all(&root);
    for file in files {
        let path = root.join(file);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, "").unwrap();
    }
    root
}

#[test]
fn visual_studio_s_compiler_cmake_and_ninja_are_found_in_its_installation() {
    let vs = tree(
        "visual-studio",
        &[
            "VC/Auxiliary/Build/Microsoft.VCToolsVersion.default.txt",
            "VC/Tools/MSVC/14.38.33130/bin/Hostx64/x64/cl.exe",
            "VC/Tools/MSVC/14.40.33807/bin/Hostx64/x64/cl.exe",
            "Common7/IDE/CommonExtensions/Microsoft/CMake/CMake/bin/cmake.exe",
            "Common7/IDE/CommonExtensions/Microsoft/CMake/Ninja/ninja.exe",
        ],
    );
    // The version Visual Studio names as its default wins; without it, the newest.
    std::fs::write(vs.join("VC/Auxiliary/Build/Microsoft.VCToolsVersion.default.txt"), "14.38.33130\r\n").unwrap();
    let folders = clarity_gen::tools::visual_studio_folders(&vs);
    assert_eq!(folders[0], vs.join("VC/Tools/MSVC/14.38.33130/bin/Hostx64/x64"));
    assert!(folders.contains(&vs.join("Common7/IDE/CommonExtensions/Microsoft/CMake/CMake/bin")));
    assert!(folders.contains(&vs.join("Common7/IDE/CommonExtensions/Microsoft/CMake/Ninja")));
    std::fs::remove_file(vs.join("VC/Auxiliary/Build/Microsoft.VCToolsVersion.default.txt")).unwrap();
    assert_eq!(clarity_gen::tools::visual_studio_folders(&vs)[0], vs.join("VC/Tools/MSVC/14.40.33807/bin/Hostx64/x64"));
}

#[test]
fn visual_studio_installations_are_found_in_program_files() {
    let root = tree(
        "program-files",
        &["Microsoft Visual Studio/2022/Community/VC/Tools/MSVC/14.40.33807/bin/Hostx64/x64/cl.exe", "Microsoft Visual Studio/2019/BuildTools/VC/Tools/MSVC/14.29.30133/bin/Hostx64/x64/cl.exe", "Microsoft Visual Studio/Installer/vswhere.exe"],
    );
    let found = clarity_gen::tools::visual_studio_installations(std::slice::from_ref(&root));
    assert_eq!(found, [root.join("Microsoft Visual Studio/2022/Community"), root.join("Microsoft Visual Studio/2019/BuildTools")]);
}

#[test]
fn windows_folders_cover_the_usual_installers() {
    let vars: HashMap<&str, &str> = [
        ("ProgramFiles", "C:\\Program Files"),
        ("SystemDrive", "C:"),
        ("LOCALAPPDATA", "C:\\Users\\me\\AppData\\Local"),
        ("ChocolateyInstall", "C:\\ProgramData\\chocolatey"),
    ]
    .into_iter()
    .collect();
    let folders: Vec<String> = clarity_gen::tools::windows_folders(&|name| vars.get(name).map(std::ffi::OsString::from), Some(std::path::Path::new("C:\\Users\\me")))
        .iter()
        .map(|f| f.to_string_lossy().replace('\\', "/"))
        .collect();
    for wanted in [
        "C:/Program Files/Go/bin",
        "C:/Program Files/dotnet",
        "C:/Program Files/CMake/bin",
        "C:/Program Files/LLVM/bin",
        "C:/msys64/ucrt64/bin",
        "C:/msys64/mingw64/bin",
        "C:/msys64/clang64/bin",
        "C:/Users/me/scoop/shims",
        "C:/ProgramData/chocolatey/bin",
        "C:/Users/me/AppData/Local/Microsoft/WinGet/Links",
    ] {
        assert!(folders.iter().any(|f| f == wanted), "{wanted} in {folders:?}");
    }
}

#[test]
fn visual_studio_s_cl_counts_as_a_c_and_cpp_compiler() {
    // cl has no --version: it says who it is when run alone, on its error output.
    let machine = Machine::default()
        .with("cl", "", "Microsoft (R) C/C++ Optimizing Compiler Version 19.40.33811 for x64\n")
        .with("cmake", "--version", "cmake version 3.29.5\n")
        .with("ninja", "--version", "1.11.1\n");
    let cpp = check(Backend::Cpp, Ui::Clarity, &with_libraries(machine));
    assert!(cpp.found.contains(&"Microsoft (R) C/C++ Optimizing Compiler Version 19.40.33811 for x64".to_string()), "{:?}", cpp.found);
    assert!(!cpp.missing.iter().any(|m| m.contains("compiler")), "{:?}", cpp.missing);
}

#[test]
fn mingw_s_make_counts_as_make() {
    let machine = Machine::default().with("g++", "--version", "g++ (Rev2, Built by MSYS2 project) 14.2.0\n").with("cmake", "--version", "cmake version 3.31.6\n").with("mingw32-make", "--version", "GNU Make 4.4.1\n");
    let cpp = check(Backend::Cpp, Ui::Clarity, &with_libraries(machine));
    assert!(cpp.found.contains(&"GNU Make 4.4.1".to_string()), "{:?}", cpp.found);
    assert!(!cpp.missing.iter().any(|m| m.contains("make or Ninja")), "{:?}", cpp.missing);
}

/// A heading's anchor as the manual's window makes it (app/src/lib/markdown.ts):
/// lower case, without marks, words joined by hyphens.
fn anchor(heading: &str) -> String {
    let kept: String = heading.to_lowercase().chars().filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == ' ' || *c == '-').collect();
    kept.split_whitespace().collect::<Vec<_>>().join("-")
}

/// Each missing tool points at the part of the manual's setup page that
/// tells how to install it on this system, and every part the check can
/// point at, on any system, is a heading of that page.
#[test]
fn every_missing_tool_points_at_the_manual() {
    use clarity_gen::tools::{SETUP_TOPIC, guide_headings};
    assert_eq!(SETUP_TOPIC, "setup.tools");
    let page = include_str!("../../../docs/manual/setup.tools.md");
    let anchors: Vec<String> = page.lines().filter_map(|l| l.strip_prefix("### ").or_else(|| l.strip_prefix("## "))).map(anchor).collect();
    for heading in guide_headings() {
        assert!(anchors.iter().any(|a| a == heading), "the setup page has no heading #{heading}: {anchors:?}");
    }
    // A machine without anything misses every tool, each with its part of the page.
    for (backend, ui) in [(Backend::Rust, Ui::Clarity), (Backend::Rust, Ui::Clarity), (Backend::Csharp, Ui::Clarity), (Backend::Go, Ui::Clarity), (Backend::C, Ui::Clarity), (Backend::Cpp, Ui::Clarity)] {
        let bare = check(backend, ui, &Machine::default());
        assert_eq!(bare.missing.len(), bare.guides.len(), "{backend:?}: {:?} {:?}", bare.missing, bare.guides);
        assert!(bare.guides.iter().all(|g| guide_headings().contains(g)), "{:?}", bare.guides);
    }
    let rust = check(Backend::Rust, Ui::Clarity, &Machine::default());
    let expected = if cfg!(windows) { "rust-on-windows" } else if cfg!(target_os = "macos") { "rust-on-macos" } else { "rust-on-linux" };
    assert_eq!(rust.guides[0], expected);
}

/// The web target of C# needs nothing beyond the SDK: its server serves
/// Clarity.UI's browser files, which Clarity carries (DESIGN 7.9, B2).
#[test]
fn the_web_target_of_csharp_needs_only_the_sdk() {
    let sdk = "10.0.402 [/usr/share/dotnet/sdk]\n";
    let tools = check(Backend::Csharp, Ui::Clarity, &Machine::default().with("dotnet", "--list-sdks", sdk));
    assert!(tools.ready(), "{:?}", tools.missing);
    assert!(!tools.missing.iter().chain(&tools.found).any(|t| t.contains("wasm-tools")), "{tools:?}");
}

/// No stack asks for Node.js or npm: a web stack's windows are markup the
/// screen Clarity ships reads, so building an application needs only the
/// stack's own tools (DESIGN 7.8).
#[test]
fn no_stack_needs_node() {
    for (backend, ui) in [(Backend::Rust, Ui::Clarity), (Backend::Rust, Ui::Clarity), (Backend::Go, Ui::Clarity), (Backend::C, Ui::Clarity), (Backend::Cpp, Ui::Clarity), (Backend::Csharp, Ui::Clarity)] {
        let check = check(backend, ui, &Machine::default());
        assert!(!check.missing.iter().any(|m| m.contains("Node") || m.contains("npm")), "{backend:?}: {:?}", check.missing);
        assert!(!clarity_gen::tools::requirements(backend).iter().any(|r| r.contains("Node")), "{backend:?}");
    }
}
