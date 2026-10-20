//! Engines hosted by the Rust runtime, against the cases every host shares
//! (conformance/engines.json).

#![cfg(feature = "engines")]

use std::sync::{Arc, Mutex};

use clarity_runtime::engine::{Engine, Limits, Services};
use serde_json::Value;

const CASES: &str = include_str!("../../../conformance/engines.json");
const TEST_ENGINE: &[u8] = include_bytes!("../../../conformance/engines/test.wasm");

fn cases() -> Value {
    serde_json::from_str(CASES).unwrap()
}

/// The limits every case runs under.
fn limits(cases: &Value) -> Limits {
    Limits { seconds: cases["limits"]["seconds"].as_u64().unwrap() as u32, megabytes: cases["limits"]["megabytes"].as_u64().unwrap() as u32 }
}

/// The services an engine uses, as `load` takes them.
fn uses(list: &Value) -> Vec<&str> {
    list.as_array().unwrap().iter().map(|f| f.as_str().unwrap()).collect()
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
    let engine = Engine::load("Test", TEST_ENGINE, &functions, &uses(&cases["uses"]), limits(&cases)).unwrap();
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
    let cases = cases();
    for case in cases["refused"].as_array().unwrap() {
        let bytes = match case["bytes"].as_str() {
            Some(text) => hex(text),
            None => TEST_ENGINE.to_vec(),
        };
        let functions = case.get("functions").map(names).unwrap_or_default();
        let functions: Vec<&str> = functions.iter().map(String::as_str).collect();
        let used = uses(case.get("uses").unwrap_or(&cases["uses"]));
        let error = Engine::load("Test", &bytes, &functions, &used, limits(&cases)).err().unwrap_or_else(|| panic!("loaded: {case}"));
        let message = case["message"].as_str().unwrap();
        assert!(error.to_string().contains(message), "{error} lacks {message:?}");
        assert!(error.to_string().starts_with("engine Test "), "{error}");
    }
}

#[test]
fn a_call_keeps_to_the_limits_of_its_engine() {
    let cases = cases();
    let functions = names(&cases["functions"]);
    let functions: Vec<&str> = functions.iter().map(String::as_str).collect();
    let engine = Engine::load("Test", TEST_ENGINE, &functions, &uses(&cases["uses"]), limits(&cases)).unwrap();
    for case in cases["limited"].as_array().unwrap() {
        let function = case["function"].as_str().unwrap();
        let got = engine.call(function, case["args"].as_array().unwrap().clone());
        match (case.get("gives"), case["fails"].as_str()) {
            (Some(gives), _) => assert_eq!(serde_json::json!({ "value": got.unwrap() }), *gives, "{case}"),
            (None, Some(fails)) => {
                let error = got.err().unwrap_or_else(|| panic!("answered: {case}")).to_string();
                assert!(error.starts_with(fails), "{error:?} does not start with {fails:?}");
            }
            (None, None) => panic!("a case gives or fails: {case}"),
        }
    }
}

#[test]
fn an_engine_uses_the_services_it_declares() {
    let cases = cases();
    let functions = names(&cases["functions"]);
    let functions: Vec<&str> = functions.iter().map(String::as_str).collect();
    let lines = Arc::new(Mutex::new(Vec::<String>::new()));
    let written = lines.clone();
    let services = Services {
        clock: cases["services"]["clock"].as_i64(),
        seed: cases["services"]["seed"].as_u64(),
        log: Arc::new(move |line: &str| written.lock().unwrap().push(line.to_string())),
    };
    let engine = Engine::load("Test", TEST_ENGINE, &functions, &uses(&cases["uses"]), limits(&cases)).unwrap().with_services(services);
    for case in cases["served"].as_array().unwrap() {
        lines.lock().unwrap().clear();
        let got = engine.call(case["function"].as_str().unwrap(), case["args"].as_array().unwrap().clone()).unwrap();
        assert_eq!(serde_json::json!({ "value": got }), case["gives"], "{case}");
        let logged: Vec<Value> = lines.lock().unwrap().iter().map(|l| Value::from(l.as_str())).collect();
        assert_eq!(Value::Array(logged), case.get("log").cloned().unwrap_or(Value::Array(vec![])), "{case}");
    }
}
