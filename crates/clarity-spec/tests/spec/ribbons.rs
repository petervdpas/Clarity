//! A ribbon: a named part of a window's (or the app's) frame, with named tabs
//! and groups. The person folds a collapsible ribbon to its tabs and opens a
//! tab to unfold it; code reads and sets `collapsed`, the open `tab` (from
//! 1), and hides tabs and groups by their names.

use clarity_spec::cxaml::apply;
use clarity_spec::edit::Edit;
use clarity_spec::parser::parse;
use clarity_spec::{Severity, load_converted};

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
    load_converted([("m.clarity", source)]).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
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

/// Name them names the ribbon, its tabs and its groups after what they are.
#[test]
fn name_them_names_a_ribbon_its_tabs_and_groups() {
    let unnamed = design("module m\n\napp Shop\n    ribbon\n        tab \"Home\"\n            group \"Clipboard Tools\"\n                button \"Paste\" name pasteButton\n", "Shop");
    let named = edit(&unnamed, &Edit::NameParts);
    assert!(named.contains("  <Ribbon name=\"mainRibbon\">\n    <Tab title=\"Home\" name=\"homeTab\">\n      <Group title=\"Clipboard Tools\" name=\"clipboardToolsGroup\">\n"), "{named}");
}

/// The designer sets a ribbon's name and folding, and a tab's or group's name.
#[test]
fn the_designer_sets_a_ribbons_options_and_its_parts_names() {
    let window = design(WINDOW, "Main");
    let text = edit(&window, &Edit::UpdateRibbon { item: at(&window, "<Ribbon"), name: Some("tools".into()), collapsible: true, collapsed: false });
    assert!(text.contains("  <Ribbon name=\"tools\" collapsible=\"true\">\n"), "{text}");
    let text = edit(&text, &Edit::SetName { item: at(&text, "<Tab title=\"View\""), name: "showTab".into() });
    assert!(text.contains("    <Tab title=\"View\" name=\"showTab\">\n"), "{text}");
    let unnamed = text.replace("<Group title=\"Zoom\" name=\"zoomGroup\">", "<Group title=\"Zoom\">");
    let text = edit(&unnamed, &Edit::SetName { item: at(&unnamed, "<Group title=\"Zoom\""), name: "sizeGroup".into() });
    assert!(text.contains("      <Group title=\"Zoom\" name=\"sizeGroup\">\n"), "{text}");
}
