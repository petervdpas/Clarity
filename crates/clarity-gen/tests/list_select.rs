//! A list's `on select` runs when the person chooses a row, not when the
//! list's rows are read again and its choice is cleared: in C# the list's
//! SelectionChanged then has no row added, and a handler that reads
//! `list.selected` would find none.

use clarity_gen::generate;
use clarity_project::{Backend, Ui, starter};

const FIRST: &str = "migration \"First\"
    add entity Item
        title: text(80)
";

const PLAN: &str = "module plan

entity Item
    title: text(80)

app Plan
    main Main

page Main: window
    data items: list<Item>
    data current: Item
    list name itemList at 1,1 span 4 rows 4 bind items show title on select: itemSelected

code Main
    handler itemSelected
        current = itemList.selected
";

#[test]
fn a_list_runs_on_select_only_for_a_row_chosen() {
    let mut settings = starter("Plan", "plan");
    settings.stack.backend = Backend::Csharp;
    settings.stack.ui = Ui::Avalonia;
    let project = clarity_spec::load_in([("plan.clarity", PLAN)], [("0001.clarity", FIRST)], &clarity_project::source_refs(&settings));
    let errors: Vec<&str> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.as_str()).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    let code: String = generate(&project, &settings).unwrap().iter().map(|f| f.text.as_str()).collect();
    assert!(code.contains("itemList.SelectionChanged += (_, e) => { if (e.AddedItems.Count > 0) Vm.ItemSelectedCommand.Execute(null); };"), "{}", code.lines().filter(|l| l.contains("SelectionChanged")).collect::<Vec<_>>().join("\n"));
}

/// A window's grid is as high as the window (the runtime's WindowClient), so
/// rows that grow share it and a long list scrolls in its rows; a window
/// lower than the rows at their least scrolls.
#[test]
fn a_window_with_rows_that_grow_scrolls_when_it_is_small() {
    let mut settings = starter("Plan", "plan");
    settings.stack.backend = Backend::Csharp;
    settings.stack.ui = Ui::Avalonia;
    let project = clarity_spec::load_in([("plan.clarity", PLAN)], [("0001.clarity", FIRST)], &clarity_project::source_refs(&settings));
    let files = generate(&project, &settings).unwrap();
    let view = files.iter().find(|f| f.path.ends_with("MainView.axaml")).map(|f| f.text.as_str()).unwrap_or_default();
    let scroll = view.find("<ScrollViewer HorizontalScrollBarVisibility=\"Disabled\" VerticalScrollBarVisibility=\"Auto\">").expect("the window scrolls");
    assert!(view[scroll..].trim_start_matches(|c| c != '\n').trim_start().starts_with("<c:WindowClient>"), "the window's grid is in its client: {view}");
    assert!(view.contains("Height=\"*\" MinHeight="), "the list's rows still grow: {view}");
}

/// A list of cards (`cards`): each row a card, its first field the title and the others lines below it.
#[test]
fn a_list_of_cards_shows_each_row_as_a_card() {
    let mut settings = starter("Plan", "plan");
    settings.stack.backend = Backend::Csharp;
    settings.stack.ui = Ui::Avalonia;
    let carded = PLAN.replace("show title on select", "show title, title cards on select");
    let project = clarity_spec::load_in([("plan.clarity", carded.as_str())], [("0001.clarity", FIRST)], &clarity_project::source_refs(&settings));
    let errors: Vec<&str> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.as_str()).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    let files = generate(&project, &settings).unwrap();
    let view = files.iter().find(|f| f.path.ends_with("MainView.axaml")).map(|f| f.text.as_str()).unwrap_or_default();
    assert!(view.contains("<Border Classes=\"card\""), "{view}");
    assert!(view.contains("FontWeight=\"SemiBold\" />") && view.contains("Foreground=\"{DynamicResource ClarityAccent}\" />"), "a title and a line: {view}");
}
