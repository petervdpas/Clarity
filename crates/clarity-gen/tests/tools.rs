//! The build tools of each stack (P3). Clarity is installed as a finished
//! program, so a machine may lack cargo or the .NET SDK; Clarity looks for
//! them and says what to install, while designing and generating keep
//! working. The probe is faked here, so the tests need no tools.

use std::collections::HashMap;
use std::path::PathBuf;

use clarity_gen::tools::{Probe, check, engine_tools, find_on_path, search_path};
use clarity_project::{Backend, Ui};

/// A machine with these programs, each answering one command line.
#[derive(Default)]
struct Machine {
    programs: HashMap<&'static str, HashMap<String, &'static str>>,
}

impl Machine {
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
}

#[test]
fn rust_needs_cargo_and_on_linux_webkitgtk() {
    let empty = check(Backend::Rust, Ui::Web, &Machine::default());
    assert!(!empty.ready());
    assert!(empty.generated);
    assert!(empty.missing[0].contains("Rust") && empty.missing[0].contains("https://rustup.rs"), "{:?}", empty.missing);

    let machine = Machine::default()
        .with("cargo", "--version", "cargo 1.91.0 (ea2d97820 2025-10-10)\n")
        .with("pkg-config", "--exists webkit2gtk-4.1", "");
    let ready = check(Backend::Rust, Ui::Web, &machine);
    assert!(ready.ready(), "{:?}", ready.missing);
    assert_eq!(ready.found, ["cargo 1.91.0 (ea2d97820 2025-10-10)"]);

    if cfg!(target_os = "linux") {
        let without = Machine::default().with("cargo", "--version", "cargo 1.91.0\n");
        let check = check(Backend::Rust, Ui::Web, &without);
        assert_eq!(check.missing.len(), 1);
        assert!(check.missing[0].contains("libwebkit2gtk-4.1-dev"), "{:?}", check.missing);
    }
}

#[test]
fn avalonia_needs_the_dotnet_sdk_the_project_targets() {
    let old = Machine::default().with("dotnet", "--list-sdks", "8.0.426 [/usr/share/dotnet/sdk]\n9.0.319 [/usr/share/dotnet/sdk]\n");
    let check_old = check(Backend::Csharp, Ui::Avalonia, &old);
    assert!(!check_old.ready());
    assert!(check_old.missing[0].contains(".NET 10 SDK"), "{:?}", check_old.missing);
    // The runtime alone, without an SDK, cannot build.
    let runtime_only = Machine::default().with("dotnet", "--list-sdks", "");
    assert!(!check(Backend::Csharp, Ui::Avalonia, &runtime_only).ready());

    let new = Machine::default().with("dotnet", "--list-sdks", "9.0.319 [/usr/share/dotnet/sdk]\n10.0.402 [/usr/share/dotnet/sdk]\n");
    let ready = check(Backend::Csharp, Ui::Avalonia, &new);
    assert!(ready.ready(), "{:?}", ready.missing);
    assert_eq!(ready.found, [".NET SDK 10.0.402"]);
}

