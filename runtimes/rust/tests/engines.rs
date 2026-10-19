//! Engines hosted by the Rust runtime, against the cases every host shares
//! (conformance/engines.json).

#![cfg(feature = "engines")]

use clarity_runtime::engine::Engine;
use serde_json::Value;

const CASES: &str = include_str!("../../../conformance/engines.json");
const TEST_ENGINE: &[u8] = include_bytes!("../../../conformance/engines/test.wasm");

fn cases() -> Value {
    serde_json::from_str(CASES).unwrap()
}

fn names(list: &Value) -> Vec<String> {
    list.as_array().unwrap().iter().map(|f| f.as_str().unwrap().to_string()).collect()
}

fn hex(text: &str) -> Vec<u8> {
    (0..text.len()).step_by(2).map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap()).collect()
}

#[test]
fn every_call_gives_what_every_host_gives() {
    let cases = cases();
    let functions = names(&cases["functions"]);
    let functions: Vec<&str> = functions.iter().map(String::as_str).collect();
    let engine = Engine::load("Test", TEST_ENGINE, &functions).unwrap();
    for case in cases["calls"].as_array().unwrap() {
        let function = case["function"].as_str().unwrap();
        let args = case["args"].as_array().unwrap().clone();
        let got = match engine.call(function, args) {
            Ok(value) => serde_json::json!({ "value": value }),
            Err(error) => serde_json::json!({ "error": error.to_string() }),
        };
        assert_eq!(got, case["gives"], "{case}");
    }
}

#[test]
fn a_module_that_is_no_engine_is_refused_when_it_loads() {
    for case in cases()["refused"].as_array().unwrap() {
        let bytes = match case["bytes"].as_str() {
            Some(text) => hex(text),
            None => TEST_ENGINE.to_vec(),
        };
        let functions = case.get("functions").map(names).unwrap_or_default();
        let functions: Vec<&str> = functions.iter().map(String::as_str).collect();
        let error = Engine::load("Test", &bytes, &functions).err().unwrap_or_else(|| panic!("loaded: {case}"));
        let message = case["message"].as_str().unwrap();
        assert!(error.to_string().contains(message), "{error} lacks {message:?}");
        assert!(error.to_string().starts_with("engine Test "), "{error}");
    }
}
