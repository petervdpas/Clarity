//! A long engine call belongs to the handler that waits for it (DESIGN 5.12):
//! its progress and its Stop are that handler's watch, on that handler's
//! thread. A handler beside it (another window stack, a session) sees none
//! of its progress and its Stop stops nothing of it; a report after the
//! handler ended goes nowhere.

#![cfg(feature = "engines")]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};

use clarity_runtime::engine::{Declaration, Engine, Limits, State, Watch, watching};
use serde_json::json;

const LONG: &[u8] = include_bytes!("../../../conformance/engines/long.wasm");

fn engine() -> Engine {
    Engine::load("Long", LONG, &Declaration { functions: &["work"], uses: &["progress"], limits: Limits { seconds: 60, megabytes: 16 }, state: State::None, opens: false }).unwrap()
}

/// A watch that keeps every report.
fn kept() -> (Watch, Arc<Mutex<Vec<(i64, i64)>>>) {
    let reports = Arc::new(Mutex::new(Vec::new()));
    let sink = reports.clone();
    let watch = Watch { progress: Some(Arc::new(move |done, total| sink.lock().unwrap().push((done, total)))), stop: Arc::new(AtomicBool::new(false)) };
    (watch, reports)
}

#[test]
fn a_long_call_holds_back_only_the_handler_it_belongs_to() {
    let (waiting, waiting_saw) = kept();
    let (started, ready) = mpsc::channel();
    let stop = waiting.stop.clone();
    let long = std::thread::spawn(move || {
        let engine = engine();
        started.send(()).unwrap();
        watching(&waiting, || engine.call("work", vec![json!(1_000_000_000_i64)]))
    });
    ready.recv().unwrap();
    // Wait until the long call has reported.
    let until = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while waiting_saw.lock().unwrap().is_empty() && std::time::Instant::now() < until {
        std::thread::yield_now();
    }
    // A handler beside it runs, sees only its own progress, and its Stop stops nothing of the first.
    let (other, other_saw) = kept();
    let ran = watching(&other, || engine().call("work", vec![json!(2)]));
    assert_eq!(ran.unwrap(), json!(2));
    assert_eq!(*other_saw.lock().unwrap(), [(1, 2), (2, 2)]);
    other.stop.store(true, Ordering::SeqCst);
    std::thread::sleep(std::time::Duration::from_millis(50));
    assert!(!long.is_finished(), "the other's Stop stopped the long call");
    assert!(waiting_saw.lock().unwrap().iter().all(|(_, total)| *total == 1_000_000_000));
    // Its own Stop ends it.
    stop.store(true, Ordering::SeqCst);
    assert_eq!(long.join().unwrap().unwrap_err().to_string(), "engine Long failed in work: it was stopped");
}

#[test]
fn a_report_after_the_handler_ended_shows_nothing() {
    let (watch, saw) = kept();
    let engine = engine();
    watching(&watch, || engine.call("work", vec![json!(1)])).unwrap();
    assert_eq!(saw.lock().unwrap().len(), 1);
    // The handler has ended: a call after it is watched by nobody.
    engine.call("work", vec![json!(3)]).unwrap();
    assert_eq!(saw.lock().unwrap().len(), 1);
}

/// A watch the screen knows by number (`Program::start_watch` of a generated
/// program): the screen asks how far the call is and stops it by that
/// number, plain data both ways; once ended, the thread's calls are watched
/// by nobody.
#[test]
fn a_numbered_watch_says_how_far_and_stops_by_its_number() {
    use clarity_runtime::engine::{end_watch, start_watch, stop_watch, watch_progress};
    const ID: u64 = 9_001;
    let engine = engine();
    start_watch(ID);
    engine.call("work", vec![json!(2)]).unwrap();
    assert_eq!(Some((2, 2)), watch_progress(ID));
    stop_watch(ID);
    assert_eq!(engine.call("work", vec![json!(5)]).unwrap_err().to_string(), "engine Long failed in work: it was stopped");
    end_watch(ID);
    assert_eq!(None, watch_progress(ID));
    // Watched by nobody now: Stop no more holds, nothing is kept.
    assert_eq!(engine.call("work", vec![json!(1)]).unwrap(), json!(1));
    assert_eq!(None, watch_progress(ID));
    // A Stop that comes before the handler starts holds once it does.
    stop_watch(ID + 1);
    start_watch(ID + 1);
    assert!(engine.call("work", vec![json!(3)]).is_err());
    end_watch(ID + 1);
    end_watch(ID + 1);
}
