//! The noughts example's Board, written in Clarity, compiled and played
//! through the Rust runtime's engine host.

use clarity_runtime::engine::{Declaration, Engine, Limits, Services, State};
use serde_json::json;

#[test]
fn the_noughts_board_compiles_and_plays() {
    let source = include_str!("../../../examples/noughts/noughts.clarity");
    let project = clarity_spec::load_sources([("noughts.clarity", source)]);
    for mode in [clarity_wasm::Mode::Compiled, clarity_wasm::Mode::Interpreted] {
        plays(&project, clarity_wasm::build(&project, "Board", mode).unwrap_or_else(|e| panic!("{mode:?} {e:#?}")));
    }
}

fn plays(project: &clarity_spec::Project, bytes: Vec<u8>) {
    let decl = project.files.iter().flat_map(|f| &f.engines).find(|e| e.name.name == "Board").unwrap();
    let functions: Vec<&str> = decl.functions.iter().map(|f| f.name.name.as_str()).collect();
    let declaration = Declaration { functions: &functions, uses: &["random"], limits: Limits { seconds: 5, megabytes: 64 }, state: State::Window, opens: false };
    let engine = Engine::load("Board", &bytes, &declaration).unwrap().with_services(Services { seed: Some(7), ..Services::default() });
    assert_eq!(engine.call_in(1, "play", vec![json!(5)]).unwrap(), json!("yourMove"));
    assert_eq!(engine.call_in(1, "mark", vec![json!(5)]).unwrap(), json!(1));
    // The computer answered somewhere: two squares are taken.
    let taken: i64 = (1..=9).map(|s| engine.call_in(1, "mark", vec![json!(s)]).unwrap().as_i64().unwrap()).filter(|m| *m != 0).count() as i64;
    assert_eq!(taken, 2);
    assert_eq!(engine.call_in(1, "play", vec![json!(5)]).unwrap_err().to_string(), "thatSquareIsTaken");
    // Another window has a board of its own.
    assert_eq!(engine.call_in(2, "mark", vec![json!(5)]).unwrap(), json!(0));
}
