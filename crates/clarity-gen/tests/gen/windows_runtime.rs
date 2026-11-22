//! Clarity's Windows libraries need Microsoft's C++ runtime (VCRUNTIME140.dll,
//! MSVCP140.dll), which a fresh Windows has only with the Visual C++
//! Redistributable. The libraries ship it beside themselves
//! (.github/scripts/vc_runtime.py, checked by check_dll_imports.py), and
//! every stack that links them puts it beside the program with them.

use std::path::{Path, PathBuf};

use clarity_project::{Backend, Settings, Target, Ui, starter};

const SOURCE: &str = "module notes

page Main: window
    data note: text
    textbox name noteBox at 1,1 span 4 bind note
";

fn repo(path: &str) -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(path)).unwrap()
}

/// The runtime's files, as vc_runtime.py copies them.
fn runtime_files() -> Vec<String> {
    let script = repo(".github/scripts/vc_runtime.py");
    let line = script.lines().find(|l| l.starts_with("FILES = [")).expect("vc_runtime.py names its FILES");
    line.split('"').skip(1).step_by(2).map(String::from).collect()
}

/// Whether a file name is one the stacks copy as the runtime.
fn copied_as_runtime(name: &str) -> bool {
    name.starts_with("vcruntime140") || name.starts_with("msvcp140")
}

#[test]
fn the_libraries_ship_the_runtime_their_imports_need() {
    let files = runtime_files();
    assert_eq!(files, ["vcruntime140.dll", "vcruntime140_1.dll", "msvcp140.dll"]);
    assert!(files.iter().all(|f| copied_as_runtime(f)), "every file the stacks copy: {files:?}");
    for workflow in [".github/workflows/release-clarity-ui.yml", ".github/scripts/stage-clarity-ui.sh"] {
        let text = repo(workflow);
        assert!(text.contains("vc_runtime.py") && text.contains("check_dll_imports.py"), "{workflow} ships the runtime and checks the imports");
    }
}

#[test]
fn rust_copies_the_runtime_beside_the_program() {
    let mut settings: Settings = starter("Notes", "notes");
    settings.stack.backend = Backend::Rust;
    settings.stack.ui = Ui::Clarity;
    let project = clarity_spec::keyed::load_keyed([("notes.clarity", SOURCE)], &[], "en");
    let files = clarity_gen::generate(&project, &settings).unwrap();
    let build = &files.iter().find(|f| f.path == "build.rs").unwrap().text;
    assert!(build.contains("name.starts_with(\"vcruntime140\") || name.starts_with(\"msvcp140\")"), "{build}");
}

#[test]
fn c_and_cpp_copy_the_runtime_beside_the_program() {
    for (cmake, function) in [("runtimes/c/clarity.cmake", "function(clarity_beside target)"), ("runtimes/cpp/clarity.cmake", "function(clarity_ui_beside target)")] {
        let text = repo(cmake);
        let body = &text[text.find(function).unwrap_or_else(|| panic!("{cmake} has {function}"))..];
        let body = &body[..body.find("endfunction()").unwrap()];
        assert!(body.contains("\"${CLARITY_UI}/lib/vcruntime140*.dll\" \"${CLARITY_UI}/lib/msvcp140*.dll\"") && body.contains("${CLARITY_VC_RUNTIME} $<TARGET_FILE_DIR:${target}>"), "{cmake}: {body}");
    }
}

/// Go has no build step of its own after `go build`: Clarity puts the
/// libraries beside the program, the runtime with them.
#[test]
fn go_gets_the_libraries_beside_the_program() {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("windows-runtime-go");
    let _ = std::fs::remove_dir_all(&root);
    let lib = root.join("clarity-ui/lib");
    std::fs::create_dir_all(&lib).unwrap();
    for name in ["clarity_ui.dll", "clarity_ui.dll.lib", "vcruntime140.dll", "msvcp140.dll", "libclarity_ui.so", "openh264-2.6.0-win64.dll"] {
        std::fs::write(lib.join(name), name).unwrap();
    }
    let bin = root.join("out/bin");
    clarity_gen::libraries_beside(&root.join("clarity-ui"), &bin.join("calculator.exe"), "windows").unwrap();
    let mut placed: Vec<String> = std::fs::read_dir(&bin).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    placed.sort();
    assert_eq!(placed, ["clarity_ui.dll", "msvcp140.dll", "openh264-2.6.0-win64.dll", "vcruntime140.dll"]);
    clarity_gen::libraries_beside(&root.join("clarity-ui"), &root.join("linux/bin/calculator"), "linux").unwrap();
    assert!(root.join("linux/bin/libclarity_ui.so").is_file());
    assert!(!root.join("linux/bin/clarity_ui.dll").exists());
    std::fs::remove_dir_all(&root).unwrap();
}

