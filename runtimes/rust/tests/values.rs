//! Values in code as every runtime computes them (conformance/rounding.json).

use clarity_runtime::{Decimal, round};
use std::str::FromStr;

const CASES: &str = include_str!("../../../conformance/rounding.json");

#[test]
fn every_rounding_case_rounds_as_every_runtime_does() {
    let file: serde_json::Value = serde_json::from_str(CASES).unwrap();
    let cases = file["cases"].as_array().unwrap();
    assert!(!cases.is_empty());
    for case in cases {
        let value = Decimal::from_str(case["value"].as_str().unwrap()).unwrap();
        let places = case["places"].as_i64().unwrap();
        let gives = Decimal::from_str(case["gives"].as_str().unwrap()).unwrap();
        assert_eq!(round(value, places), gives, "{case}");
    }
}

const DRAWING: &str = include_str!("../../../conformance/drawing.json");

#[test]
fn sin_and_cos_take_degrees_as_every_runtime_does() {
    let file: serde_json::Value = serde_json::from_str(DRAWING).unwrap();
    for case in file["angles"].as_array().unwrap() {
        let degrees = Decimal::from_str(case["degrees"].as_str().unwrap()).unwrap();
        let sin = Decimal::from_str(case["sin"].as_str().unwrap()).unwrap();
        let cos = Decimal::from_str(case["cos"].as_str().unwrap()).unwrap();
        assert_eq!((clarity_runtime::sin(degrees), clarity_runtime::cos(degrees)), (sin, cos), "{case}");
    }
}

#[test]
fn a_draw_starts_an_empty_drawing_that_travels_with_the_part() {
    use clarity_runtime::ui::{Ui, WindowState};
    let mut ui = Ui::from_state(&WindowState::default());
    ui.begin_drawing("dial");
    ui.draw("dial", serde_json::json!({"op": "clear", "color": "#101810"}));
    ui.draw("dial", serde_json::json!({"op": "circle", "x": 50.0, "y": 50.0, "r": clarity_runtime::units(Decimal::new(405, 1)), "color": "accent"}));
    let reply = ui.reply("Main", serde_json::Map::new());
    let drawing = serde_json::to_value(&reply.state.parts["dial"]).unwrap()["drawing"].clone();
    assert_eq!(drawing, serde_json::json!([{"op": "clear", "color": "#101810"}, {"op": "circle", "x": 50.0, "y": 50.0, "r": 40.5, "color": "accent"}]));
    // The next draw starts anew.
    let mut ui = Ui::from_state(&reply.state);
    ui.begin_drawing("dial");
    let reply = ui.reply("Main", serde_json::Map::new());
    assert_eq!(serde_json::to_value(&reply.state.parts["dial"]).unwrap()["drawing"], serde_json::json!([]));
}
