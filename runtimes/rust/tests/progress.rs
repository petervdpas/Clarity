//! Progress of a long engine call and Stop, against the cases every host
//! shares (conformance/progress.json).

#![cfg(feature = "engines")]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use clarity_runtime::engine::{Declaration, Engine, Limits, State, Watch};
use serde_json::{Value, json};

const CASES: &str = include_str!("../../../conformance/progress.json");
const LONG: &[u8] = include_bytes!("../../../conformance/engines/long.wasm");

fn cases() -> Value {
    serde_json::from_str(CASES).unwrap()
}

fn load(cases: &Value, uses: &[&str]) -> clarity_runtime::Result<Engine> {
    let limits = Limits { seconds: cases["limits"]["seconds"].as_u64().unwrap() as u32, megabytes: cases["limits"]["megabytes"].as_u64().unwrap() as u32 };
    Engine::load(cases["name"].as_str().unwrap(), LONG, &Declaration { functions: &["work"], uses, limits, state: State::None, opens: false })
}

/// A watch that keeps every report and sets Stop at report `stop_at` (0: never by a report).
fn watch(stop_at: i64) -> (Watch, Arc<Mutex<Vec<(i64, i64)>>>) {
    let reports = Arc::new(Mutex::new(Vec::new()));
    let stop = Arc::new(AtomicBool::new(false));
    let (kept, flag) = (reports.clone(), stop.clone());
    let progress = Arc::new(move |done: i64, total: i64| {
        kept.lock().unwrap().push((done, total));
        if done == stop_at {
            flag.store(true, Ordering::SeqCst);
        }
    });
    (Watch { progress: Some(progress), stop }, reports)
}

fn pairs(list: &Value) -> Vec<(i64, i64)> {
    list.as_array().unwrap().iter().map(|p| (p[0].as_i64().unwrap(), p[1].as_i64().unwrap())).collect()
}

#[test]
fn a_long_call_reports_how_far_it_is() {
    let cases = cases();
    let engine = load(&cases, &["progress"]).unwrap();
    for case in cases["reported"].as_array().unwrap() {
        let (watch, reports) = watch(0);
        let got = engine.call_watched(0, case["function"].as_str().unwrap(), case["args"].as_array().unwrap().clone(), &watch).unwrap();
        assert_eq!(got, case["gives"], "{case}");
        assert_eq!(*reports.lock().unwrap(), pairs(&case["progress"]), "{case}");
    }
    // Without a sink, the reports go nowhere.
    assert_eq!(engine.call("work", vec![json!(3)]).unwrap(), json!(3));
}

#[test]
fn stop_ends_a_long_call() {
    let cases = cases();
    let engine = load(&cases, &["progress"]).unwrap();
    for case in cases["stopped"].as_array().unwrap() {
        let (watch, reports) = watch(case["stopAt"].as_i64().unwrap());
        if case["stopAt"] == 0 {
            watch.stop.store(true, Ordering::SeqCst);
        }
        let got = engine.call_watched(0, case["function"].as_str().unwrap(), case["args"].as_array().unwrap().clone(), &watch);
        assert_eq!(got.unwrap_err().to_string(), case["fails"].as_str().unwrap(), "{case}");
        let first = pairs(&case["firstReports"]);
        assert_eq!(reports.lock().unwrap()[..first.len()], first[..], "{case}");
    }
}

#[test]
fn progress_is_a_service_the_engine_must_use() {
    let cases = cases();
    let refused = load(&cases, &[]).err().unwrap();
    assert_eq!(refused.to_string(), cases["refused"]["message"].as_str().unwrap());
}
