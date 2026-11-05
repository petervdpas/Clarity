//! A table's own Add and trash buttons: `add` and `remove` on its line, run
//! by `on add` and `on remove`, with Clarity's words `addRow` and `removeRow`.

use clarity_spec::keyed::load_keyed;
use clarity_spec::parser::parse;
use clarity_spec::Severity;

const PAGE: &str = "module m

entity Line
    name: text(40)

page Lines: window
    data lines: list<Line>
    table name lineTable at 1,1 span 12 rows 6 bind lines columns name edit name add remove on add: added on remove: removed

code Lines
    handler added
        let line = new Line
        save line

    handler removed
        delete lineTable.selected
";

fn errors(text: &str) -> Vec<String> {
    load_keyed([("m.clarity", text)], &[], "en").diagnostics.into_iter().filter(|d| d.severity == Severity::Error && !d.message.contains("has no value")).map(|d| d.message).collect()
}

#[test]
fn a_table_has_its_own_add_and_trash_buttons() {
    assert!(errors(PAGE).is_empty(), "{:#?}", errors(PAGE));
    let (file, _) = parse(PAGE, "m.clarity");
    let p = &file.pages[0].widgets[0].props;
    assert!(p.add && p.remove);
    assert!(clarity_spec::widgets::widget_line(p).contains(" add remove"), "{}", clarity_spec::widgets::widget_line(p));
    let icons = clarity_spec::icons::used(&load_keyed([("m.clarity", PAGE)], &[], "en"));
    assert!(icons.contains_key("plus") && icons.contains_key("trash"));
}

#[test]
fn each_needs_its_handler_and_clarity_has_their_words() {
    assert!(errors(&PAGE.replace(" on add: added", "")).iter().any(|m| m.contains("'add' shows an Add button")));
    assert!(errors(&PAGE.replace(" on remove: removed", "")).iter().any(|m| m.contains("'remove' shows a trash button")));
    assert_eq!(clarity_spec::translations::TABLE_ROW_KEYS.map(|k| k.0), ["addRow", "removeRow", "removeRowAsk", "keepRow"]);
    // `remove confirm` asks first.
    let (file, _) = parse(&PAGE.replace(" add remove ", " add remove confirm "), "m.clarity");
    assert!(file.pages[0].widgets[0].props.confirm);
    assert!(clarity_spec::widgets::widget_line(&file.pages[0].widgets[0].props).contains(" remove confirm"));
}
