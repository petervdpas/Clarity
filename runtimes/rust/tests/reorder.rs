//! Reordering (manual/parts.data.md) against its cases (conformance/reorder.json),
//! as Clarity.UI moves a row with it.

use clarity_runtime::reorder::{self, Move, Row};
use serde_json::{Value, json};

const CASES: &str = include_str!("../../../conformance/reorder.json");

fn rows(case: &Value) -> Vec<Row> {
    case["rows"].as_array().unwrap().iter().map(|r| Row { id: r["id"].clone(), parent: r.get("parent").cloned().unwrap_or(Value::Null) }).collect()
}

fn check(part: &str, tree: bool) {
    let all: Value = serde_json::from_str(CASES).unwrap();
    for case in all[part].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let mv: Move = serde_json::from_value(case["move"].clone()).unwrap_or_else(|e| panic!("{name}: {e}"));
        let moved = reorder::reorder(&rows(case), &mv, tree);
        if case.get("refused").is_some() {
            assert!(moved.is_none(), "{name}: not refused");
            continue;
        }
        let moved = moved.unwrap_or_else(|| panic!("{name}: refused"));
        assert_eq!(json!(moved.order), case["order"], "{name}: order");
        assert_eq!(json!(moved.numbers), case["numbers"], "{name}: numbers");
        if tree {
            assert_eq!(json!(moved.parents), case["parents"], "{name}: parents");
        }
    }
}

#[test]
fn a_list_reorders_as_every_runtime_does() {
    check("lists", false);
}

#[test]
fn a_tree_reorders_as_every_runtime_does() {
    check("trees", true);
}
