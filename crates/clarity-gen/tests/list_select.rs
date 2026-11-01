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
    assert!(view.contains("Classes=\"line\""), "a title and a line: {view}");
    assert!(view.contains("<ListBox ") && view.contains("Classes=\"cards\""), "{view}");
    // Its look is the theme's (App.axaml reads the theme's values), not the window's.
    assert!(!view.contains("ClarityAccent") && !view.contains("CornerRadius="), "{view}");
    let app = files.iter().find(|f| f.path.ends_with("App.axaml")).map(|f| f.text.as_str()).unwrap_or_default();
    for key in ["ClarityCardBackground", "ClarityCardSelectedBackground", "ClarityCardLineText", "ClarityListSelectedBackground", "ClarityCardPadding"] {
        assert!(app.contains(&format!("{{DynamicResource {key}}}")), "{key} in the application's styles");
    }
    let theme: String = files.iter().filter(|f| f.path.contains("Themes/")).map(|f| f.text.as_str()).collect();
    for key in ["ClarityCardBackground", "ClarityCardSelectedBackground", "ClarityCardLineText", "ClarityListSelectedBackground", "ClarityCardPadding", "ClarityCardRadius"] {
        assert!(theme.contains(&format!("x:Key=\"{key}\"")), "{key} in the themes");
    }
}

/// A list's and a card's look come from the theme (`part list`, `part card`,
/// `part card_line`, with `hover` and `selected`), alike on every stack.
#[test]
fn a_list_and_its_cards_look_as_the_theme_says() {
    let theme = "module m\n\ntheme Shop\n    part card background panel, border accent, radius 10\n    part card selected background success\n    part card_line text warning\n    part list selected background error_soft\n";
    let (file, diagnostics) = clarity_spec::parser::parse(theme, "m.clarity");
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let css = clarity_gen::web::theme_css(&file.themes[0]);
    for line in ["--card-bg: var(--panel)", "--card-radius: 10px", "--card-selected-bg: var(--success)", "--card-line-text: var(--warning)", "--list-selected-bg: var(--error-soft)"] {
        assert!(css.contains(line), "{line} in {css}");
    }
    let base = clarity_gen::base_css();
    for line in ["--list-bg:", "--list-selected-bg:", "--card-bg:", "--card-selected-bg:", "--card-line-text:"] {
        assert!(base.contains(line), "Clarity's own theme sets {line}: {base}");
    }
}

/// A splitter (`pane span 4`) starts its first pane at 4 of its 12 columns,
/// framed, its panes padded and the divider dragged, as on the web.
#[test]
fn a_splitter_starts_its_first_pane_at_its_span() {
    let mut settings = starter("Plan", "plan");
    settings.stack.backend = Backend::Csharp;
    settings.stack.ui = Ui::Avalonia;
    let split = PLAN.replace(
        "    list name itemList at 1,1 span 4 rows 4 bind items show title on select: itemSelected\n",
        "    splitter name split at 1,1 span 12 rows 4\n        pane span 4\n            list name itemList at 1,1 span 4 rows 4 bind items show title on select: itemSelected\n        pane\n            text name note at 1,1 span 8\n",
    );
    let project = clarity_spec::load_in([("plan.clarity", split.as_str())], [("0001.clarity", FIRST)], &clarity_project::source_refs(&settings));
    let errors: Vec<&str> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.as_str()).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    let files = generate(&project, &settings).unwrap();
    let view = files.iter().find(|f| f.path.ends_with("MainView.axaml")).map(|f| f.text.as_str()).unwrap_or_default();
    assert!(view.contains("<Border Classes=\"splitter\""), "{view}");
    assert!(view.contains("ColumnDefinitions=\"4*,Auto,8*\""), "{view}");
    assert!(view.contains("<Border Classes=\"pane\" Grid.Column=\"0\">"), "{view}");
    let app = files.iter().find(|f| f.path.ends_with("App.axaml")).map(|f| f.text.as_str()).unwrap_or_default();
    assert!(app.contains("<Style Selector=\"Border.pane\">"), "the panes are padded");
}

