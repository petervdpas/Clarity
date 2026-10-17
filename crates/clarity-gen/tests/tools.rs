//! The build tools of each stack (P3). Clarity is installed as a finished
//! program, so a machine may lack cargo or the .NET SDK; Clarity looks for
//! them and says what to install, while designing and generating keep
//! working. The probe is faked here, so the tests need no tools.

use std::collections::HashMap;
use std::path::PathBuf;

use clarity_gen::tools::{Probe, check, find_on_path};
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
fn stacks_clarity_does_not_generate_yet_still_say_what_they_need() {
    let go = check(Backend::Go, Ui::Web, &Machine::default());
    assert!(!go.generated);
    assert!(go.missing[0].contains("https://go.dev/dl"), "{:?}", go.missing);
    // C and C++ build with CMake (the generated project has a CMakeLists.txt) and make or Ninja.
    let c = check(
        Backend::C,
        Ui::Web,
        &Machine::default()
            .with("clang", "--version", "clang version 19.1.7\nTarget: x86_64\n")
            .with("cmake", "--version", "cmake version 3.31.6\n\nCMake suite maintained by Kitware\n")
            .with("make", "--version", "GNU Make 4.4.1\n"),
    );
    assert_eq!(c.found, ["clang version 19.1.7", "cmake version 3.31.6", "GNU Make 4.4.1"]);
    assert!(c.missing.is_empty());
    let cpp = check(Backend::Cpp, Ui::Web, &Machine::default().with("g++", "--version", "g++ (Debian 14.2.0-19) 14.2.0\n").with("ninja", "--version", "1.12.1\n"));
    assert_eq!(cpp.found, ["g++ (Debian 14.2.0-19) 14.2.0", "ninja 1.12.1"]);
    assert_eq!(cpp.missing.len(), 1);
    assert!(cpp.missing[0].contains("CMake") && cpp.missing[0].contains("https://cmake.org/download"), "{:?}", cpp.missing);
    let bare = check(Backend::C, Ui::Web, &Machine::default().with("cc", "--version", "cc (Debian 14.2.0-19) 14.2.0\n").with("cmake", "--version", "cmake version 3.31.6\n"));
    assert!(bare.missing[0].contains("make or Ninja"), "{:?}", bare.missing);
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

#[test]
fn every_stack_of_the_catalog_is_checked_and_reported() {
    let machine = Machine::default().with("dotnet", "--list-sdks", "10.0.402 [/usr/share/dotnet/sdk]\n");
    let all = clarity_gen::tools::check_all(&machine);
    assert!(all.iter().any(|s| s.backend == Backend::Rust && s.ui == Ui::Web));
    assert!(all.iter().any(|s| s.backend == Backend::Csharp && s.ui == Ui::Avalonia));
    let report = clarity_gen::tools::report(&all);
    assert!(report.contains("C# with Avalonia: ready (.NET SDK 10.0.402)\n"), "{report}");
    assert!(report.contains("Rust with the web renderer: missing Rust with cargo, from https://rustup.rs"), "{report}");
    assert!(report.contains("Go with the web renderer: not generated yet; missing Go, from https://go.dev/dl\n"), "{report}");
}
