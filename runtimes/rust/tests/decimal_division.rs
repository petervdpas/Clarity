//! Dividing decimals as every stack divides them (conformance/decimal-division.json):
//! generated Rust divides through `decimal_div`, a value error for dividing by zero.

use std::str::FromStr;

use clarity_runtime::Error;
use rust_decimal::Decimal;
use serde_json::Value;

#[test]
fn decimals_divide_as_every_stack_divides_them() {
    let file: Value = serde_json::from_str(include_str!("../../../conformance/decimal-division.json")).unwrap();
    for case in file["cases"].as_array().unwrap() {
        let (a, b) = (Decimal::from_str(case["a"].as_str().unwrap()).unwrap(), Decimal::from_str(case["b"].as_str().unwrap()).unwrap());
        match clarity_runtime::decimal_div(a, b) {
            Ok(q) => assert_eq!(q.to_string(), case["quotient"].as_str().unwrap_or("an error"), "{a} / {b}"),
            Err(e) => {
                assert!(case["error"].as_bool() == Some(true), "{a} / {b}: {e}");
                assert!(matches!(e, Error::DividedByZero), "{a} / {b}: a value error, not {e}");
            }
        }
    }
}
