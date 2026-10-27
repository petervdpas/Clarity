//! Engines written in Clarity, compiled to WebAssembly or run by the
//! interpreter, each run by the Rust runtime's engine host as any engine
//! file (conformance/clarity-engines.json): both ways give the same.

use clarity_runtime::engine::{Declaration, Engine, Limits, Services, State};
use serde_json::{Value, json};

const CASES: &str = include_str!("../../../conformance/clarity-engines.json");

#[test]
fn clarity_engines_give_what_the_cases_say() {
    let cases: Value = serde_json::from_str(CASES).unwrap();
    let project = clarity_spec::load_sources([("play.clarity", cases["source"].as_str().unwrap())]);
    let errors: Vec<&str> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.as_str()).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    for (mode, run) in [clarity_wasm::Mode::Compiled, clarity_wasm::Mode::Interpreted].into_iter().flat_map(|m| cases["engines"].as_array().unwrap().iter().map(move |r| (m, r))) {
        let name = run["engine"].as_str().unwrap();
        let bytes = clarity_wasm::build(&project, name, mode).unwrap_or_else(|e| panic!("{mode:?} {e:#?}"));
        let decl = project.files.iter().flat_map(|f| &f.engines).find(|e| e.name.name == name).unwrap();
        let functions: Vec<&str> = decl.functions.iter().map(|f| f.name.name.as_str()).collect();
        let uses: Vec<&str> = run["uses"].as_array().unwrap().iter().map(|u| u.as_str().unwrap()).collect();
        let state = match run["state"].as_str().unwrap() {
            "app" => State::App,
            _ => State::None,
        };
        let declaration = Declaration { functions: &functions, uses: &uses, limits: Limits { seconds: 5, megabytes: 64 }, state, opens: false };
        let services = Services { seed: Some(cases["seed"].as_u64().unwrap()), ..Services::default() };
        let engine = Engine::load(name, &bytes, &declaration).unwrap_or_else(|e| panic!("{e}")).with_services(services);
        for case in run["calls"].as_array().unwrap() {
            let function = case["function"].as_str().unwrap();
            let got = match engine.call(function, case["args"].as_array().unwrap().clone()) {
                Ok(value) => json!({ "value": value }),
                Err(error) => json!({ "error": error.to_string() }),
            };
            assert_eq!(got, case["gives"], "{mode:?} {name}: {case}");
        }
    }
}
