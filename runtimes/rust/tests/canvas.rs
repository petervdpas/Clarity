//! Canvas modes (DESIGN 5.31) against the cases every runtime shares
//! (conformance/canvas-modes.json), and the edit `canvas_event` makes.

use clarity_runtime::canvas::{self, CanvasArgs};
use clarity_runtime::ui::{Reply, Ui, WindowState};
use clarity_runtime::{Db, Result};
use serde_json::{Map, Value, json};

const CASES: &str = include_str!("../../../conformance/canvas-modes.json");

fn cases(part: &str) -> Vec<Value> {
    let all: Value = serde_json::from_str(CASES).unwrap();
    all[part].as_array().unwrap().clone()
}

fn units(value: &Value) -> [i64; 2] {
    serde_json::from_value(value.clone()).unwrap()
}

fn shapes(value: &Value) -> Vec<Map<String, Value>> {
    serde_json::from_value(value.clone()).unwrap()
}

#[test]
fn a_drawing_is_written_and_read_as_svg() {
    for case in cases("svg") {
        let name = case["name"].as_str().unwrap();
        let text = canvas::drawing_text(units(&case["units"]), &shapes(&case["shapes"]));
        assert_eq!(text, case["text"].as_str().unwrap(), "{name}: written");
        let (read_units, read) = canvas::drawing_shapes(&text, [1000, 1000]);
        assert_eq!(read_units, units(&case["units"]), "{name}: units");
        assert_eq!(json!(read), case["shapes"], "{name}: read");
    }
    for case in cases("read") {
        let name = case["name"].as_str().unwrap();
        let (read_units, read) = canvas::drawing_shapes(case["text"].as_str().unwrap(), [1000, 1000]);
        if !case["units"].is_null() {
            assert_eq!(read_units, units(&case["units"]), "{name}: units");
        }
        assert_eq!(json!(read), case["shapes"], "{name}: read");
    }
}

#[test]
fn a_canvas_action_changes_the_data_as_every_runtime_does() {
    for case in cases("actions") {
        let name = case["name"].as_str().unwrap();
        let args: CanvasArgs = serde_json::from_value(case.clone()).unwrap_or_else(|e| panic!("{name}: {e}"));
        let mut state = WindowState::new("", case["data"].as_object().unwrap().clone());
        let changed = canvas::apply(&mut state, &args, case["action"].as_object().unwrap()).unwrap_or_else(|e| panic!("{name}: {e}"));
        let refused = case["refused"] == true;
        assert_eq!(changed, !refused, "{name}: changed");
        if refused {
            assert_eq!(Value::Object(state.data), case["data"], "{name}: a refused action changed the data");
            continue;
        }
        assert_eq!(Value::Object(state.data.clone()), case["after"], "{name}: data");
        assert_eq!(json!(state.selected.get(&args.part)), case["selected"], "{name}: selected");
    }
}

fn drawn(state: WindowState) -> Result<Reply> {
    let data = state.data.clone();
    Ok(Ui::from_state(&state).reply(&state.window, data))
}

fn card() -> WindowState {
    let mut data = Map::new();
    data.insert("types".into(), json!([{ "id": 1, "x": 0, "y": 0 }, { "id": 2, "x": 40, "y": 40 }]));
    data.insert("note".into(), json!({ "drawing": "" }));
    WindowState::new("Card", data)
}

fn objects() -> CanvasArgs {
    serde_json::from_value(json!({ "part": "diagram", "mode": "objects", "list": "types", "at": ["x", "y"], "grid": 20, "snap": true })).unwrap()
}

fn action(v: Value) -> Map<String, Value> {
    v.as_object().unwrap().clone()
}

#[test]
fn a_dropped_object_is_one_edit_and_chosen() {
    let db = Db::memory().unwrap();
    let state = card();
    clarity_runtime::edits::replied(&drawn(state.clone()).unwrap());
    let mut ran = false;
    let reply = canvas::event(&db, state.clone(), &objects(), &action(json!({ "move": { "id": 2, "x": 67, "y": 31 } })), Some("diagram"), |state| {
        ran = true;
        drawn(state)
    }, drawn)
    .unwrap();
    assert!(ran, "on change runs");
    assert!(!reply.refused);
    assert_eq!(reply.state.data["types"], json!([{ "id": 1, "x": 0, "y": 0 }, { "id": 2, "x": 60, "y": 40 }]));
    assert_eq!(reply.state.selected.get("diagram"), Some(&1), "by its place in the list");
    let back = clarity_runtime::edits::undo(&db, reply.state, "diagram", true, drawn).unwrap();
    assert_eq!(back.state.data["types"], state.data["types"], "Ctrl+Z takes the move back");
}

#[test]
fn a_refused_action_runs_nothing_and_says_refused() {
    let db = Db::memory().unwrap();
    let reply = canvas::event(&db, card(), &objects(), &action(json!({ "move": { "id": 1, "x": 5, "y": -4 } })), None, |_| panic!("dropped where it is"), drawn).unwrap();
    assert!(reply.refused);
    assert_eq!(reply.state.data, card().data);
}

#[test]
fn a_shape_is_drawn_into_the_text_field() {
    let db = Db::memory().unwrap();
    let args: CanvasArgs = serde_json::from_value(json!({ "part": "drawing", "mode": "vector", "path": "note.drawing", "units": [400, 300] })).unwrap();
    let add = action(json!({ "add": { "op": "rect", "x": 1, "y": 2, "width": 3, "height": 4, "color": "#ff0000" } }));
    let reply = canvas::event(&db, card(), &args, &add, None, drawn, drawn).unwrap();
    let want = "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 400 300\">\n<rect x=\"1\" y=\"2\" width=\"3\" height=\"4\" fill=\"#ff0000\"/>\n</svg>\n";
    assert_eq!(reply.state.data["note"]["drawing"], json!(want));
    assert_eq!(reply.state.selected.get("drawing"), Some(&0));
}

#[test]
fn an_action_that_does_not_fit_its_mode_fails() {
    let mut state = card();
    let args: CanvasArgs = serde_json::from_value(json!({ "part": "x", "mode": "sculpt" })).unwrap();
    assert!(canvas::apply(&mut state, &args, &action(json!({ "move": {} }))).is_err());
    let args: CanvasArgs = serde_json::from_value(json!({ "part": "x", "mode": "vector", "path": "note.drawing" })).unwrap();
    assert!(canvas::apply(&mut state, &args, &action(json!({ "add": { "op": "star" } }))).is_err());
}
