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
    menu name mainMenu
        item \"File\" name fileItem
            item \"New customer\" name newCustomerItem key \"Ctrl+N\" on click -> CustomerCard
            separator
            item \"Quit\" name quitItem on click -> quit
        item \"Help\" name helpItem
            item \"About\" name aboutItem on click -> Overview
    toolbar name mainToolbar
        button \"New\" name newButton icon \"plus\" on click -> CustomerCard
        separator
    ribbon name mainRibbon
        tab \"Home\" name homeTab
            group \"Customers\" name customersGroup
                button \"New\" name newRibbon icon \"plus\" large on click -> CustomerCard
    sidebar left name leftSidebar width 192
        list name list1 at 1,1 span 3 rows 10 bind recent show name
    footer
        status \"Ready\"
        label \"v1\" name label1 at 11,1 span 2
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
    assert_eq!(app.frame.sidebars[0].props.side, Side::Left);
    assert_eq!(app.frame.sidebars[0].props.columns, 3);
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
        "module m\napp A\n    menu name mainMenu\n        button \"X\" name xButton\n    toolbar name mainToolbar\n        item \"Y\" name yItem\n";
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
    let source = "module m\napp A\n    menu name mainMenu\n        item \"A\" name aItem\n    menu name mainMenu\n        item \"B\" name bItem\n    sidebar left name leftSidebar width 192\n    sidebar left name leftSidebar width 192\n";
    let (_, diags) = parse(source, "a.clarity");
    let messages: Vec<_> = diags.iter().map(|d| d.message.as_str()).collect();
    assert!(messages.contains(&"duplicate 'menu'"));
    assert!(messages.contains(&"duplicate 'sidebar left'"));
}

#[test]
fn only_windows_have_regions() {
    let pages = "module p\npage V: view\n    menu name mainMenu\n        item \"A\" name a2\n";
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
    let app = APP.replace("label \"v1\" name label1 at 11,1 span 2", "label \"v1\" name label1 at 11,2 span 2");
    assert!(
        errors(&app)
            .iter()
            .any(|m| m.contains("a footer has one row"))
    );
}

#[test]
fn sidebar_widgets_fit_the_sidebar() {
    let app = APP.replace("list name list1 at 1,1 span 3", "list name list1 at 1,1 span 4");
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
        "button \"New\" name newButton icon \"plus\" on click -> CustomerCard\n        separator",
        "button \"New\" name newButton icon \"plus\" on click: newClicked\n        separator",
    );
    // Wired but without code behind yet: a warning, not an error.
    assert_eq!(
        diagnostics(&app),
        vec![(Severity::Warning, "handler 'newClicked' is wired but not written yet".to_string())]
    );
    let (file, _) = parse(&app, "shop.clarity");
    let toolbar = file.apps[0].frame.toolbar.as_ref().unwrap();
    assert_eq!(toolbar.items[0].props.name.as_deref(), Some("newButton"));
    assert_eq!(toolbar.items[0].props.handler.as_deref(), Some("newClicked"));
    // A name is unique across the window, menus and toolbars included.
    let clash = app.replace("item \"Quit\" name quitItem", "item \"Quit\" name newButton");
    assert!(errors(&clash).iter().any(|e| e.contains("duplicate part name 'newButton'")), "{:#?}", errors(&clash));
}

/// A window shows the app's regions, each of its own replacing the app's of
/// the same kind (a sidebar, of the same side); a dialog shows none.
#[test]
fn a_window_shows_its_own_regions_before_the_apps() {
    let pages = "module p
page Plain: window
    title \"Plain\"
page Own: window
    toolbar name mainToolbar
        button \"Mine\" name mineButton on click -> Plain
    sidebar right name rightSidebar width 128 grid 2 columns
page Ask: dialog
    title \"Ask\"
";
    let app = "module shop
use p

app Shop
    main Plain
    menu name mainMenu
        item \"File\" name file2
    toolbar name mainToolbar
        button \"Theirs\" name theirsButton on click -> Plain
    sidebar left name leftSidebar width 192
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
    assert_eq!(own.left.map(|s| s.props.columns), Some(3));
    assert_eq!(own.right.map(|s| s.props.columns), Some(2));
    assert!(own.menu.is_some());

    let ask = clarity_spec::frame::shown(app, page("Ask"));
    assert!(ask.menu.is_none() && ask.toolbar.is_none() && ask.footer.is_none() && ask.left.is_none());
}

