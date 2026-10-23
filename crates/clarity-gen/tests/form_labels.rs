//! A form's label column (U1b) on every stack: the labels the spec makes
//! for it are parts like any other, in the model the web stacks carry and
//! in the AXAML and part states of Avalonia, and code sets them by name.

use clarity_gen::{OutFile, generate};
use clarity_project::{Backend, Settings, Ui, starter};
use clarity_spec::keyed::load_keyed;

/// The build tests' fixture, whose Albums window has a form with `labels 3`.
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
fn the_web_stacks_carry_the_made_labels_and_set_them_by_name() {
    let rust = generated(Backend::Rust, Ui::Web);
    has(&rust, "\"name\":\"albumTitleBoxLabel\"");
    has(&rust, "\"made\":true");
    has(&rust, "ui.set_visible(\"albumCopiesBoxLabel\"");
    has(&generated(Backend::Go, Ui::Web), "ui.SetVisible(\"albumCopiesBoxLabel\"");
}

#[test]
fn avalonia_writes_the_made_labels_as_its_labels() {
    let code = generated(Backend::Csharp, Ui::Avalonia);
    has(&code, "x:Name=\"albumTitleBoxLabel\"");
    has(&code, "public PartState AlbumCopiesBoxLabel { get; }");
    has(&code, "AlbumCopiesBoxLabel.IsVisible = ");
    has(&code, "x:Key=\"ClarityFramePadding\"");
}
