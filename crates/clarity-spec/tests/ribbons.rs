//! A ribbon: a named part of a window's (or the app's) frame, with named tabs
//! and groups. The person folds a collapsible ribbon to its tabs and opens a
//! tab to unfold it; code reads and sets `collapsed`, the open `tab` (from
//! 1), and hides tabs and groups by their names.

use clarity_spec::edit::{Edit, apply};
use clarity_spec::parser::parse;
use clarity_spec::{Severity, load_sources};

const WINDOW: &str = "module m

page Main: window
    ribbon name mainRibbon collapsible collapsed
        tab \"Home\" name homeTab
            group \"Clipboard\" name clipboardGroup
                button \"Paste\" name pasteButton large on click: pasteClicked
        tab \"View\" name viewTab
            group \"Zoom\" name zoomGroup
                button \"Zoom in\" name zoomInButton on click: pasteClicked
    text name mainText at 1,1 span 6

code Main
    handler pasteClicked
        mainRibbon.collapsed = false
        if mainRibbon.tab == 1
            mainRibbon.tab = 2
        clipboardGroup.visible = false
        viewTab.visible = not viewTab.visible
";

fn errors(source: &str) -> Vec<String> {
    load_sources([("m.clarity", source)]).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
}

fn assert_has(messages: &[String], needle: &str) {
    assert!(messages.iter().any(|m| m.contains(needle)), "missing {needle:?} in {messages:#?}");
}

#[test]
fn a_ribbon_has_a_name_tabs_and_groups_and_can_fold() {
    let (file, diags) = parse(WINDOW, "m.clarity");
    assert!(diags.is_empty(), "{diags:#?}");
    let ribbon = file.pages[0].frame.ribbon.as_ref().unwrap();
    assert_eq!(ribbon.name.as_deref(), Some("mainRibbon"));
    assert!(ribbon.collapsible && ribbon.collapsed);
    assert_eq!(ribbon.tabs[0].name.as_deref(), Some("homeTab"));
    assert_eq!(ribbon.tabs[0].groups[0].name.as_deref(), Some("clipboardGroup"));
    assert!(errors(WINDOW).is_empty(), "{:#?}", errors(WINDOW));
}

#[test]
fn a_ribbon_is_checked() {
    assert_has(&errors(&WINDOW.replace("ribbon name mainRibbon collapsible", "ribbon collapsible")), "the ribbon has no name; every part has one");
    assert_has(&errors(&WINDOW.replace("tab \"View\" name viewTab", "tab \"View\"")), "tab \"View\" has no name; every part has one");
    assert_has(&errors(&WINDOW.replace("group \"Zoom\" name zoomGroup", "group \"Zoom\"")), "group \"Zoom\" has no name; every part has one");
    assert_has(&errors(&WINDOW.replace("name zoomGroup", "name zoom_group")), "part 'zoom_group' is not camelCase");
    assert_has(&errors(&WINDOW.replace("collapsible collapsed", "collapsed")), "the ribbon: 'collapsed' needs 'collapsible'");
    let fixed = WINDOW.replace("collapsible collapsed", "");
    assert_has(&errors(&fixed), "ribbon 'mainRibbon' has no property 'collapsed'");
    assert_has(&errors(&WINDOW.replace("mainRibbon.tab = 2", "mainRibbon.tab = \"View\"")), "integer");
}

/// Name them names the ribbon, its tabs and its groups after what they are.
#[test]
fn name_them_names_a_ribbon_its_tabs_and_groups() {
    let unnamed = "module m\n\napp Shop\n    ribbon\n        tab \"Home\"\n            group \"Clipboard Tools\"\n                button \"Paste\" name pasteButton\n";
    let named = apply(unnamed, &Edit::NameParts).unwrap().text;
    assert!(named.contains("    ribbon name mainRibbon\n        tab \"Home\" name homeTab\n            group \"Clipboard Tools\" name clipboardToolsGroup\n"), "{named}");
}

/// The designer sets a ribbon's name and folding, and a tab's or group's name.
#[test]
fn the_designer_sets_a_ribbons_options_and_its_parts_names() {
    let at = |source: &str, needle: &str| source.find(needle).unwrap() as u32;
    let text = apply(WINDOW, &Edit::UpdateRibbon { item: at(WINDOW, "ribbon name"), name: Some("tools".into()), collapsible: true, collapsed: false }).unwrap().text;
    assert!(text.contains("    ribbon name tools collapsible\n"), "{text}");
    let text = apply(&text, &Edit::SetName { item: at(&text, "tab \"View\""), name: "showTab".into() }).unwrap().text;
    assert!(text.contains("        tab \"View\" name showTab\n"), "{text}");
    let unnamed = text.replace("group \"Zoom\" name zoomGroup", "group \"Zoom\"");
    let text = apply(&unnamed, &Edit::SetName { item: at(&unnamed, "group \"Zoom\""), name: "sizeGroup".into() }).unwrap().text;
    assert!(text.contains("            group \"Zoom\" name sizeGroup\n"), "{text}");
}
