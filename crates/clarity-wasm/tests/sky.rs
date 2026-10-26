//! The engine beside the screen written in Clarity that the renderers are
//! tested with (conformance/engines/sky.clarity, screen-engines.json
//! `written`). The renderers run the compiled sky.wasm, which this test
//! keeps in step with its source: with CLARITY_RECORD_SKY=1 it writes it.

use clarity_runtime::engine::{Declaration, Engine, Limits, State};
use serde_json::Value;

const SOURCE: &str = include_str!("../../../conformance/engines/sky.clarity");
const CASES: &str = include_str!("../../../conformance/screen-engines.json");

fn compiled() -> Vec<u8> {
    let project = clarity_spec::load_sources([("sky.clarity", SOURCE)]);
    let errors: Vec<&str> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.as_str()).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    clarity_wasm::compile(&project, "Sky").unwrap_or_else(|e| panic!("{e:#?}"))
}

#[test]
fn sky_wasm_is_its_source_compiled() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance/engines/sky.wasm");
    let bytes = compiled();
    if std::env::var_os("CLARITY_RECORD_SKY").is_some() {
        std::fs::write(&path, &bytes).unwrap();
    }
    assert_eq!(std::fs::read(&path).ok().as_deref(), Some(bytes.as_slice()), "sky.wasm is not sky.clarity compiled: write it with CLARITY_RECORD_SKY=1");
}

/// Its frames as the cases give them, called as the renderers call them:
/// hears first, then clarity_tick, then clarity_draw (its tells the renderers' tests check).
#[test]
fn its_frames_draw_what_the_cases_say() {
    let cases: Value = serde_json::from_str(CASES).unwrap();
    let functions = ["clarity_tick", "clarity_draw", "start"];
    let declaration = Declaration { functions: &functions, uses: &[], limits: Limits { seconds: 5, megabytes: 64 }, state: State::App, opens: false };
    let engine = Engine::load("Sky", &compiled(), &declaration).unwrap();
    for (i, frame) in cases["written"]["frames"].as_array().unwrap().iter().enumerate() {
        for input in frame["input"].as_array().unwrap() {
            engine.call(input["hear"].as_str().unwrap(), input["values"].as_array().unwrap().clone()).unwrap();
        }
        assert_eq!(engine.call("clarity_tick", vec![frame["tick"].clone()]).unwrap(), Value::Null);
        assert_eq!(engine.call("clarity_draw", vec![]).unwrap(), frame["draws"], "frame {i}");
    }
}
