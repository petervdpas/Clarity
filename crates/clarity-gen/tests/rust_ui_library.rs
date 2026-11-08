//! A Rust desktop program gets Clarity.UI in one of two ways (DESIGN 7.9):
//! by default it links the ready-made library through the runtime's
//! `native_ui`, as Go, C and C++ do, so it builds no Skia; with `ui crate`
//! in its project file it builds the clarity-ui crate from source, as a Rust
//! custom part needs. The project decides. A change of Clarity.UI is tried
//! by rebuilding the base libraries (`clarity libraries`).

use clarity_gen::{OutFile, generate};
use clarity_project::{Backend, Settings, Ui, starter};

const SOURCE: &str = "module notes

page Main: window
    data note: text
    textbox name noteBox at 1,1 span 4 bind note
";

fn generated(settings: &Settings) -> Vec<OutFile> {
    let project = clarity_spec::keyed::load_keyed([("notes.clarity", SOURCE)], &[], "en");
    generate(&project, settings).unwrap_or_else(|e| panic!("{e:#?}"))
}

fn file<'a>(files: &'a [OutFile], path: &str) -> &'a str {
    files.iter().find(|f| f.path == path).map(|f| f.text.as_str()).unwrap_or_else(|| panic!("no {path}"))
}

fn rust() -> Settings {
    let mut settings = starter("Notes", "notes");
    settings.stack.backend = Backend::Rust;
    settings.stack.ui = Ui::Clarity;
    settings
}

#[test]
fn a_rust_desktop_program_links_the_ready_made_library_by_default() {
    let files = generated(&rust());
    let cargo = file(&files, "Cargo.toml");
    assert!(!cargo.contains("clarity-ui"), "no crate to build: {cargo}");
    assert!(cargo.contains("\"native-ui\""), "{cargo}");
    let main = file(&files, "src/main.rs");
    assert!(main.contains("clarity_runtime::native_ui::run(app, FILES)"), "{main}");
    assert!(main.contains("const FILES: clarity_runtime::native_ui::AppFiles"), "{main}");
    assert!(!main.contains("clarity_ui::"), "nothing of the crate: {main}");
    let build = file(&files, "build.rs");
    assert!(build.contains("CLARITY_UI") && build.contains("rustc-link-lib=dylib={name}") && build.contains("const LIBRARIES: &[&str] = &[\"clarity_ui\"];"), "{build}");
    assert!(clarity_gen::uses_clarity_ui(&rust()), "Clarity's build names the library's folder");
}

#[test]
fn ui_crate_builds_clarity_ui_from_source() {
    let settings = Settings { ui_crate: true, ..rust() };
    let files = generated(&settings);
    let cargo = file(&files, "Cargo.toml");
    assert!(cargo.contains("clarity-ui = \""), "{cargo}");
    assert!(!cargo.contains("native-ui"), "{cargo}");
    assert!(file(&files, "src/main.rs").contains("clarity_ui::desktop::run(app, FILES)"));
    assert!(!files.iter().any(|f| f.path == "build.rs"), "nothing to link");
    assert!(!clarity_gen::uses_clarity_ui(&settings));
}

#[test]
fn the_base_libraries_are_rebuilt_inside_clarity_s_repository() {
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap();
    assert_eq!(clarity_gen::repository_of(&repo.join("examples/modelling")), Some(repo.clone()));
    assert_eq!(clarity_gen::repository_of(std::path::Path::new("/")), None);
    assert!(clarity_gen::BASE_LIBRARIES.contains(&"clarity-ui-ffi"));
}
