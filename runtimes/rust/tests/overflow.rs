//! Whole numbers and decimals that do not fit, as every stack treats them
//! (conformance/overflow.json): generated Rust works them out through
//! `whole_add`, `decimal_add` and the others, a value error, never a wrapped number.

use std::str::FromStr;

use clarity_runtime::{Error, Result};
use rust_decimal::Decimal;
use serde_json::Value;

fn whole(op: &str, a: i64, b: i64) -> Result<String> {
    let n = match op {
        "+" => clarity_runtime::whole_add(a, b),
        "-" => clarity_runtime::whole_sub(a, b),
        "*" => clarity_runtime::whole_mul(a, b),
        "div" => clarity_runtime::whole_div(a, b),
        "mod" => clarity_runtime::whole_mod(a, b),
        _ => clarity_runtime::whole_neg(a),
    }?;
    Ok(n.to_string())
}

fn decimal(op: &str, a: Decimal, b: Decimal) -> Result<String> {
    let d = match op {
        "+" => clarity_runtime::decimal_add(a, b),
        "-" => clarity_runtime::decimal_sub(a, b),
        "*" => clarity_runtime::decimal_mul(a, b),
        _ => Ok(-a),
    }?;
    Ok(d.to_string())
}

#[test]
fn numbers_that_do_not_fit_are_a_value_error() {
    let file: Value = serde_json::from_str(include_str!("../../../conformance/overflow.json")).unwrap();
    for case in file["cases"].as_array().unwrap() {
        let (kind, op, a) = (case["kind"].as_str().unwrap(), case["op"].as_str().unwrap(), case["a"].as_str().unwrap());
        let b = case["b"].as_str().unwrap_or("0");
        let said = format!("{kind} {a} {op} {b}");
        let got = if kind == "whole" {
            whole(op, a.parse().unwrap(), b.parse().unwrap())
        } else {
            decimal(op, Decimal::from_str(a).unwrap(), Decimal::from_str(b).unwrap())
        };
        match got {
            Ok(result) => assert_eq!(result, case["result"].as_str().unwrap_or("an error"), "{said}"),
            Err(e) => {
                assert!(case["error"].as_bool() == Some(true), "{said}: {e}");
                assert!(matches!(e, Error::TooLarge), "{said}: a number too large, not {e}");
            }
        }
    }
}
