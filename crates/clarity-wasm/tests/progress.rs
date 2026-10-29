//! An engine written in Clarity says how far a long call is, compiled or
//! interpreted, as an engine brought along does (conformance/progress.json):
//! `progress(day, total)` is the service `progress` of module `clarity`.

use std::sync::{Arc, Mutex};

use clarity_runtime::engine::{Declaration, Engine, Limits, State, Watch};
use serde_json::json;

const PLAN: &str = "module plan

engine Planner
    uses progress

    function plan(days: list<integer>, total: integer) -> integer
        for day in days
            progress(day, total)
        return total
";

#[test]
fn a_clarity_engine_reports_how_far_it_is() {
    let project = clarity_spec::load_sources([("plan.clarity", PLAN)]);
    let errors: Vec<&str> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.as_str()).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    for mode in [clarity_wasm::Mode::Compiled, clarity_wasm::Mode::Interpreted] {
        let bytes = clarity_wasm::build(&project, "Planner", mode).unwrap_or_else(|e| panic!("{mode:?} {e:#?}"));
        let declaration = Declaration { functions: &["plan"], uses: &["progress"], limits: Limits { seconds: 5, megabytes: 64 }, state: State::None, opens: false };
        let engine = Engine::load("Planner", &bytes, &declaration).unwrap_or_else(|e| panic!("{mode:?} {e}"));
        let reports = Arc::new(Mutex::new(Vec::new()));
        let kept = reports.clone();
        let watch = Watch { progress: Some(Arc::new(move |done, total| kept.lock().unwrap().push((done, total)))), ..Watch::default() };
        assert_eq!(engine.call_watched(0, "plan", vec![json!([1, 2, 3]), json!(3)], &watch).unwrap(), json!(3), "{mode:?}");
        assert_eq!(*reports.lock().unwrap(), [(1, 3), (2, 3), (3, 3)], "{mode:?}");
    }
}
