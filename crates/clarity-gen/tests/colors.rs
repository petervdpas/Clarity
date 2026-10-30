//! The color field type of U1c on every stack: kept as text in every
//! backend; the web stacks draw the color input from the model, Avalonia
//! writes Clarity's ColorInput.

use clarity_gen::{OutFile, generate};
use clarity_project::{Backend, Settings, Ui, starter};
use clarity_spec::keyed::load_keyed;

/// The build tests' fixture, whose Album has a color with its input.
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
fn every_backend_keeps_a_color_as_text_with_its_default() {
    has(&generated(Backend::Rust, Ui::Web), "String::from(\"#3b82f6\")");
    has(&generated(Backend::Go, Ui::Web), "\"#3b82f6\"");
    has(&generated(Backend::C, Ui::Web), "\"#3b82f6\"");
    has(&generated(Backend::Cpp, Ui::Web), "\"#3b82f6\"");
    has(&generated(Backend::Csharp, Ui::Avalonia), "\"#3b82f6\"");
    has(&generated(Backend::Rust, Ui::Web), "\"kind\":\"color\"");
}

#[test]
fn avalonia_writes_the_color_input() {
    let code = generated(Backend::Csharp, Ui::Avalonia);
    has(&code, "<c:ColorInput x:Name=\"tintBox\"");
    has(&code, "Value=\"{Binding Album.Tint}\"");
    has(&code, "tintBox.ValueChanged += (_, _) => Vm.EditOf(\"tintBox\", \"tintBox\", true, () => Vm.FavouriteChangedCommand.Execute(null));");
}
