//! A date compared with an empty one, as every stack compares them
//! (conformance/compare-empty.json): generated Rust compares the fields'
//! `Option`s, where an empty value comes first.

use chrono::{NaiveDate, NaiveDateTime};
use serde_json::Value;

fn ordered<T: PartialOrd>(a: Option<T>, b: Option<T>) -> [bool; 4] {
    [a < b, a <= b, a > b, a >= b]
}

#[test]
fn an_empty_date_is_smallest_as_every_stack_compares_it() {
    let file: Value = serde_json::from_str(include_str!("../../../conformance/compare-empty.json")).unwrap();
    for case in file["cases"].as_array().unwrap() {
        let got = match case["kind"].as_str().unwrap() {
            "date" => ordered(case["a"].as_str().map(|t| NaiveDate::parse_from_str(t, "%Y-%m-%d").unwrap()), case["b"].as_str().map(|t| NaiveDate::parse_from_str(t, "%Y-%m-%d").unwrap())),
            _ => ordered(case["a"].as_str().map(|t| NaiveDateTime::parse_from_str(t, "%Y-%m-%dT%H:%M:%S").unwrap()), case["b"].as_str().map(|t| NaiveDateTime::parse_from_str(t, "%Y-%m-%dT%H:%M:%S").unwrap())),
        };
        let want = ["<", "<=", ">", ">="].map(|op| case[op].as_bool().unwrap());
        assert_eq!(got, want, "{case}");
    }
}
