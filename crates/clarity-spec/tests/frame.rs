use clarity_spec::model::{CommandKind, Side};
use clarity_spec::parser::parse;
use clarity_spec::{Severity, load_sources};

const PAGES: &str = "module p
entity Customer
    name: text
page CustomerCard: window
    title \"Card\"
page Overview: view
    title \"Overview\"
";

const APP: &str = "module shop
use p

app Shop
    title \"Shop\"
    main CustomerCard
    data recent: list<Customer>
    menu
        item \"File\"
            item \"New customer\" key \"Ctrl+N\" on click -> CustomerCard
            separator
            item \"Quit\" on click -> quit
        item \"Help\"
            item \"About\" on click -> Overview
    toolbar
        button \"New\" icon \"add\" on click -> CustomerCard
        separator
    ribbon
        tab \"Home\"
            group \"Customers\"
                button \"New\" icon \"add\" large on click -> CustomerCard
    sidebar left span 3
        list at 1,1 span 3 rows 10 bind recent show name
    footer
        status \"Ready\"
        label \"v1\" at 11,1 span 2
";

fn diagnostics(app: &str) -> Vec<(Severity, String)> {
    load_sources([("p.clarity", PAGES), ("shop.clarity", app)])
        .diagnostics
        .into_iter()
        .map(|d| (d.severity, d.message))
        .collect()
}

fn errors(app: &str) -> Vec<String> {
    diagnostics(app)
        .into_iter()
        .filter(|(s, _)| *s == Severity::Error)
        .map(|(_, m)| m)
        .collect()
}

#[test]
fn parses_an_app_with_every_region() {
    let (file, diags) = parse(APP, "shop.clarity");
    assert!(diags.is_empty(), "{diags:?}");
    let app = &file.apps[0];
    assert_eq!(app.title.as_deref(), Some("Shop"));
    assert_eq!(app.main.as_ref().unwrap().name, "CustomerCard");
    let menu = app.frame.menu.as_ref().unwrap();
    assert_eq!(menu.items.len(), 2);
    let file_menu = &menu.items[0];
    assert_eq!(file_menu.items.len(), 3);
    assert_eq!(file_menu.items[0].props.shortcut.as_deref(), Some("Ctrl+N"));
    assert_eq!(file_menu.items[1].props.kind, CommandKind::Separator);
    let ribbon = app.frame.ribbon.as_ref().unwrap();
    assert!(ribbon.tabs[0].groups[0].items[0].props.large);
    assert_eq!(app.frame.sidebars[0].side, Side::Left);
    assert_eq!(app.frame.sidebars[0].width, 3);
    let footer = app.frame.footer.as_ref().unwrap();
    assert_eq!(footer.status.as_deref(), Some("Ready"));
    assert_eq!(footer.widgets.len(), 1);
}

#[test]
fn the_example_app_checks_clean() {
    assert_eq!(diagnostics(APP), vec![]);
}

#[test]
fn reports_misplaced_commands() {
    let source =
        "module m\napp A\n    menu\n        button \"X\"\n    toolbar\n        item \"Y\"\n";
    let (_, diags) = parse(source, "a.clarity");
    let messages: Vec<_> = diags.iter().map(|d| d.message.as_str()).collect();
    assert!(
        messages
            .iter()
            .any(|m| m.contains("expected 'item' or 'separator', found 'button'"))
    );
    assert!(
        messages
            .iter()
            .any(|m| m.contains("expected 'button' or 'separator', found 'item'"))
    );
}

#[test]
fn reports_duplicate_regions() {
    let source = "module m\napp A\n    menu\n        item \"A\"\n    menu\n        item \"B\"\n    sidebar left\n    sidebar left\n";
    let (_, diags) = parse(source, "a.clarity");
    let messages: Vec<_> = diags.iter().map(|d| d.message.as_str()).collect();
    assert!(messages.contains(&"duplicate 'menu'"));
    assert!(messages.contains(&"duplicate 'sidebar left'"));
}

#[test]
fn only_windows_have_regions() {
    let pages = "module p\npage V: view\n    menu\n        item \"A\"\n";
    let errors: Vec<_> = load_sources([("p.clarity", pages)])
        .diagnostics
        .into_iter()
        .filter(|d| d.severity == Severity::Error)
        .map(|d| d.message)
        .collect();
    assert!(
        errors.iter().any(|m| m.contains("only windows have menus")),
        "{errors:?}"
    );
}

