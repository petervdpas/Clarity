//! The button kinds of U1b on every stack: the web stacks read and set an
//! unbound toggle as `checked` in the part's state; Avalonia writes a
//! ToggleButton, a joined group and a SplitButton with a MenuFlyout.

use clarity_gen::{OutFile, generate};
use clarity_project::{Backend, Settings, Ui, starter};
use clarity_spec::keyed::load_keyed;

/// The build tests' fixture, whose Albums window has a toggle, a group and a split button.
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
fn the_web_stacks_read_and_set_an_unbound_toggle_as_checked() {
    has(&generated(Backend::Rust, Ui::Web), "ui.set_checked(\"rightToggle\", !ui.checked(\"leftToggle\"))");
    has(&generated(Backend::Go, Ui::Web), "ui.SetChecked(\"rightToggle\", !ui.Checked(\"leftToggle\"))");
    has(&generated(Backend::C, Ui::Web), "clarity_ui_set_checked(ui, \"rightToggle\"");
    has(&generated(Backend::Cpp, Ui::Web), "ui.set_checked(\"rightToggle\"");
}

#[test]
fn avalonia_writes_a_toggle_button_a_joined_group_and_a_split_button() {
    let code = generated(Backend::Csharp, Ui::Avalonia);
    has(&code, "<ToggleButton x:Name=\"favouriteToggle\"");
    has(&code, "IsChecked=\"{Binding Album.Favourite}\"");
    has(&code, "IsChecked=\"{Binding RightToggle.IsChecked}\"");
    has(&code, "favouriteToggle.Click += (_, _) => Vm.FavouriteChangedCommand.Execute(null);");
    has(&code, "Classes=\"buttons\"");
    has(&code, "ColumnSpacing=\"0\"");
    has(&code, "<SplitButton x:Name=\"saveButton\"");
    has(&code, "<MenuFlyout Placement=\"BottomEdgeAlignedLeft\">");
    has(&code, "Command=\"{Binding NewClickedCommand}\"");
    has(&code, "public PartState FavouriteToggle { get; } = new(\"favouriteToggle\", caption: \"favouriteToggleCaption\")");
}
