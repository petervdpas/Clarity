//! The clipboard on every stack: `copy text` hands the text, as it shows,
//! to the screen with the handler's reply.

use clarity_gen::{OutFile, generate};
use clarity_project::{Backend, Ui, starter};

const SOURCE: &str = "module notes

page Output: window
    data diagram: text
    button \"Copy\" name copyButton at 1,1 span 2 on click: copyClicked

code Output
    handler copyClicked
        copy diagram
";

fn generated(backend: Backend, ui: Ui) -> String {
    let project = clarity_spec::keyed::load_keyed([("notes.clarity", SOURCE)], &[], "en");
    let mut settings = starter("Notes", "notes");
    settings.stack.backend = backend;
    settings.stack.ui = ui;
    let files: Vec<OutFile> = generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"));
    files.iter().map(|f| f.text.as_str()).collect::<Vec<_>>().join("\n")
}

fn has(text: &str, expected: &str) {
    assert!(text.contains(expected), "expected\n{expected}\nin the generated program");
}

#[test]
fn every_stack_copies_to_the_clipboard() {
    has(&generated(Backend::Rust, Ui::Web), "ui.copy(");
    has(&generated(Backend::Go, Ui::Web), "ui.Copy(");
    has(&generated(Backend::C, Ui::Web), "clarity_ui_copy(ui, ");
    has(&generated(Backend::Cpp, Ui::Web), "ui.copy(");
    has(&generated(Backend::Csharp, Ui::Avalonia), "Copy(");
}
