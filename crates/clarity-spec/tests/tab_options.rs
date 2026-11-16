//! A tabs part's options, off by default (`closing`, `cycling`, `available`,
//! and `permanent` on a tab): the person closes a tab with its x (a tab
//! marked permanent has none), goes through tabs that do not fit with two
//! sideways triangles, and brings a closed tab back from a dropdown of the
//! closed ones. In the window's file `closing="true" cycling="true"
//! available="true"` and `<Tab permanent="true">`. Code hears a close with
//! `on close`, and brings a tab back with `customerTabs.reopen(2)`, which
//! opens it as `customerTabs.tab = 2` does.

use clarity_spec::code::StmtKind;
use clarity_spec::cxaml::{apply, read_page};
use clarity_spec::edit::Edit;
use clarity_spec::parser::parse;
use clarity_spec::widgets::{self, Event, WidgetKind, WidgetProperty};
use clarity_spec::{Severity, load_converted};

const WINDOW: &str = "module m

page Main: window
    tabs name customerTabs at 1,1 span 8 rows 6 closing cycling available on close: tabClosed
        tab \"Overview\" permanent
            label \"Overview\" name overviewLabel at 1,1 span 4
        tab \"Orders\"
            label \"Orders\" name ordersLabel at 1,1 span 4
        tab \"Notes\"
            label \"Notes\" name notesLabel at 1,1 span 4

code Main
    handler tabClosed
        customerTabs.reopen(2)
";

fn errors(source: &str) -> Vec<String> {
    load_converted([("m.clarity", source)]).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
}

#[test]
fn the_options_read_and_write_back() {
    let (file, diagnostics) = parse(WINDOW, "m.clarity");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let tabs = &file.pages[0].widgets[0];
    assert!(tabs.props.closing && tabs.props.cycling && tabs.props.available);
    assert_eq!(tabs.pages.iter().map(|p| p.permanent).collect::<Vec<_>>(), [true, false, false]);
    assert!(widgets::widget_line(&tabs.props).contains("closing cycling available"), "{}", widgets::widget_line(&tabs.props));
    for property in [WidgetProperty::Closing, WidgetProperty::Cycling, WidgetProperty::Available] {
        assert!(widgets::has_property(WidgetKind::Tabs, property), "{property:?}");
        assert!(!widgets::has_property(WidgetKind::Accordion, property), "only tabs: {property:?}");
    }
    assert!(widgets::has_event(WidgetKind::Tabs, Event::Close));
    let files = clarity_spec::converted([("m.clarity", WINDOW)]);
    let design = files.iter().find(|(p, _)| p.ends_with("Main.cxaml")).map(|(_, t)| t.clone()).unwrap();
    assert!(design.contains("closing=\"true\"") && design.contains("cycling=\"true\"") && design.contains("available=\"true\""), "{design}");
    assert!(design.contains("<Tab title=\"Overview\" permanent=\"true\">"), "{design}");
    assert!(!design.contains("title=\"Orders\" permanent"), "only the permanent tab says so: {design}");
    assert!(errors(WINDOW).is_empty(), "{:#?}", errors(WINDOW));
}

#[test]
fn the_options_are_off_by_default() {
    let plain = WINDOW.replace(" closing cycling available on close: tabClosed", "").replace(" permanent", "").replace("        customerTabs.reopen(2)\n", "        customerTabs.tab = 2\n");
    let (file, _) = parse(&plain, "m.clarity");
    let tabs = &file.pages[0].widgets[0];
    assert!(!tabs.props.closing && !tabs.props.cycling && !tabs.props.available && tabs.pages.iter().all(|p| !p.permanent));
    let files = clarity_spec::converted([("m.clarity", plain.as_str())]);
    let design = files.iter().find(|(p, _)| p.ends_with("Main.cxaml")).map(|(_, t)| t.clone()).unwrap();
    for word in ["closing", "cycling", "available", "permanent"] {
        assert!(!design.contains(word), "{word} is not written when off: {design}");
    }
}

