//! The rows tables, trees and charts show, which Clarity.UI works out with
//! these modules (manual/start.how-it-works.md), against their cases
//! (conformance/table-order.json, table-pages.json, trees.json, charts.json `groups`).

use clarity_runtime::charts::{self, ChartRow};
use clarity_runtime::reorder::Row;
use clarity_runtime::tables::{self, TableColumn, TableOrder};
use clarity_runtime::trees;
use serde_json::{Map, Value, json};

fn file(text: &str) -> Value {
    serde_json::from_str(text).unwrap()
}

#[test]
fn a_table_shows_its_rows_in_its_order() {
    let all = file(include_str!("../../../conformance/table-order.json"));
    let columns: Vec<TableColumn> = serde_json::from_value(all["columns"].clone()).unwrap();
    let rows: Vec<Map<String, Value>> = serde_json::from_value(all["rows"].clone()).unwrap();
    for case in all["cases"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let mut order: Option<TableOrder> = serde_json::from_value(case["order"].clone()).unwrap();
        let (mut events, mut chosen) = (vec![], None);
        for step in case["steps"].as_array().unwrap() {
            match step[0].as_str().unwrap() {
                "sort" if case["sortable"] == true => order = Some(tables::next_order(order.as_ref(), step[1].as_str().unwrap())),
                "sort" => {}
                _ => {
                    let place = tables::shown_order(&rows, &columns, order.as_ref())[step[1].as_u64().unwrap() as usize];
                    events.push(format!("select {place}"));
                    chosen = Some(place);
                }
            }
        }
        let expect = &case["expect"];
        assert_eq!(json!(tables::shown_order(&rows, &columns, order.as_ref())), expect["shown"], "{name}");
        assert_eq!(json!(events), expect["events"], "{name}: events");
        if let Some(c) = expect.get("chosen") {
            assert_eq!(json!(chosen), *c, "{name}: chosen");
        }
    }
}

#[test]
fn a_table_shows_a_page_at_a_time() {
    let all = file(include_str!("../../../conformance/table-pages.json"));
    for case in all["pages"].as_array().unwrap() {
        let n = |name: &str| case[name].as_i64().unwrap();
        let paged = tables::page_of(n("count") as usize, n("size") as usize, n("page"));
        assert_eq!((paged.page as i64, paged.pages as i64, paged.from as i64, paged.to as i64), (n("shows"), n("pages"), n("from"), n("to")), "{case}");
        assert_eq!(paged.text(), case["text"].as_str().unwrap(), "{case}");
    }
    for case in all["selected"].as_array().unwrap() {
        let page = tables::page_with(case["size"].as_u64().unwrap() as usize, case["at"].as_u64().unwrap() as usize);
        assert_eq!(json!(page), case["page"], "{case}");
    }
}

fn tree_rows(case: &Value) -> Vec<Row> {
    case["rows"].as_array().unwrap().iter().map(|r| Row { id: r["id"].clone(), parent: r["parent"].clone() }).collect()
}

#[test]
fn a_tree_shows_its_open_rows() {
    let all = file(include_str!("../../../conformance/trees.json"));
    for case in all["cases"].as_array().unwrap() {
        let shown = trees::shown_tree(&tree_rows(case), case["open"].as_array().unwrap());
        assert_eq!(serde_json::to_value(&shown).unwrap(), case["shows"], "{}", case["name"]);
    }
    for case in all["reveal"].as_array().unwrap() {
        let opens = trees::above(&tree_rows(case), case["at"].as_u64().unwrap() as usize);
        assert_eq!(json!(opens), case["opens"], "{case}");
    }
}

#[test]
fn a_chart_groups_its_rows() {
    let all = file(include_str!("../../../conformance/charts.json"));
    for case in all["groups"].as_array().unwrap() {
        let rows: Vec<ChartRow> = serde_json::from_value(case["rows"].clone()).unwrap();
        let order = case["order"].as_array().cloned().unwrap_or_default();
        let groups = charts::groups(&rows, case["by"].as_str().unwrap(), case["measure"].as_str().unwrap(), &order);
        let gives = case["gives"].as_array().unwrap();
        assert_eq!(groups.len(), gives.len(), "{}", case["name"]);
        for (g, want) in groups.iter().zip(gives) {
            assert_eq!(g.key, want["key"], "{}", case["name"]);
            assert_eq!(g.value, want["value"].as_f64().unwrap(), "{}", case["name"]);
        }
    }
}
