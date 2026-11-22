//! A sidebar: a named part docked to the left or right edge of a window (or
//! of every window, in the app), with its own grid of parts. The person can
//! resize it between its min and max, fold it to a button and back, and let
//! it fill the window; code reads and sets `collapsed` and `maximized`.

use clarity_spec::cxaml::apply;
use clarity_spec::edit::Edit;
use clarity_spec::frame::sidebar_line;
use clarity_spec::model::Side;
use clarity_spec::parser::parse;
use clarity_spec::{Severity, load_converted};

const WINDOW: &str = "module m

page Main: window
    data shown: text
    sidebar left name nav width 280 min 200 max 480 resizable collapsible collapsed maximizable grid 4 columns
        text name navText at 1,1 span 4 bind shown
    text name mainText at 1,1 span 6 bind shown
    button \"Menu\" name menuButton at 7,1 span 2 on click: menuClick

code Main
    handler menuClick
        nav.collapsed = not nav.collapsed
        if nav.maximized
            nav.maximized = false
";

fn errors(source: &str) -> Vec<String> {
    load_converted([("m.clarity", source)]).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
}

fn assert_has(messages: &[String], needle: &str) {
    assert!(messages.iter().any(|m| m.contains(needle)), "missing {needle:?} in {messages:#?}");
}

#[test]
fn a_sidebar_has_a_width_its_limits_and_what_the_person_can_do() {
    let (file, diags) = parse(WINDOW, "m.clarity");
    assert!(diags.is_empty(), "{diags:#?}");
    let nav = &file.pages[0].frame.sidebars[0];
    let p = &nav.props;
    assert_eq!((p.side, p.name.as_deref(), p.width, p.min, p.max), (Side::Left, Some("nav"), 280, Some(200), Some(480)));
    assert!(p.resizable && p.collapsible && p.collapsed && p.maximizable);
    assert_eq!(p.columns, 4);
    assert_eq!(nav.widgets.len(), 1);
    // Written back as it was read.
    assert_eq!(sidebar_line(p), "sidebar left name nav width 280 min 200 max 480 resizable collapsible collapsed maximizable grid 4 columns");
    assert!(errors(WINDOW).is_empty(), "{:#?}", errors(WINDOW));
}

#[test]
fn a_plain_sidebar_takes_the_defaults() {
    let (file, diags) = parse("module m\n\npage Main: window\n    sidebar right name info\n", "m.clarity");
    assert!(diags.is_empty(), "{diags:#?}");
    let p = &file.pages[0].frame.sidebars[0].props;
    assert_eq!((p.width, p.columns, p.min, p.max), (240, 3, None, None));
    assert!(!p.resizable && !p.collapsible && !p.collapsed && !p.maximizable);
    assert_eq!(sidebar_line(p), "sidebar right name info");
}

#[test]
fn a_sidebar_is_checked() {
    assert_has(&errors(&WINDOW.replace("sidebar left name nav width", "sidebar left width")), "sidebar left has no name; every part has one");
    assert_has(&errors(&WINDOW.replace("name nav width", "name side_nav width")), "part 'side_nav' is not camelCase");
    assert_has(&errors(&WINDOW.replace("width 280 min 200", "width 280 min 300")), "sidebar nav: min 300 is more than its width 280");
    assert_has(&errors(&WINDOW.replace("width 280 min 200 max 480", "width 280 min 200 max 250")), "sidebar nav: max 250 is less than its width 280");
    assert_has(&errors(&WINDOW.replace("max 480 resizable", "max 480")), "sidebar nav: 'min' and 'max' are for a resizable sidebar");
    assert_has(&errors(&WINDOW.replace("resizable collapsible collapsed", "resizable collapsed")), "'collapsed' needs 'collapsible'");
    assert_has(&errors(&WINDOW.replace("width 280", "width 20")), "a sidebar is 100 to 1600 pixels wide");
    assert_has(&errors(&WINDOW.replace("maximizable grid", "grid").replace("        if nav.maximized\n            nav.maximized = false\n", "        nav.maximized = true\n")), "sidebar 'nav' has no property 'maximized'");
}

/// The old sidebar, `span 3` columns wide, is now pixels: an error. Read,
/// it is that many pixels wide already, without a name; Update the
/// sidebars names it in the window's `.cxaml`.
#[test]
fn an_old_sidebar_is_written_anew() {
    let old = "module m\n\npage Main: window\n    sidebar left span 3\n        text name navText at 1,1 span 3\n";
    assert_has(&errors(old), "a sidebar's width is in pixels now; Update the sidebars, in Problems, writes it so");
    let (file, _) = parse(old, "m.clarity");
    let read = &file.pages[0].frame.sidebars[0].props;
    assert_eq!((read.width, read.columns, read.name.as_deref()), (192, 3, None));
    // The window as its file holds it, the sidebar unnamed.
    let markup = clarity_spec::cxaml::page_source(&file.pages[0], "m");
    assert!(markup.contains("  <Sidebar side=\"left\" width=\"192\">\n"), "{markup}");
    let new = apply(None, &markup, &Edit::UpdateSidebars).unwrap().text;
    // Three columns is the default grid, so the element leaves it out.
    assert!(new.contains("  <Sidebar side=\"left\" name=\"leftSidebar\" width=\"192\">\n    <Text name=\"navText\""), "{new}");
    let project = clarity_spec::load_sources([("m/m.clarity", "module m\n"), ("m/design/Main.cxaml", new.as_str())]);
    let errors: Vec<&str> = project.diagnostics.iter().filter(|d| d.severity == Severity::Error).map(|d| d.message.as_str()).collect();
    assert!(errors.is_empty(), "{errors:#?}");
}
