//! Context menus and popovers (DESIGN 5.16) on every stack: the web stacks
//! run the menu's handlers as any handler and keep a popover's `visible` in
//! the part's state; Avalonia writes its ContextMenu and Popup, bound to the
//! parts' state.

use clarity_gen::{OutFile, generate};
use clarity_project::{Backend, Settings, Ui, starter};
use clarity_spec::keyed::load_keyed;

/// The build tests' fixture: a dialog whose table has a context menu and a popover with a filter.
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
fn the_web_stacks_close_a_popover_and_disable_an_item_in_the_parts_state() {
    has(&generated(Backend::Rust, Ui::Web), "ui.set_visible(\"filters\", false)");
    has(&generated(Backend::Go, Ui::Web), "ui.SetVisible(\"filters\", false)");
    has(&generated(Backend::C, Ui::Web), "clarity_ui_set_visible(ui, \"filters\", false)");
    has(&generated(Backend::Cpp, Ui::Web), "ui.set_visible(\"filters\", false)");
    has(&generated(Backend::Rust, Ui::Web), "ui.set_enabled(\"pickItem\"");
}

#[test]
fn avalonia_writes_a_context_menu_and_a_popup() {
    let code = generated(Backend::Csharp, Ui::Avalonia);
    has(&code, "<Panel.ContextMenu>");
    has(&code, "<MenuItem Header=");
    has(&code, "Command=\"{Binding PickedCommand}\" IsEnabled=\"{Binding PickItem.IsEnabled}\" IsVisible=\"{Binding PickItem.IsVisible}\"");
    has(&code, "Command=\"{Binding Show_filtersCommand}\"");
    has(&code, "<Popup x:Name=\"filters\" IsOpen=\"{Binding Filters.IsVisible, Mode=TwoWay}\" PlacementTarget=\"{Binding ElementName=filterButton}\"");
    has(&code, "IsLightDismissEnabled=\"True\"");
    has(&code, "<Border Classes=\"popover\" Width=\"362\">");
    has(&code, "<Style Selector=\"Border.popover\">");
    has(&code, "public PartState Filters { get; } = new(\"filters\") { IsVisible = false };");
    has(&code, "public PartState PickItem { get; } = new(\"pickItem\"");
    has(&code, "void Show_filters() => Filters.IsVisible = true;");
}
