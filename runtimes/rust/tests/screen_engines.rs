//! Engines beside the screen on Rust (DESIGN 5.12): wasmi in the program's
//! own process, one instance per canvas, which keeps its memory from frame to
//! frame, as C#'s ScreenEngine (conformance/screen-engines.json).

use clarity_runtime::screen::ScreenInput;
use serde_json::Value;

const CASES: &str = include_str!("../../../conformance/screen-engines.json");

fn cases() -> Value {
    serde_json::from_str(CASES).unwrap()
}

/// An input as the conformance file writes it.
fn input_of(i: &Value) -> ScreenInput {
    if let Some(kind) = i["pointer"].as_str() {
        ScreenInput::Pointer { kind: kind.into(), x: i["x"].as_i64().unwrap(), y: i["y"].as_i64().unwrap() }
    } else if let Some(hear) = i["hear"].as_str() {
        ScreenInput::Hear { name: hear.into(), values: i["values"].as_array().unwrap().clone() }
    } else {
        ScreenInput::Key { kind: i["key"].as_str().unwrap().into(), name: i["name"].as_str().unwrap().into() }
    }
}

fn inputs(list: &Value) -> Vec<ScreenInput> {
    list.as_array().map(|l| l.iter().map(input_of).collect()).unwrap_or_default()
}

#[test]
fn moves_in_a_row_keep_only_the_last() {
    for case in cases()["queued"].as_array().unwrap() {
        let mut waiting = Vec::new();
        for given in case["given"].as_array().unwrap() {
            ScreenInput::queue(&mut waiting, input_of(given));
        }
        assert_eq!(waiting, inputs(&case["sent"]));
    }
}

#[cfg(feature = "engines")]
mod engines {
    use super::*;
    use std::sync::{Arc, Mutex};

    use clarity_runtime::screen::{Frames, Hears};
    use serde_json::json;
    use clarity_runtime::engine::{Limits, ScreenEngine, Services};

    const SCREEN: &[u8] = include_bytes!("../../../conformance/engines/screen.wasm");

    fn uses() -> Vec<String> {
        cases()["uses"].as_array().unwrap().iter().map(|u| u.as_str().unwrap().to_string()).collect()
    }

    fn load(opens: bool, seconds: u32) -> ScreenEngine {
        let uses = uses();
        let uses: Vec<&str> = uses.iter().map(String::as_str).collect();
        ScreenEngine::load("Screen", SCREEN, &uses, opens, Limits { seconds, megabytes: 256 }).unwrap()
    }

    fn told(drawn: &clarity_runtime::screen::Drawn) -> Value {
        Value::Array(drawn.tells.iter().map(|t| json!({ "tell": t.tell, "values": t.values })).collect())
    }

    #[test]
    fn each_frame_gives_the_canvas_s_shapes() {
        for run in cases()["frames"].as_array().unwrap() {
            let mut engine = load(run["opens"].as_bool().unwrap(), 5);
            for (i, (tick, draws)) in run["ticks"].as_array().unwrap().iter().zip(run["draws"].as_array().unwrap()).enumerate() {
                let drawn = engine.frame(tick.as_f64().unwrap(), &[]).unwrap();
                assert_eq!(&Value::Array(drawn.shapes), draws, "{}, frame {i}", run["name"]);
            }
        }
    }

    #[test]
    fn a_frame_that_fails_or_runs_too_long_ends_the_engine() {
        for fails in cases()["fails"].as_array().unwrap() {
            let mut engine = load(true, fails["seconds"].as_u64().unwrap() as u32);
            for ms in fails["ticks"].as_array().unwrap() {
                engine.frame(ms.as_f64().unwrap(), &[]).unwrap();
            }
            let failed = engine.frame(fails["tick"].as_f64().unwrap(), &inputs(&fails["input"])).unwrap_err();
            assert_eq!(failed, fails["fails"].as_str().unwrap(), "{}", fails["name"]);
            // Ended, it draws no more.
            assert_eq!(engine.frame(16.0, &[]).unwrap_err(), failed);
        }
    }