fn generated(backend: Backend, targets: &[Target]) -> Vec<clarity_gen::OutFile> {
    let mut settings: Settings = starter("Notes", "notes");
    settings.stack.backend = backend;
    settings.stack.ui = Ui::Clarity;
    settings.stack.targets = targets.to_vec();
    let project = clarity_spec::keyed::load_keyed([("notes.clarity", SOURCE)], &[], "en");
    clarity_gen::generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"))
}

fn text<'a>(files: &'a [clarity_gen::OutFile], path: &str) -> &'a str {
    files.iter().find(|f| f.path == path).map(|f| f.text.as_str()).unwrap_or_else(|| panic!("no {path}"))
}

/// A program that is only a window opens no console on Windows (started from
/// Explorer, a console would open beside its window); one that also serves a
/// browser keeps it, for what `--serve` says.
#[test]
fn a_window_program_opens_no_console_on_windows() {
    let rust = generated(Backend::Rust, &[Target::Desktop]);
    assert!(text(&rust, "src/main.rs").starts_with("// Generated by Clarity. Do not edit: change the project in Clarity instead.\n// A window program: no console of its own on Windows.\n#![windows_subsystem = \"windows\"]\n"), "{}", text(&rust, "src/main.rs"));
    assert!(!text(&generated(Backend::Rust, &[Target::Desktop, Target::Web]), "src/main.rs").contains("windows_subsystem"));
    for backend in [Backend::C, Backend::Cpp] {
        let lists = text(&generated(backend, &[Target::Desktop]), "CMakeLists.txt").to_string();
        assert!(lists.contains("clarity_window_program(notes)"), "{backend:?}: {lists}");
        assert!(!text(&generated(backend, &[Target::Desktop, Target::Web]), "CMakeLists.txt").contains("clarity_window_program"), "{backend:?}");
    }
    for cmake in ["runtimes/c/clarity.cmake", "runtimes/cpp/clarity.cmake"] {
        let text = repo(cmake);
        assert!(text.contains("function(clarity_window_program target)") && text.contains("WIN32_EXECUTABLE TRUE") && text.contains("/ENTRY:mainCRTStartup"), "{cmake}");
    }
}

#[test]
fn a_go_window_program_is_built_with_no_console_on_windows() {
    let settings = |targets: &[Target]| {
        let mut s: Settings = starter("Notes", "notes");
        s.stack.backend = Backend::Go;
        s.stack.ui = Ui::Clarity;
        s.stack.targets = targets.to_vec();
        s
    };
    assert_eq!(clarity_gen::go_ldflags(&settings(&[Target::Desktop]), false, "windows"), Some("-ldflags=-H=windowsgui".to_string()));
    assert_eq!(clarity_gen::go_ldflags(&settings(&[Target::Desktop]), true, "windows"), Some("-ldflags=-s -w -H=windowsgui".to_string()));
    assert_eq!(clarity_gen::go_ldflags(&settings(&[Target::Desktop]), true, "linux"), Some("-ldflags=-s -w".to_string()));
    assert_eq!(clarity_gen::go_ldflags(&settings(&[Target::Desktop, Target::Web]), false, "windows"), None);
    assert_eq!(clarity_gen::go_ldflags(&settings(&[Target::Desktop]), false, "linux"), None);
}

/// C#'s packages carry the runtime once, in Clarity.Runtime, which every
/// package with a native library stands on: one file in several packages
/// would break `dotnet publish`.
#[test]
fn csharp_carries_the_runtime_in_clarity_runtime() {
    let workflow = repo(".github/workflows/release-csharp-runtime.yml");
    assert!(workflow.contains("vc_runtime.py") && workflow.contains("check_dll_imports.py"), "the native jobs ship the runtime and check the imports");
    assert!(workflow.contains("runtimes/csharp/Clarity.Runtime/runtimes/$rid/native"), "the pack job places it in Clarity.Runtime");
    assert!(workflow.contains("runtimes/win-x64/native/vcruntime140.dll"), "the pack job checks Clarity.Runtime carries it");
    let csproj = repo("runtimes/csharp/Clarity.Runtime/Clarity.Runtime.csproj");
    assert!(csproj.contains("<None Include=\"runtimes/**/*\" Pack=\"true\" PackagePath=\"runtimes/\" />"), "{csproj}");
    for native in ["Clarity.Ui", "Clarity.Audio", "Clarity.Video", "Clarity.Image", "Clarity.Formats", "Clarity.Auth", "Clarity.AppMcp"] {
        let project = repo(&format!("runtimes/csharp/{native}/{native}.csproj"));
        assert!(project.contains("../Clarity.Runtime/Clarity.Runtime.csproj") || project.contains("../Clarity.Ui/Clarity.Ui.csproj"), "{native} stands on Clarity.Runtime");
    }
}