/// A part that does not grow keeps the height of its rows when a list beside
/// it makes them grow: a button stays a button's height, at the top.
#[test]
fn a_button_beside_a_list_keeps_its_height() {
    let mut settings = starter("Plan", "plan");
    settings.stack.backend = Backend::Csharp;
    settings.stack.ui = Ui::Avalonia;
    let beside = PLAN.replace(
        "    list name itemList at 1,1 span 4 rows 4 bind items show title on select: itemSelected\n",
        "    list name itemList at 1,1 span 4 rows 4 bind items show title on select: itemSelected\n    button \"Add\" name addItem at 5,4 span 2\n    textbox name note at 5,1 span 4 rows 2\n",
    );
    let project = clarity_spec::load_in([("plan.clarity", beside.as_str())], [("0001.clarity", FIRST)], &clarity_project::source_refs(&settings));
    let errors: Vec<&str> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.as_str()).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    let files = generate(&project, &settings).unwrap();
    let view = files.iter().find(|f| f.path.ends_with("MainView.axaml")).map(|f| f.text.as_str()).unwrap_or_default();
    let button = view.lines().find(|l| l.contains("x:Name=\"AddItem\"") || l.contains("Name=\"addItem\"")).unwrap_or_else(|| panic!("{view}"));
    assert!(button.contains("VerticalAlignment=\"Top\"") && button.contains("Height=\"36\""), "{button}");
    let note = view.lines().find(|l| l.contains("Name=\"note\"") || l.contains("Name=\"Note\"")).unwrap_or_else(|| panic!("{view}"));
    assert!(note.contains("Height=\"80\""), "two rows and the gap between: {note}");
    let list = view.lines().find(|l| l.contains("<ListBox")).unwrap();
    assert!(!list.contains("VerticalAlignment=\"Top\""), "the list grows: {list}");
}

/// A list or table in a container (a tab page, a pane, a panel) shows no
/// border of its own, its container framing it, unless it has `frame`; one
/// on the window keeps its border (DESIGN 7.8), as on the web.
#[test]
fn a_list_in_a_container_has_no_border_of_its_own_unless_framed() {
    let mut settings = starter("Plan", "plan");
    settings.stack.backend = Backend::Csharp;
    settings.stack.ui = Ui::Avalonia;
    let view_of = |source: &str| {
        let project = clarity_spec::load_in([("plan.clarity", source)], [("0001.clarity", FIRST)], &clarity_project::source_refs(&settings));
        let errors: Vec<&str> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.as_str()).collect();
        assert!(errors.is_empty(), "{errors:#?}");
        let files = generate(&project, &settings).unwrap();
        files.iter().find(|f| f.path.ends_with("MainView.axaml")).map(|f| f.text.clone()).unwrap_or_default()
    };
    let list_line = |view: &str| view.lines().find(|l| l.contains("<ListBox")).unwrap_or_default().to_string();
    let on_window = view_of(PLAN);
    assert!(!list_line(&on_window).contains("BorderThickness=\"0\""), "{}", list_line(&on_window));
    let in_panel = PLAN.replace(
        "    list name itemList at 1,1 span 4 rows 4 bind items show title on select: itemSelected\n",
        "    panel \"Items\" name itemPanel at 1,1 span 4 rows 5\n        list name itemList at 1,1 span 4 rows 4 bind items show title on select: itemSelected\n",
    );
    assert!(list_line(&view_of(&in_panel)).contains("BorderThickness=\"0\""), "{}", list_line(&view_of(&in_panel)));
    let framed = in_panel.replace("show title on select", "show title frame on select");
    assert!(!list_line(&view_of(&framed)).contains("BorderThickness=\"0\""), "{}", list_line(&view_of(&framed)));
}
