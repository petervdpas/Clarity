//! `striped`: a part that shows rows (a table, a list, a tree, a tree table)
//! draws every other row with a faint stripe. It is a bare word on the
//! part's line; a part without rows refuses it.

use clarity_spec::highlight::{PieceKind, highlight};
use clarity_spec::keyed::load_keyed;
use clarity_spec::parser::parse;
use clarity_spec::widgets::{self, WidgetKind, WidgetProperty};
use clarity_spec::Severity;

const SHOP: &str = "module shop

entity Order
    number: text(20)
    total: integer
    parent: ref Order
    children: list<Order>

page Main: window
    data orders: list<Order>
";

fn with(part: &str) -> String {
    format!("{SHOP}    {part}\n")
}

fn errors(part: &str) -> Vec<String> {
    let text = with(part);
    load_keyed([("shop.clarity", text.as_str())], &[], "en").diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
}

#[test]
fn a_striped_part_reads_and_writes_back() {
    for line in [
        "table name orders at 1,1 span 12 rows 8 bind orders columns number, total striped",
        "list name orderList at 1,1 span 6 rows 8 bind orders show number striped",
        "tree name orderTree at 1,1 span 4 rows 8 bind orders children children striped",
        "treetable name orderGrid at 1,1 span 12 rows 8 bind orders children children columns number, total striped",
    ] {
        let text = with(line);
        let (file, diagnostics) = parse(&text, "shop.clarity");
        assert!(diagnostics.is_empty(), "{line}: {diagnostics:?}");
        let props = &file.pages[0].widgets[0].props;
        assert!(props.striped, "{line}");
        assert_eq!(widgets::widget_line(props), line);
        assert!(errors(line).is_empty(), "{line}: {:?}", errors(line));
    }
}

#[test]
fn without_striped_a_part_is_written_as_before() {
    let line = "table name orders at 1,1 span 12 rows 8 bind orders columns number, total";
    let text = with(line);
    let (file, _) = parse(&text, "shop.clarity");
    let props = &file.pages[0].widgets[0].props;
    assert!(!props.striped);
    assert_eq!(widgets::widget_line(props), line);
}

#[test]
fn only_parts_with_rows_are_striped() {
    for kind in [WidgetKind::Table, WidgetKind::List, WidgetKind::Tree, WidgetKind::TreeTable] {
        assert!(widgets::has_property(kind, WidgetProperty::Striped), "{kind:?}");
    }
    for kind in [WidgetKind::Button, WidgetKind::Chart, WidgetKind::Dropdown, WidgetKind::Panel] {
        assert!(!widgets::has_property(kind, WidgetProperty::Striped), "{kind:?}");
    }
    let button = errors("button \"Go\" name go at 1,1 span 2 striped");
    assert!(button.iter().any(|m| m.contains("'striped' does not apply to a button")), "{button:?}");
    let mut props = widgets::new_props(WidgetKind::Button, 1, 1);
    props.striped = true;
    assert_eq!(widgets::validate_props(&props), Err("'striped' does not apply to a button".to_string()));
}

#[test]
fn striped_is_a_keyword() {
    let source = with("table name orders at 1,1 span 12 rows 8 bind orders columns number striped");
    let lines: Vec<&str> = source.split('\n').collect();
    let found = highlight(&source).into_iter().any(|p| {
        let line: Vec<u16> = lines[p.line as usize - 1].encode_utf16().collect();
        String::from_utf16_lossy(&line[p.from as usize..p.to as usize]) == "striped" && p.kind == PieceKind::Keyword
    });
    assert!(found);
    assert!(clarity_spec::highlight::keywords().iter().any(|w| w == "striped"));
}