/// `-> Window in new window`: the one way a design asks for a desktop window
/// of its own (DESIGN.md, Page kinds); without it, a window opens in place.
#[test]
fn a_click_target_can_open_in_a_new_window() {
    let app = "module shop
use p

app Shop
    main CustomerCard
    menu name mainMenu
        item \"File\" name file3
            item \"New customer\" name newCustomerItem on click -> CustomerCard in new window
            item \"Card\" name cardItem on click -> CustomerCard
";
    let pages = "module p
page CustomerCard: window
    title \"Card\"
    button \"Other\" name button1 at 1,1 span 2 on click -> CustomerCard in new window
";
    let project = load_sources([("p.clarity", pages), ("shop.clarity", app)]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let app = project.apps().next().unwrap();
    let items = &app.frame.menu.as_ref().unwrap().items[0].items;
    assert!(items[0].props.new_window);
    assert!(!items[1].props.new_window);
    assert_eq!(clarity_spec::frame::command_line(&items[0].props), "item \"New customer\" name newCustomerItem on click -> CustomerCard in new window");
    let button = &project.pages().next().unwrap().widgets[0].props;
    assert!(button.new_window);
    assert!(clarity_spec::widgets::widget_line(button).ends_with("on click -> CustomerCard in new window"), "{}", clarity_spec::widgets::widget_line(button));

    // Only a window opens in a new window.
    let quit = "module shop\nuse p\n\napp Shop\n    menu name mainMenu\n        item \"Quit\" name quitItem on click -> quit in new window\n";
    assert!(errors(quit).iter().any(|e| e == "only a window opens in a new window"), "{:?}", errors(quit));
}

/// Ribbon and toolbar buttons have keys too (DESIGN 5.28), as menu items do.
#[test]
fn ribbon_and_toolbar_buttons_have_keys() {
    let app = APP
        .replace("button \"New\" name newButton icon \"plus\" on click -> CustomerCard", "button \"New\" name newButton icon \"plus\" key \"Ctrl+N\" on click -> CustomerCard")
        .replace("button \"New\" name newRibbon icon \"plus\" large on click -> CustomerCard", "button \"New\" name newRibbon icon \"plus\" large key \"Ctrl+Shift+N\" on click -> CustomerCard");
    assert_eq!(errors(&app), Vec::<String>::new());
    let (file, _) = parse(&app, "shop.clarity");
    let frame = &file.apps[0].frame;
    assert_eq!(frame.toolbar.as_ref().unwrap().items[0].props.shortcut.as_deref(), Some("Ctrl+N"));
    assert_eq!(frame.ribbon.as_ref().unwrap().tabs[0].groups[0].items[0].props.shortcut.as_deref(), Some("Ctrl+Shift+N"));
}

/// One key runs one command where it is active (DESIGN 5.28): in a window's
/// frame, and among its own buttons. The same target on a menu item and a
/// toolbar button is one command; a window's own button may reuse a frame's
/// key on purpose, since what is innermost goes first.
#[test]
fn one_key_runs_one_command_where_it_is_active() {
    // Ctrl+N on the menu item and the toolbar button, both opening CustomerCard: one command.
    let same = APP.replace("button \"New\" name newButton icon \"plus\" on click -> CustomerCard", "button \"New\" name newButton icon \"plus\" key \"Ctrl+N\" on click -> CustomerCard");
    assert_eq!(errors(&same), Vec::<String>::new());
    // Ctrl+N opening Overview from the toolbar: two commands.
    let two = APP.replace("button \"New\" name newButton icon \"plus\" on click -> CustomerCard", "button \"New\" name newButton icon \"plus\" key \"Ctrl+N\" on click -> Overview");
    assert!(
        errors(&two).iter().any(|e| e == "key 'Ctrl+N' runs both newCustomerItem and newButton in the app's frame; give one of them another key"),
        "{:#?}",
        errors(&two)
    );
    // A window's own button may take the frame's key; two of its buttons may not share one.
    let pages = PAGES.replace(
        "page CustomerCard: window\n    title \"Card\"\n",
        "page CustomerCard: window\n    title \"Card\"\n    button \"Save\" name saveButton at 1,1 span 2 key \"Ctrl+N\" on click: saveClicked\n    button \"Keep\" name keepButton at 3,1 span 2 key \"Ctrl+N\" on click: keepClicked\n",
    );
    let found: Vec<String> = load_sources([("p.clarity", pages.as_str()), ("shop.clarity", APP)])
        .diagnostics
        .into_iter()
        .filter(|d| d.severity == Severity::Error)
        .map(|d| d.message)
        .collect();
    assert_eq!(found, ["key 'Ctrl+N' runs both saveButton and keepButton in window CustomerCard; give one of them another key"]);
}
