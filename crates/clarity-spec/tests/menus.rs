//! A menu and a toolbar: named parts of a window's (or the app's) frame.
//! Code hides and disables them by their names, as it does any part.

use clarity_spec::cxaml::apply;
use clarity_spec::edit::Edit;
use clarity_spec::frame::RegionKind;
use clarity_spec::parser::parse;
use clarity_spec::{Severity, load_converted};

const WINDOW: &str = "module m

page Main: window
    menu name mainMenu
        item \"File\" name fileItem
            item \"Quit\" name quitItem on click -> quit
    toolbar name mainToolbar
        button \"Save\" name saveButton on click: saveClick
    text name mainText at 1,1 span 6

code Main
    handler saveClick
        mainToolbar.visible = false
        mainMenu.enabled = not mainMenu.enabled
";

fn errors(source: &str) -> Vec<String> {
    load_converted([("m.clarity", source)]).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
}

fn assert_has(messages: &[String], needle: &str) {
    assert!(messages.iter().any(|m| m.contains(needle)), "missing {needle:?} in {messages:#?}");
}

#[test]
fn a_menu_and_a_toolbar_have_names() {
    let (file, diags) = parse(WINDOW, "m.clarity");
    assert!(diags.is_empty(), "{diags:#?}");
    let frame = &file.pages[0].frame;
    assert_eq!(frame.menu.as_ref().unwrap().name.as_deref(), Some("mainMenu"));
    assert_eq!(frame.toolbar.as_ref().unwrap().name.as_deref(), Some("mainToolbar"));
    assert!(errors(WINDOW).is_empty(), "{:#?}", errors(WINDOW));
}

#[test]
fn a_menu_and_a_toolbar_are_checked() {
    assert_has(&errors(&WINDOW.replace("menu name mainMenu", "menu")), "the menu has no name; every part has one");
    assert_has(&errors(&WINDOW.replace("toolbar name mainToolbar", "toolbar")), "the toolbar has no name; every part has one");
    assert_has(&errors(&WINDOW.replace("name mainToolbar", "name main_toolbar")), "part 'main_toolbar' is not camelCase");
    assert_has(&errors(&WINDOW.replace("menu name mainMenu", "menu big")), "unknown menu property 'big'");
    assert_has(&errors(&WINDOW.replace("name mainToolbar", "name saveButton")), "duplicate part name 'saveButton'");
    assert_has(&errors(&WINDOW.replace("mainMenu.enabled = not mainMenu.enabled", "mainMenu.caption = \"x\"")), "menu 'mainMenu' has no property 'caption'");
}


/// The `.cxaml` the page or app `name` of `source` becomes.
fn design(source: &str, name: &str) -> String {
    let file = format!("design/{name}.cxaml");
    clarity_spec::converted([("m/m.clarity", source)])
        .into_iter()
        .find(|(p, _)| p.ends_with(&file))
        .map(|(_, t)| t)
        .unwrap_or_else(|| panic!("no {file}"))
}

/// Where the element that starts with `needle` starts in `markup`.
fn at(markup: &str, needle: &str) -> u32 {
    markup.find(needle).unwrap_or_else(|| panic!("no {needle:?} in {markup}")) as u32
}

fn edit(markup: &str, edit: &Edit) -> String {
    apply(None, markup, edit).unwrap_or_else(|e| panic!("edit failed: {e}")).text
}

/// Name them names the menu and the toolbar.
#[test]
fn name_them_names_a_menu_and_a_toolbar() {
    let unnamed = design("module m\n\napp Shop\n    menu\n        item \"Quit\" name quitItem on click -> quit\n    toolbar\n        button \"Quit\" name quitButton on click -> quit\n", "Shop");
    let named = edit(&unnamed, &Edit::NameParts);
    assert!(named.contains("  <Menu name=\"mainMenu\">\n"), "{named}");
    assert!(named.contains("  <Toolbar name=\"mainToolbar\">\n"), "{named}");
}

/// The designer renames a menu and a toolbar.
#[test]
fn the_designer_names_a_menu_and_a_toolbar() {
    let window = design(WINDOW, "Main");
    let text = edit(&window, &Edit::SetName { item: at(&window, "<Menu"), name: "appMenu".into() });
    assert!(text.contains("  <Menu name=\"appMenu\">\n"), "{text}");
    let unnamed = text.replace("<Toolbar name=\"mainToolbar\">", "<Toolbar>");
    let text = edit(&unnamed, &Edit::SetName { item: at(&unnamed, "<Toolbar>"), name: "tools".into() });
    assert!(text.contains("  <Toolbar name=\"tools\">\n"), "{text}");
}

/// A new menu or toolbar comes with its name.
#[test]
fn a_new_menu_or_toolbar_is_named() {
    let bare = design("module m\n\npage Main: window\n    text name mainText at 1,1\n", "Main");
    for (region, line) in [(RegionKind::Menu, "  <Menu name=\"mainMenu\">\n"), (RegionKind::Toolbar, "  <Toolbar name=\"mainToolbar\">\n")] {
        let text = edit(&bare, &Edit::AddRegion { owner: at(&bare, "<Window"), region, side: None });
        assert!(text.contains(line), "{text}");
    }
}
