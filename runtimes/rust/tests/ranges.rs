//! The slider and the spin box (U1c) against the cases every runtime shares
//! (conformance/ranges.json), and the value `range_event` writes.

use clarity_runtime::ranges::{self, Range, RangeMove};
use clarity_runtime::ui::{Reply, Ui, WindowState};
use clarity_runtime::{Db, Result};
use serde_json::{Map, Value, json};

const CASES: &str = include_str!("../../../conformance/ranges.json");

fn cases(part: &str) -> Vec<Value> {
    let all: Value = serde_json::from_str(CASES).unwrap();
    all[part].as_array().unwrap().clone()
}

fn text(value: &Value) -> Option<String> {
    value.as_str().map(str::to_string)
}

fn range(case: &Value) -> Range {
    Range { min: text(&case["min"]), max: text(&case["max"]), step: text(&case["step"]).unwrap_or_else(|| "1".into()) }
}

#[test]
fn a_key_moves_the_value_within_its_range() {
    for case in cases("keys") {
        let key = serde_json::from_value(case["key"].clone()).unwrap();
        let after = ranges::keyed(text(&case["value"]).as_deref(), &range(&case), key);
        assert_eq!(after.as_deref(), case["after"].as_str(), "{case}");
    }
}

#[test]
fn a_drag_takes_the_nearest_step() {
    for case in cases("drag") {
        let value = ranges::dragged(case["at"].as_str().unwrap(), &range(&case));
        assert_eq!(value.as_deref(), case["value"].as_str(), "{case}");
    }
}

#[test]
fn a_typed_number_stays_within_the_range() {
    for case in cases("typed") {
        let after = ranges::typed_in(case["value"].as_str().unwrap(), &range(&case));
        assert_eq!(after.as_deref(), case["after"].as_str(), "{case}");
    }
}

#[test]
fn text_that_is_no_number_gives_nothing() {
    let range = Range { min: Some("0".into()), max: Some("10".into()), step: "1".into() };
    assert_eq!(ranges::typed_in("five", &range), None);
    assert_eq!(ranges::dragged("", &range), None);
    assert_eq!(ranges::dragged("3", &Range { step: "0".into(), ..range.clone() }), Some("3".into()), "no step: the place, within the range");
}

fn drawn(state: WindowState) -> Result<Reply> {
    let data = state.data.clone();
    Ok(Ui::from_state(&state).reply(&state.window, data))
}

fn album() -> WindowState {
    let mut data = Map::new();
    data.insert("album".into(), json!({ "id": 1, "rating": 5, "price": "9.5" }));
    WindowState::new("Album", data)
}

#[test]
fn a_whole_value_is_written_as_a_number_and_a_decimal_as_text() {
    let db = Db::memory().unwrap();
    let range = Range { min: Some("0".into()), max: Some("10".into()), step: "1".into() };
    let up = RangeMove::Key(ranges::RangeKey::Up);
    let reply = ranges::event(&db, album(), "stars", "album.rating", &range, true, &up, None, drawn, drawn).unwrap();
    assert_eq!(reply.state.data["album"]["rating"], json!(6));
    let range = Range { min: Some("0".into()), max: Some("20".into()), step: "0.25".into() };
    let reply = ranges::event(&db, album(), "price", "album.price", &range, false, &RangeMove::Typed("12.125".into()), None, drawn, drawn).unwrap();
    assert_eq!(reply.state.data["album"]["price"], json!("12.125"));
}

#[test]
fn a_value_that_stays_runs_no_handler() {
    let db = Db::memory().unwrap();
    let range = Range { min: Some("0".into()), max: Some("5".into()), step: "1".into() };
    let up = RangeMove::Key(ranges::RangeKey::Up);
    let reply = ranges::event(&db, album(), "stars", "album.rating", &range, true, &up, None, |_| panic!("nothing changed"), drawn).unwrap();
    assert_eq!(reply.state.data["album"]["rating"], json!(5));
}