    #[test]
    fn pointer_and_keys_reach_the_engine_before_its_tick() {
        for run in cases()["input"].as_array().unwrap() {
            let mut engine = load(run["opens"].as_bool().unwrap(), 5);
            assert_eq!(engine.hears(), Hears { pointer: true, keys: true });
            for (i, frame) in run["frames"].as_array().unwrap().iter().enumerate() {
                let drawn = engine.frame(frame["tick"].as_f64().unwrap(), &inputs(&frame["input"])).unwrap();
                assert_eq!(Value::Array(drawn.shapes), frame["draws"], "{}, frame {i}", run["name"]);
            }
        }
    }

    #[test]
    fn an_engine_tells_with_its_answers_and_hears_through_its_functions() {
        for run in cases()["tells"].as_array().unwrap() {
            let mut engine = load(run["opens"].as_bool().unwrap(), 5);
            for (i, frame) in run["frames"].as_array().unwrap().iter().enumerate() {
                let drawn = engine.frame(frame["tick"].as_f64().unwrap(), &inputs(&frame["input"])).unwrap();
                assert_eq!(told(&drawn), frame["tells"], "frame {i}");
                assert_eq!(Value::Array(drawn.shapes), frame["draws"], "frame {i}");
            }
        }
    }

    #[test]
    fn an_engine_beside_the_screen_written_in_clarity_runs_as_any_compiled_or_interpreted() {
        let written = &cases()["written"];
        for file in written["engines"].as_array().unwrap() {
            let file = file.as_str().unwrap();
            let bytes = std::fs::read(format!("{}/../../conformance/engines/{file}", env!("CARGO_MANIFEST_DIR"))).unwrap();
            let mut engine = ScreenEngine::load(written["name"].as_str().unwrap(), &bytes, &[], false, Limits { seconds: 5, megabytes: 256 }).unwrap();
            for (i, frame) in written["frames"].as_array().unwrap().iter().enumerate() {
                let drawn = engine.frame(frame["tick"].as_f64().unwrap(), &inputs(&frame["input"])).unwrap();
                assert_eq!(told(&drawn), frame["tells"], "{file}, frame {i}");
                assert_eq!(Value::Array(drawn.shapes), frame["draws"], "{file}, frame {i}");
            }
        }
    }

    #[test]
    fn an_engine_beside_the_screen_logs_with_the_service_it_uses() {
        for run in cases()["logs"].as_array().unwrap() {
            let lines = Arc::new(Mutex::new(Vec::<String>::new()));
            let kept = lines.clone();
            let services = Services { log: Arc::new(move |line: &str| kept.lock().unwrap().push(line.to_string())), ..Services::default() };
            let uses = uses();
            let uses: Vec<&str> = uses.iter().map(String::as_str).collect();
            let mut engine = ScreenEngine::load("Screen", SCREEN, &uses, run["opens"].as_bool().unwrap(), Limits { seconds: 5, megabytes: 256 }).unwrap().with_services(services);
            engine.frame(0.0, &inputs(&run["input"])).unwrap();
            let expected: Vec<String> = run["lines"].as_array().unwrap().iter().map(|l| l.as_str().unwrap().to_string()).collect();
            assert_eq!(*lines.lock().unwrap(), expected);
        }
    }

    #[test]
    fn an_engine_beside_the_screen_exports_its_frame_loop() {
        let test = include_bytes!("../../../conformance/engines/test.wasm");
        let refused = ScreenEngine::load("Test", test, &["clock", "random", "log", "language"], false, Limits { seconds: 5, megabytes: 256 }).err().expect("refused");
        assert_eq!(refused, "engine Test lacks clarity_tick, which an engine beside the screen needs");
    }

    /// A canvas holds its engine as frames, whatever made them.
    #[test]
    fn a_screen_engine_is_frames() {
        let mut frames: Box<dyn Frames> = Box::new(load(true, 5));
        assert!(frames.frame(0.0, &[]).is_ok());
    }
}
