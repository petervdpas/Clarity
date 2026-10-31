//! Reordering (DESIGN 5.30) against the cases every runtime shares
//! (conformance/reorder.json), and the move of a window's list that
//! `reorder_event` makes.

use clarity_runtime::reorder::{self, Move, Row};
use clarity_runtime::ui::{Reply, Ui, WindowState};
use clarity_runtime::{Db, Result};
use serde_json::{Map, Value, json};

const CASES: &str = include_str!("../../../conformance/reorder.json");

fn rows(case: &Value) -> Vec<Row> {
    case["rows"].as_array().unwrap().iter().map(|r| Row { id: r["id"].clone(), parent: r.get("parent").cloned().unwrap_or(Value::Null) }).collect()
}

fn check(part: &str, tree: bool) {
    let all: Value = serde_json::from_str(CASES).unwrap();
    for case in all[part].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let mv: Move = serde_json::from_value(case["move"].clone()).unwrap_or_else(|e| panic!("{name}: {e}"));
        let moved = reorder::reorder(&rows(case), &mv, tree);
        if case.get("refused").is_some() {
            assert!(moved.is_none(), "{name}: not refused");
            continue;
        }
        let moved = moved.unwrap_or_else(|| panic!("{name}: refused"));
        assert_eq!(json!(moved.order), case["order"], "{name}: order");
        assert_eq!(json!(moved.numbers), case["numbers"], "{name}: numbers");
        if tree {
            assert_eq!(json!(moved.parents), case["parents"], "{name}: parents");
        }
    }
}

#[test]
fn a_list_reorders_as_every_runtime_does() {
    check("lists", false);
}

#[test]
fn a_tree_reorders_as_every_runtime_does() {
    check("trees", true);
}

fn drawn(state: WindowState) -> Result<Reply> {
    let data = state.data.clone();
    Ok(Ui::from_state(&state).reply(&state.window, data))
}

fn tracks() -> WindowState {
    let mut data = Map::new();
    data.insert("tracks".into(), json!([{ "id": 1, "title": "A", "rank": 1 }, { "id": 2, "title": "B", "rank": 2 }, { "id": 3, "title": "C", "rank": 3 }]));
    WindowState::new("Album", data)
}

#[test]
fn a_move_gives_the_list_its_order_and_numbers_and_selects_the_row() {
    let db = Db::memory().unwrap();
    let mv: Move = serde_json::from_value(json!({ "row": 2, "where": "before", "target": 0 })).unwrap();
    let mut ran = None;
    let reply = reorder::event(&db, tracks(), "list", "tracks", "rank", None, &mv, None, |state| {
        ran = Some(state.data["tracks"].clone());
        drawn(state)
    }, drawn)
    .unwrap();
    let after = json!([{ "id": 3, "title": "C", "rank": 1 }, { "id": 1, "title": "A", "rank": 2 }, { "id": 2, "title": "B", "rank": 3 }]);
    assert_eq!(reply.state.data["tracks"], after);
    assert_eq!(ran, Some(after), "the handler runs on the new order");
    assert_eq!(reply.state.selected.get("list"), Some(&0));
}

#[test]
fn a_tree_move_writes_each_rows_parent() {
    let db = Db::memory().unwrap();
    let mut data = Map::new();
    data.insert("steps".into(), json!([{ "id": 1, "up": null, "n": 1 }, { "id": 2, "up": null, "n": 2 }]));
    let mv: Move = serde_json::from_value(json!({ "row": 1, "key": "right" })).unwrap();
    let reply = reorder::event(&db, WindowState::new("Plan", data), "tree", "steps", "n", Some("up"), &mv, None, drawn, drawn).unwrap();
    assert_eq!(reply.state.data["steps"], json!([{ "id": 1, "up": null, "n": 1 }, { "id": 2, "up": 1, "n": 1 }]));
}

#[test]
fn a_refused_move_answers_the_window_as_it_is() {
    let db = Db::memory().unwrap();
    let state = tracks();
    let mv: Move = serde_json::from_value(json!({ "row": 0, "key": "up" })).unwrap();
    let reply = reorder::event(&db, state.clone(), "list", "tracks", "rank", None, &mv, None, |_| panic!("no handler runs on a refused move"), drawn).unwrap();
    assert_eq!(reply.state.data, state.data);
}

#[test]
fn a_move_in_a_scope_is_an_edit_to_take_back() {
    let db = Db::memory().unwrap();
    let state = tracks();
    clarity_runtime::edits::replied(&drawn(state.clone()).unwrap());
    let mv: Move = serde_json::from_value(json!({ "row": 0, "key": "down" })).unwrap();
    let reply = reorder::event(&db, state.clone(), "list", "tracks", "rank", None, &mv, Some("list"), drawn, drawn).unwrap();
    assert_ne!(reply.state.data, state.data);
    let back = clarity_runtime::edits::undo(&db, reply.state, "list", true, drawn).unwrap();
    assert_eq!(back.state.data, state.data);
}
