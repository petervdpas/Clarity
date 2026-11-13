//! The status item: a footer's (or any grid's) small widget, as a panel of a
//! status bar. It says its caption, with an icon, a small gauge of a number
//! it binds (0 to its max, 100 without one), a check mark while code sets it
//! checked, and a warning while code sets an error on it.

use clarity_spec::keyed::load_keyed;
use clarity_spec::parser::parse;
use clarity_spec::widgets::{self, WidgetKind};
use clarity_spec::{Project, Severity};

const LEDGER: &str = "module ledger

page Main: window
    data done: integer
    data note: text
    statusItem \"Progress\" name progressItem icon \"gauge\" at 1,1 span 3 bind done max 100 on click: showDetails
    statusItem \"Reviewed\" name reviewItem at 4,1 span 3
    statusItem \"Done\" name doneItem at 7,1 span 3 bind done
";

const CODE: &str = "
code Main
    handler showDetails
        progressItem.caption = \"Done\"
        reviewItem.checked = true
        reviewItem.error = \"Changed since review\"
        doneItem.checked = done > 0
";

fn project(page: &str) -> Project {
    let text = format!("{page}{CODE}");
    load_keyed([("ledger.clarity", text.as_str())], &[], "en")
}

fn errors(page: &str) -> Vec<String> {
    project(page).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
}

#[test]
fn a_status_item_reads_and_writes_back_as_written() {
    let (file, diagnostics) = parse(LEDGER, "ledger.clarity");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let parts = &file.pages[0].widgets;
    assert_eq!(parts.iter().map(|w| w.props.kind).collect::<Vec<_>>(), [WidgetKind::StatusItem, WidgetKind::StatusItem, WidgetKind::StatusItem]);
    let lines: Vec<&str> = LEDGER.lines().filter(|l| l.trim_start().starts_with("statusItem")).collect();
    for (widget, line) in parts.iter().zip(lines) {
        assert_eq!(widgets::widget_line(&widget.props), line.trim(), "{line}");
    }
    assert_eq!(parts[0].props.max.as_deref(), Some("100"));
    let info = widgets::info(WidgetKind::StatusItem);
    assert_eq!(info.keyword, "statusItem");
    assert!(info.clickable, "a status item can be clicked");
    assert!(widgets::has_property(WidgetKind::StatusItem, widgets::WidgetProperty::Icon));
}

#[test]
fn a_window_with_status_items_checks_clean_and_code_sets_their_caption_check_and_warning() {
    let p = project(LEDGER);
    let errors: Vec<&str> = p.diagnostics.iter().filter(|d| d.severity == Severity::Error).map(|d| d.message.as_str()).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    let en = &p.translations["en"];
    for text in ["Progress", "Reviewed", "Done", "Changed since review"] {
        assert!(en.values().any(|v| v == text), "{text} is a key: {en:?}");
    }
}

#[test]
fn the_rules_of_a_status_item() {
    let says = |page: String, wanted: &str| {
        let found = errors(&page);
        assert!(found.iter().any(|m| m.contains(wanted)), "{wanted}: {found:#?}");
    };
    // Its gauge shows a number.
    says(LEDGER.replace("bind done", "bind note"), "a statusItem shows integer, decimal");
    says(LEDGER.replace("max 100", "max 0"), "'max' is a number above 0");
}
