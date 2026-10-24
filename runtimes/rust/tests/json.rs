//! Reading JSON into records (DESIGN 5.23) against the cases every runtime
//! shares (conformance/json.json): what is read is fitted to the entity.

use clarity_runtime::json::{self, Field, Kind};
use serde_json::Value;

const CASES: &str = include_str!("../../../conformance/json.json");

/// Kept for the test's length: the kinds name static values.
fn leak<T>(value: T) -> &'static T {
    Box::leak(Box::new(value))
}

fn kind(word: &str, field: &Value) -> Kind {
    match word {
        "text" => Kind::Text,
        "integer" => Kind::Integer,
        "decimal" => Kind::Decimal,
        "bool" => Kind::Bool,
        "date" => Kind::Date,
        "datetime" => Kind::DateTime,
        "enum" => {
            let values: Vec<&'static str> = field["values"].as_array().unwrap().iter().map(|v| &**leak(v.as_str().unwrap().to_string())).collect();
            Kind::Enum(leak(values).as_slice())
        }
        "values" => Kind::Values(leak(kind(field["item"].as_str().unwrap(), field))),
        "other" => Kind::Other,
        other => panic!("no kind {other}"),
    }
}

#[test]
fn json_is_fitted_as_every_runtime_fits_it() {
    let cases: Value = serde_json::from_str(CASES).unwrap();
    for case in cases["cases"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let fields: Vec<Field> = case["fields"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| Field { name: leak(f["name"].as_str().unwrap().to_string()), kind: kind(f["kind"].as_str().unwrap(), f) })
            .collect();
        let got = json::fit(
            case["json"].as_str().unwrap(),
            case["at"].as_str(),
            case["many"].as_bool().unwrap_or(false),
            case["entity"].as_str().unwrap(),
            &fields,
            &case["base"],
        );
        match case.get("error") {
            Some(error) => assert_eq!(got, Err(error.as_str().unwrap().to_string()), "{name}"),
            None => assert_eq!(got, Ok(case["fitted"].clone()), "{name}"),
        }
    }
}
