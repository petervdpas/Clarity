//! A menu and a toolbar: named parts of a window's (or the app's) frame.
//! Code hides and disables them by their names, as it does any part.

use clarity_spec::edit::{Edit, apply};
use clarity_spec::frame::RegionKind;
use clarity_spec::parser::parse;
use clarity_spec::{Severity, load_sources};

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
    load_sources([("m.clarity", source)]).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
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

/// Name them names the menu and the toolbar.
#[test]
fn name_them_names_a_menu_and_a_toolbar() {
    let unnamed = "module m\n\napp Shop\n    menu\n        item \"Quit\" name quitItem on click -> quit\n    toolbar\n        button \"Quit\" name quitButton on click -> quit\n";
    let named = apply(unnamed, &Edit::NameParts).unwrap().text;
    assert!(named.contains("    menu name mainMenu\n"), "{named}");
    assert!(named.contains("    toolbar name mainToolbar\n"), "{named}");
}

/// The designer renames a menu and a toolbar.
#[test]
fn the_designer_names_a_menu_and_a_toolbar() {
    let at = |source: &str, needle: &str| source.find(needle).unwrap() as u32;
    let text = apply(WINDOW, &Edit::SetName { item: at(WINDOW, "menu name"), name: "appMenu".into() }).unwrap().text;
    assert!(text.contains("    menu name appMenu\n"), "{text}");
    let unnamed = text.replace("toolbar name mainToolbar", "toolbar");
    let text = apply(&unnamed, &Edit::SetName { item: at(&unnamed, "toolbar\n"), name: "tools".into() }).unwrap().text;
    assert!(text.contains("    toolbar name tools\n"), "{text}");
}

/// A new menu or toolbar comes with its name.
#[test]
fn a_new_menu_or_toolbar_is_named() {
    let bare = "module m\n\npage Main: window\n    text name mainText at 1,1\n";
    let page = bare.find("page Main").unwrap() as u32;
    for (region, line) in [(RegionKind::Menu, "    menu name mainMenu\n"), (RegionKind::Toolbar, "    toolbar name mainToolbar\n")] {
        let text = apply(bare, &Edit::AddRegion { owner: page, region, side: None }).unwrap().text;
        assert!(text.contains(line), "{text}");
    }
}
