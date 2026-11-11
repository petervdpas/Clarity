//! The data formats of code end to end on the Rust runtime
//! (conformance/formats-runtime.json): read through Clarity.Formats,
//! decoded by serde into records as the generator writes them, written back.

#![cfg(feature = "formats")]

use clarity_runtime::formats;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;

const CASES: &str = include_str!("../../../conformance/formats-runtime.json");

#[derive(Debug, Default, Serialize, Deserialize)]
struct Line {
    id: i64,
    product: String,
    quantity: i64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Order {
    id: i64,
    number: String,
    total: Decimal,
    placed: Option<chrono::NaiveDate>,
    lines: Vec<Line>,
}

#[test]
fn formats_read_decode_and_write_as_every_stack_does() {
    let cases: Value = serde_json::from_str(CASES).unwrap();
    for case in cases["cases"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let kind = case["kind"].to_string();
        let options = case.get("options").map(Value::to_string).unwrap_or_default();
        let input = case["input"].as_str().unwrap();
        let many = case["kind"]["kind"] == "records";
        let format = case.get("format").and_then(Value::as_str).unwrap_or("json");
        let written = if many {
            formats::read::<Vec<Order>>(format, input, &options, &kind).map(|orders| formats::write(format, &orders, &options, &kind))
        } else {
            formats::read::<Order>(format, input, &options, &kind).map(|order| formats::write(format, &order, &options, &kind))
        };
        match (case.get("written"), case.get("error")) {
            (Some(wanted), _) => assert_eq!(written.unwrap_or_else(|e| panic!("{name}: {e}")), wanted.as_str().unwrap(), "{name}"),
            (_, Some(error)) => assert_eq!(written.unwrap_err().to_string(), error.as_str().unwrap(), "{name}"),
            _ => panic!("{name}: a case has written or error"),
        }
    }
}