#[test]
fn go_needs_go_and_on_linux_webkitgtk() {
    let go = check(Backend::Go, Ui::Web, &Machine::default());
    // Go with Wails is generated (G3b); Wails builds against WebKitGTK on Linux, like Tauri.
    assert!(go.generated);
    assert!(go.missing[0].contains("https://go.dev/dl"), "{:?}", go.missing);
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
    // C needs a C++ compiler too, for the webview's one C++ file.
    let c = check(
        Backend::C,
        Ui::Web,
        &with_libraries(
            Machine::default()
                .with("clang", "--version", "clang version 19.1.7\nTarget: x86_64\n")
                .with("clang++", "--version", "clang version 19.1.7\nTarget: x86_64\n")
                .with("cmake", "--version", "cmake version 3.31.6\n\nCMake suite maintained by Kitware\n")
                .with("make", "--version", "GNU Make 4.4.1\n"),
        ),
    );
    assert!(c.generated);
    assert_eq!(c.found, ["clang version 19.1.7", "clang version 19.1.7", "cmake version 3.31.6", "GNU Make 4.4.1"]);
    assert!(c.missing.is_empty(), "{:?}", c.missing);
    let cpp = check(Backend::Cpp, Ui::Web, &with_libraries(Machine::default().with("g++", "--version", "g++ (Debian 14.2.0-19) 14.2.0\n").with("ninja", "--version", "1.12.1\n")));
    assert!(cpp.generated);
    assert_eq!(cpp.found, ["g++ (Debian 14.2.0-19) 14.2.0", "ninja 1.12.1"]);
    assert_eq!(cpp.missing.len(), 1);
    assert!(cpp.missing[0].contains("CMake") && cpp.missing[0].contains("https://cmake.org/download"), "{:?}", cpp.missing);
    let bare = check(Backend::C, Ui::Web, &Machine::default().with("cc", "--version", "cc (Debian 14.2.0-19) 14.2.0\n").with("cmake", "--version", "cmake version 3.31.6\n"));
    assert!(bare.missing.iter().any(|m| m.contains("C++ compiler")), "{:?}", bare.missing);
    assert!(bare.missing.iter().any(|m| m.contains("make or Ninja")), "{:?}", bare.missing);
    if cfg!(target_os = "linux") {
        assert!(bare.missing.iter().any(|m| m.contains("libsqlite3-dev")), "{:?}", bare.missing);
        assert!(bare.missing.iter().any(|m| m.contains("libwebkit2gtk-4.1-dev")), "{:?}", bare.missing);
        let no_json = check(Backend::Cpp, Ui::Web, &Machine::default().with("g++", "--version", "g++ 14\n"));
        assert!(no_json.missing.iter().any(|m| m.contains("nlohmann-json3-dev")), "{:?}", no_json.missing);
    }
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
    assert!(all.iter().any(|s| s.backend == Backend::Rust && s.ui == Ui::Web));
    assert!(all.iter().any(|s| s.backend == Backend::Csharp && s.ui == Ui::Avalonia));
    let report = clarity_gen::tools::report(&all);
    assert!(report.contains("C# with Avalonia: ready (.NET SDK 10.0.402)\n"), "{report}");
    assert!(report.contains("Rust with the web renderer: missing Rust with cargo, from https://rustup.rs"), "{report}");
    assert!(report.contains("Go with the web renderer: missing Go, from https://go.dev/dl"), "{report}");
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
    let cpp = check(Backend::Cpp, Ui::Web, &with_libraries(machine));
    assert!(cpp.found.contains(&"Microsoft (R) C/C++ Optimizing Compiler Version 19.40.33811 for x64".to_string()), "{:?}", cpp.found);
    assert!(!cpp.missing.iter().any(|m| m.contains("compiler")), "{:?}", cpp.missing);
}

#[test]
fn mingw_s_make_counts_as_make() {
    let machine = Machine::default().with("g++", "--version", "g++ (Rev2, Built by MSYS2 project) 14.2.0\n").with("cmake", "--version", "cmake version 3.31.6\n").with("mingw32-make", "--version", "GNU Make 4.4.1\n");
    let cpp = check(Backend::Cpp, Ui::Web, &with_libraries(machine));
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
    for (backend, ui) in [(Backend::Rust, Ui::Web), (Backend::Csharp, Ui::Avalonia), (Backend::Go, Ui::Web), (Backend::C, Ui::Web), (Backend::Cpp, Ui::Web)] {
        let bare = check(backend, ui, &Machine::default());
        assert_eq!(bare.missing.len(), bare.guides.len(), "{backend:?}: {:?} {:?}", bare.missing, bare.guides);
        assert!(bare.guides.iter().all(|g| guide_headings().contains(g)), "{:?}", bare.guides);
    }
    let rust = check(Backend::Rust, Ui::Web, &Machine::default());
    let expected = if cfg!(windows) { "rust-on-windows" } else if cfg!(target_os = "macos") { "rust-on-macos" } else { "rust-on-linux" };
    assert_eq!(rust.guides[0], expected);
}
