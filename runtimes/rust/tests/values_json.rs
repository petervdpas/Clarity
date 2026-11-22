//! A value written into JSON as every stack writes it for parts, reports, the
//! wire and `json of` (conformance/values-json.json): generated Rust records
//! write their fields with serde, in the runtime's types.

use std::str::FromStr;

use chrono::{NaiveDate, NaiveDateTime};
use rust_decimal::Decimal;
use serde_json::Value;

/// The JSON a generated record writes for a field of `kind` holding `value`.
fn written(kind: &str, value: &Value) -> String {
    let text = || value.as_str().unwrap_or_default().to_string();
    match kind {
        "decimal" | "money" => serde_json::to_string(&Decimal::from_str(&text()).unwrap()),
        "integer" => serde_json::to_string(&i64::from_str(&text()).unwrap()),
        "date" => serde_json::to_string(&value.as_str().map(|t| NaiveDate::parse_from_str(t, "%Y-%m-%d").unwrap())),
        "datetime" => serde_json::to_string(&value.as_str().map(|t| NaiveDateTime::parse_from_str(t, "%Y-%m-%dT%H:%M:%S").unwrap())),
        "ref" => serde_json::to_string(&value.as_str().map(|t| i64::from_str(t).unwrap())),
        "bool" => serde_json::to_string(&value.as_bool().unwrap()),
        "text" => serde_json::to_string(&text()),
        "list" => serde_json::to_string(&Vec::<i64>::new()),
        other => panic!("no kind {other}"),
    }
    .unwrap()
}

#[test]
fn every_value_is_written_as_every_stack_writes_it() {
    let file: Value = serde_json::from_str(include_str!("../../../conformance/values-json.json")).unwrap();
    for case in file["cases"].as_array().unwrap() {
        assert_eq!(written(case["kind"].as_str().unwrap(), &case["value"]), case["json"].as_str().unwrap(), "{}", case["name"]);
    }
}
