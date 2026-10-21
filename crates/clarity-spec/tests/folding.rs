//! A panel or form the person using the application folds and unfolds:
//! `foldable`, and `foldable folded` to start folded. Code reads and sets
//! `part.folded`.

use clarity_spec::parser::parse;
use clarity_spec::widgets::widget_line;
use clarity_spec::{Severity, load_sources};

const WINDOW: &str = "module m

entity Customer
    name: text(40)

page Main: window
    data customer: Customer
    panel \"Details\" name details at 1,1 span 6 rows 4 foldable folded
        text name nameText at 1,1 span 4 bind customer.name
    form name customerForm at 7,1 span 6 rows 4 bind customer foldable
        textbox name nameInput at 1,1 span 4 bind customer.name
    button \"More\" name more at 1,5 span 2 on click: moreClick

code Main
    handler moreClick
        if details.folded
            details.folded = false
        customerForm.folded = not details.folded
";

fn errors(source: &str) -> Vec<String> {
    load_sources([("m.clarity", source)]).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
}

fn assert_has(messages: &[String], needle: &str) {
    assert!(messages.iter().any(|m| m.contains(needle)), "missing {needle:?} in {messages:#?}");
}

#[test]
fn a_panel_or_form_can_fold() {
    let (file, diags) = parse(WINDOW, "m.clarity");
    assert!(diags.is_empty(), "{diags:#?}");
    let details = &file.pages[0].widgets[0].props;
    assert!(details.foldable && details.folded);
    let form = &file.pages[0].widgets[1].props;
    assert!(form.foldable && !form.folded);
    // Written back as it was read.
    assert_eq!(widget_line(details), "panel \"Details\" name details at 1,1 span 6 rows 4 foldable folded");
    assert!(errors(WINDOW).is_empty(), "{:#?}", errors(WINDOW));
}

#[test]
fn only_a_foldable_panel_or_form_folds() {
    assert_has(&errors(&WINDOW.replace("bind customer.name\n    form", "bind customer.name\n    form").replace("span 4 bind customer.name\n    button", "span 4 bind customer.name foldable\n    button")), "'foldable' does not apply to a textbox");
    assert_has(&errors(&WINDOW.replace("rows 4 foldable folded", "rows 4 folded")), "'folded' needs 'foldable'");
    let fixed = WINDOW.replace("rows 4 bind customer foldable", "rows 4 bind customer");
    assert_has(&errors(&fixed), "form 'customerForm' has no property 'folded'");
}
