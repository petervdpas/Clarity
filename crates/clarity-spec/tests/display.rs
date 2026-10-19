//! The display part: a number shown in seven-segment digits, as on a
//! calculator, rounded to fit the digits it has.

use clarity_spec::parser::parse;
use clarity_spec::widgets::{Category, WidgetKind, info as widget_info, widget_line};
use clarity_spec::{Severity, load_sources};

const CALC: &str = "module calc

page Main: window
    data shown: decimal
    data word: text
    display name screen at 1,1 span 4 rows 2 bind shown digits 10
";

fn errors(source: &str) -> Vec<String> {
    load_sources([("calc.clarity", source)]).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
}

#[test]
fn a_display_shows_a_number_in_its_digits() {
    let (file, diags) = parse(CALC, "calc.clarity");
    assert!(diags.is_empty(), "{diags:#?}");
    let screen = &file.pages[0].widgets[0];
    assert_eq!(screen.props.kind, WidgetKind::Display);
    assert_eq!(screen.props.digits, Some(10));
    assert_eq!(widget_line(&screen.props), "display name screen at 1,1 span 4 rows 2 bind shown digits 10");
    assert!(errors(CALC).is_empty(), "{:#?}", errors(CALC));

    let info = widget_info(WidgetKind::Display);
    assert_eq!((info.keyword.as_str(), info.category), ("display", Category::Graphics));
    assert_eq!(info.field_types, ["integer", "decimal", "money"]);
}

#[test]
fn a_display_holds_1_to_24_digits_of_a_number() {
    let bad = |from: &str, to: &str| errors(&CALC.replace(from, to));
    assert!(bad("digits 10", "digits 30").iter().any(|e| e.contains("a display has 1 to 24 digits")), "{:#?}", bad("digits 10", "digits 30"));
    assert!(bad("bind shown", "bind word").iter().any(|e| e.contains("'word' is text; a display shows integer, decimal, money")));
    assert!(bad("display name screen", "text name screen").iter().any(|e| e.contains("'digits' does not apply to a text")));
}