#[test]
fn the_checker_refuses_what_has_no_meaning() {
    let refused = |from: &str, to: &str, says: &str| {
        let source = WINDOW.replace(from, to);
        assert!(errors(&source).iter().any(|m| m.contains(says)), "{to}: {:#?}", errors(&source));
    };
    refused(" closing cycling available", " cycling available", "'available' lists the closed tabs: write 'closing'");
    refused(" closing cycling available", " cycling", "a permanent tab stays among tabs that close: write 'closing' on customerTabs");
    refused("customerTabs.reopen(2)", "customerTabs.reopen()", "customerTabs.reopen takes the tab to bring back, from 1, as in customerTabs.reopen(2)");
    refused("customerTabs.reopen(2)", "customerTabs.shut(2)", "customerTabs is tabs; it brings a closed tab back with customerTabs.reopen(2)");
    let accordion = "module m\n\npage Main: window\n    accordion name faq at 1,1 span 6 rows 6 closing\n        section \"A\"\n";
    let (_, diagnostics) = parse(accordion, "m.clarity");
    assert!(diagnostics.iter().any(|d| d.message.contains("'closing' does not apply to an accordion") || d.message.contains("closing")), "{diagnostics:?}");
}

#[test]
fn reopen_opens_the_tab_as_setting_tab_does() {
    // Every stack runs it as `customerTabs.tab = 2`: opening a closed tab brings it back.
    let project = load_converted([("m.clarity", WINDOW)]);
    let program = clarity_spec::views::placed(&project);
    let handler = program.files.iter().flat_map(|f| &f.code).flat_map(|c| &c.handlers).find(|h| h.name.name == "tabClosed").unwrap();
    match &handler.body[0].kind {
        StmtKind::Assign { target, value } => {
            assert_eq!(format!("{target:?}").contains("customerTabs") && format!("{target:?}").contains("\"tab\""), true, "{target:?}");
            assert!(format!("{value:?}").contains('2'), "{value:?}");
        }
        other => panic!("reopen is not an assignment of tab: {other:?}"),
    }
}

#[test]
fn the_designer_marks_a_tab_permanent_and_back() {
    let main = clarity_spec::converted([("m.clarity", WINDOW)]).into_iter().find(|(p, _)| p.ends_with("Main.cxaml")).map(|(_, t)| t).unwrap();
    let page = read_page(&main, "").0.expect("the window");
    let tabs = &page.widgets[0].pages;
    let set = apply(None, &main, &Edit::SetPagePermanent { target: tabs[1].span.start, permanent: true }).unwrap().text;
    assert!(set.contains("<Tab title=\"Orders\" permanent=\"true\">"), "{set}");
    let cleared = apply(None, &main, &Edit::SetPagePermanent { target: tabs[0].span.start, permanent: false }).unwrap().text;
    assert!(cleared.contains("<Tab title=\"Overview\">"), "{cleared}");
    let accordion = clarity_spec::converted([("m.clarity", "module m\n\npage Main: window\n    accordion name faq at 1,1 span 6 rows 6\n        section \"A\"\n")]).into_iter().find(|(p, _)| p.ends_with("Main.cxaml")).map(|(_, t)| t).unwrap();
    let section = read_page(&accordion, "").0.unwrap().widgets[0].pages[0].span.start;
    assert!(apply(None, &accordion, &Edit::SetPagePermanent { target: section, permanent: true }).is_err(), "only a tab is permanent");
}

#[test]
fn the_dropdown_says_its_words_in_the_apps_languages() {
    // `+ Add a tab` is a word of Clarity.UI's own, asked of an app whose tabs have `available`.
    assert!(clarity_spec::translations::own_words().any(|(k, v)| *k == "tabsAddCaption" && *v == "+ Add a tab"));
    let project = load_converted([("m.clarity", WINDOW)]);
    let asked: Vec<String> = clarity_spec::translations::keys(&project).into_iter().map(|k| k.key).collect();
    assert!(asked.iter().any(|k| k == "tabsAddCaption"), "{asked:?}");
    let plain = load_converted([("m.clarity", WINDOW.replace(" available", "").as_str())]);
    assert!(!clarity_spec::translations::keys(&plain).iter().any(|k| k.key == "tabsAddCaption"));
}
