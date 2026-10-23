//! Charts of U1c: `chart bar`, `chart line` and `chart pie`, bound to a list
//! the window holds, its rows grouped `by` a field and each group counted
//! (`count`) or summed or averaged over a number field (`sum total`,
//! `average total`).

use clarity_spec::model::{ChartKind, ChartMeasure};
use clarity_spec::parser::parse;
use clarity_spec::widgets::{self, WidgetKind};
use clarity_spec::{Severity, load_sources};

const SHOP: &str = "module shop

enum Status
    Open
    Paid

entity Order
    number: text(20)
    status: Status
    placed: date
    total: money
    lines: integer
    paid: bool
    photo: image

page Dashboard: window
    data orders: list<Order>
    data order: Order
    chart bar name byStatus at 1,1 span 6 rows 6 bind orders by status count
    chart line name perDay at 7,1 span 6 rows 6 bind orders by placed sum total
    chart pie name share at 1,7 span 4 rows 6 bind orders by status average lines
";

fn errors(page: &str) -> Vec<String> {
    load_sources([("shop.clarity", page)]).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
}

#[test]
fn charts_read_and_write_back() {
    let (file, diagnostics) = parse(SHOP, "shop.clarity");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let parts = &file.pages[0].widgets;
    assert_eq!(parts[0].props.kind, WidgetKind::Chart);
    assert_eq!(parts[0].props.chart, Some(ChartKind::Bar));
    assert_eq!(parts[0].props.by.as_deref(), Some("status"));
    assert_eq!(parts[0].props.measure, Some(ChartMeasure::Count));
    assert_eq!(parts[1].props.measure, Some(ChartMeasure::Sum { field: "total".into() }));
    assert_eq!(parts[2].props.chart, Some(ChartKind::Pie));
    assert_eq!(parts[2].props.measure, Some(ChartMeasure::Average { field: "lines".into() }));
    assert_eq!(widgets::widget_line(&parts[0].props), "chart bar name byStatus at 1,1 span 6 rows 6 bind orders by status count");
    assert_eq!(widgets::widget_line(&parts[1].props), "chart line name perDay at 7,1 span 6 rows 6 bind orders by placed sum total");
    assert_eq!(widgets::widget_line(&parts[2].props), "chart pie name share at 1,7 span 4 rows 6 bind orders by status average lines");
    let found = errors(SHOP);
    assert!(found.is_empty(), "{found:#?}");
}

#[test]
fn the_rules_of_a_chart() {
    let says = |page: String, wanted: &str| {
        let found = errors(&page);
        assert!(found.iter().any(|m| m.contains(wanted)), "{wanted}: {found:#?}");
    };
    says(SHOP.replace("chart bar name byStatus", "chart radar name byStatus"), "a chart is bar, line or pie, as in 'chart bar'");
    says(SHOP.replace("bind orders by status count", "bind orders count"), "a chart groups its rows by a field, as in 'by status'");
    says(SHOP.replace("bind orders by status count", "bind orders by status"), "a chart counts its rows or sums a number, as in 'count' or 'sum total'");
    says(SHOP.replace("by status count", "by colour count"), "Order has no field 'colour'");
    says(SHOP.replace("by status count", "by photo count"), "a chart cannot group by 'photo'");
    says(SHOP.replace("by placed sum total", "by placed sum number"), "'number' is text; a chart sums or averages a number");
    says(SHOP.replace("by placed sum total", "by placed sum amount"), "Order has no field 'amount'");
}

#[test]
fn a_new_chart_is_bars_that_count_and_writes_back() {
    let props = widgets::new_props(WidgetKind::Chart, 1, 1);
    assert_eq!(props.chart, Some(ChartKind::Bar));
    assert_eq!(props.measure, Some(ChartMeasure::Count));
    assert_eq!(widgets::widget_line(&props), "chart bar at 1,1 span 6 rows 6 count");
}