#[test]
fn main_must_be_a_window() {
    let app = APP.replace("main CustomerCard", "main Overview");
    assert!(
        errors(&app)
            .iter()
            .any(|m| m.contains("must be a page of kind window"))
    );
    let app = APP.replace("main CustomerCard", "main Nowhere");
    assert!(
        errors(&app)
            .iter()
            .any(|m| m.contains("'Nowhere' is not a window"))
    );
}

#[test]
fn a_project_has_one_app() {
    let second = "module other\napp Second\n    main X\n";
    let messages: Vec<_> = load_sources([
        ("p.clarity", PAGES),
        ("shop.clarity", APP),
        ("o.clarity", second),
    ])
    .diagnostics
    .into_iter()
    .map(|d| d.message)
    .collect();
    assert!(
        messages.iter().any(|m| m.contains("a project has one app")),
        "{messages:?}"
    );
}

#[test]
fn footer_widgets_stay_on_one_row() {
    let app = APP.replace("label \"v1\" at 11,1 span 2", "label \"v1\" at 11,2 span 2");
    assert!(
        errors(&app)
            .iter()
            .any(|m| m.contains("a footer has one row"))
    );
}

#[test]
fn sidebar_widgets_fit_the_sidebar() {
    let app = APP.replace("list at 1,1 span 3", "list at 1,1 span 4");
    assert!(
        errors(&app)
            .iter()
            .any(|m| m.contains("runs past column 3"))
    );
}

#[test]
fn a_submenu_item_has_no_click_target() {
    let app = APP.replace("item \"File\"", "item \"File\" on click -> CustomerCard");
    assert!(errors(&app).iter().any(|m| m.contains("opens the submenu")));
}

#[test]
fn commands_have_names_and_click_handlers() {
    let app = APP.replace(
        "button \"New\" icon \"add\" on click -> CustomerCard\n        separator",
        "button \"New\" name new_customer icon \"add\" on click: new_clicked\n        separator",
    );
    // Wired but without code behind yet: a warning, not an error.
    assert_eq!(
        diagnostics(&app),
        vec![(Severity::Warning, "handler 'new_clicked' is wired but not written yet".to_string())]
    );
    let (file, _) = parse(&app, "shop.clarity");
    let toolbar = file.apps[0].frame.toolbar.as_ref().unwrap();
    assert_eq!(toolbar.items[0].props.name.as_deref(), Some("new_customer"));
    assert_eq!(toolbar.items[0].props.handler.as_deref(), Some("new_clicked"));
    // A name is unique across the window, menus and toolbars included.
    let clash = app.replace("item \"Quit\"", "item \"Quit\" name new_customer");
    assert!(errors(&clash).iter().any(|e| e.contains("duplicate part name 'new_customer'")));
}

/// A window shows the app's regions, each of its own replacing the app's of
/// the same kind (a sidebar, of the same side); a dialog shows none.
#[test]
fn a_window_shows_its_own_regions_before_the_apps() {
    let pages = "module p
page Plain: window
    title \"Plain\"
page Own: window
    toolbar
        button \"Mine\" on click -> Plain
    sidebar right span 2
page Ask: dialog
    title \"Ask\"
";
    let app = "module shop
use p

app Shop
    main Plain
    menu
        item \"File\"
    toolbar
        button \"Theirs\" on click -> Plain
    sidebar left span 3
    footer
        status \"Ready\"
";
    let project = load_sources([("p.clarity", pages), ("shop.clarity", app)]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let app = project.apps().next();
    let page = |name: &str| project.pages().find(|p| p.name.name == name).unwrap();

    let plain = clarity_spec::frame::shown(app, page("Plain"));
    assert!(plain.menu.is_some() && plain.footer.is_some() && plain.left.is_some() && plain.right.is_none());
    assert_eq!(plain.toolbar.unwrap().items[0].props.caption.as_deref(), Some("Theirs"));

    let own = clarity_spec::frame::shown(app, page("Own"));
    assert_eq!(own.toolbar.unwrap().items[0].props.caption.as_deref(), Some("Mine"));
    assert_eq!(own.left.map(|s| s.width), Some(3));
    assert_eq!(own.right.map(|s| s.width), Some(2));
    assert!(own.menu.is_some());

    let ask = clarity_spec::frame::shown(app, page("Ask"));
    assert!(ask.menu.is_none() && ask.toolbar.is_none() && ask.footer.is_none() && ask.left.is_none());
}
