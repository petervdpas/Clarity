//! Lists of values as fields (U1c), kept in one column as JSON, against
//! the cases every runtime shares (conformance/list-fields.json).

use clarity_runtime::sql::{self, SqlValue};
use clarity_runtime::{Decimal, NaiveDate};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

const CASES: &str = include_str!("../../../conformance/list-fields.json");

fn cases(part: &str) -> Vec<Value> {
    let all: Value = serde_json::from_str(CASES).expect("list-fields.json is JSON");
    all[part].as_array().unwrap().clone()
}

fn column(case: &Value) -> SqlValue {
    case["column"].as_str().map_or(SqlValue::Null, |s| SqlValue::Text(s.to_string()))
}

/// Reads the column as the item type.
fn read<T: Serialize + DeserializeOwned>(case: &Value) {
    let items: Vec<T> = sql::get_list("tags", &column(case)).unwrap();
    assert_eq!(serde_json::to_value(&items).unwrap(), case["items"], "{case}");
}

fn kept<T: Serialize + DeserializeOwned>(case: &Value) {
    let typed: Vec<T> = serde_json::from_value(case["items"].clone()).unwrap();
    assert_eq!(sql::list(&typed), SqlValue::Text(case["column"].as_str().unwrap().to_string()), "{case}");
}

fn each(case: &Value, run: [fn(&Value); 4]) {
    match case["item"].as_str().unwrap() {
        "text" | "enum" => run[0](case),
        "integer" => run[1](case),
        "decimal" => run[2](case),
        "date" => run[3](case),
        other => panic!("no item {other}"),
    }
}

#[test]
fn a_list_is_kept_as_compact_json() {
    for case in cases("kept") {
        each(&case, [kept::<String>, kept::<i64>, kept::<Decimal>, kept::<Option<NaiveDate>>]);
    }
}

#[test]
fn a_column_gives_its_items() {
    for case in cases("read") {
        each(&case, [read::<String>, read::<i64>, read::<Decimal>, read::<Option<NaiveDate>>]);
    }
}
