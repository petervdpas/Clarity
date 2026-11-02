//! The accordion and the scroll box of U1b on every stack: the web stacks
//! read and set an accordion's open section in the part's state; C# keeps
//! it in the window's view model, which Clarity.UI draws.

use clarity_gen::{OutFile, generate};
use clarity_project::{Backend, Settings, Ui, starter};
use clarity_spec::keyed::load_keyed;

/// The build tests' fixture, whose Albums window has an accordion and a scroll box.
const MUSIC: &str = include_str!("fixtures/asks.clarity");

fn generated(backend: Backend, ui: Ui) -> String {
    let project = load_keyed([("music.clarity", MUSIC)], &[], "en");
    let errors: Vec<&str> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.as_str()).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    let mut settings: Settings = starter("Music", "music");
    settings.stack.backend = backend;
    settings.stack.ui = ui;
    let files: Vec<OutFile> = generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"));
    files.iter().map(|f| format!("// {}\n{}\n", f.path, f.text)).collect()
}

fn has(text: &str, expected: &str) {
    assert!(text.contains(expected), "missing {expected:?}");
}

#[test]
fn the_web_stacks_read_and_set_the_open_section_in_the_parts_state() {
    let rust = generated(Backend::Rust, Ui::Clarity);
    has(&rust, "ui.open_section(\"notesAccordion\")");
    has(&rust, "ui.set_open_section(\"notesAccordion\", 2");
    let go = generated(Backend::Go, Ui::Webview);
    has(&go, "ui.OpenSection(\"notesAccordion\")");
    has(&go, "ui.SetOpenSection(\"notesAccordion\", 2)");
    has(&generated(Backend::C, Ui::Webview), "clarity_ui_set_open_section(ui, \"notesAccordion\"");
    has(&generated(Backend::Cpp, Ui::Webview), "ui.set_open_section(\"notesAccordion\"");
}

#[test]
fn csharp_opens_an_accordion_section_in_its_state() {
    let code = generated(Backend::Csharp, Ui::Clarity);
    has(&code, "NotesAccordion.OpenSection = 2");
}
