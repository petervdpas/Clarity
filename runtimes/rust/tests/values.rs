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

#[test]
fn a_canvas_says_where_it_was_clicked_as_every_runtime_does() {
    use clarity_runtime::ui::{PartState, Ui, WindowState};
    let file: serde_json::Value = serde_json::from_str(DRAWING).unwrap();
    for case in file["points"].as_array().unwrap() {
        let part: PartState = serde_json::from_value(case["part"].clone()).unwrap();
        let mut state = WindowState::default();
        state.parts.insert("board".into(), part);
        let ui = Ui::from_state(&state);
        assert_eq!((ui.x("board"), ui.y("board")), (case["x"].as_i64().unwrap(), case["y"].as_i64().unwrap()), "{case}");
        // What the pointer holds besides x and y travels back unchanged.
        let reply = ui.reply("Main", serde_json::Map::new());
        assert_eq!(serde_json::to_value(&reply.state.parts["board"]).unwrap(), case["part"], "{case}");
    }
}

const FOLDING: &str = include_str!("../../../conformance/folding.json");

#[test]
fn a_foldable_part_is_folded_as_every_runtime_says() {
    use clarity_runtime::ui::{PartState, Ui, WindowState};
    let file: serde_json::Value = serde_json::from_str(FOLDING).unwrap();
    for case in file["cases"].as_array().unwrap() {
        let mut state = WindowState::default();
        state.parts.insert("details".into(), serde_json::from_value::<PartState>(case["part"].clone()).unwrap());
        let ui = Ui::from_state(&state);
        assert_eq!(ui.folded("details", case["design"].as_bool().unwrap()), case["folded"].as_bool().unwrap(), "{case}");
    }
    for case in file["set"].as_array().unwrap() {
        let mut ui = Ui::from_state(&WindowState::default());
        ui.set_folded("details", case["folded"].as_bool().unwrap());
        let reply = ui.reply("Main", serde_json::Map::new());
        assert_eq!(serde_json::to_value(&reply.state.parts["details"]).unwrap(), case["part"], "{case}");
    }
}

const SIDEBARS: &str = include_str!("../../../conformance/sidebars.json");

#[test]
fn a_sidebar_is_collapsed_and_maximized_as_every_runtime_says() {
    use clarity_runtime::ui::{PartState, Ui, WindowState};
    let file: serde_json::Value = serde_json::from_str(SIDEBARS).unwrap();
    let ui_with = |part: &serde_json::Value| {
        let mut state = WindowState::default();
        state.parts.insert("nav".into(), serde_json::from_value::<PartState>(part.clone()).unwrap());
        Ui::from_state(&state)
    };
    for case in file["collapsed"].as_array().unwrap() {
        assert_eq!(ui_with(&case["part"]).collapsed("nav", case["design"].as_bool().unwrap()), case["collapsed"].as_bool().unwrap(), "{case}");
    }
    for case in file["maximized"].as_array().unwrap() {
        assert_eq!(ui_with(&case["part"]).maximized("nav"), case["maximized"].as_bool().unwrap(), "{case}");
    }
    for case in file["set"].as_array().unwrap() {
        let mut ui = Ui::from_state(&WindowState::default());
        if let Some(collapsed) = case["collapsed"].as_bool() {
            ui.set_collapsed("nav", collapsed);
        }
        if let Some(maximized) = case["maximized"].as_bool() {
            ui.set_maximized("nav", maximized);
        }
        let reply = ui.reply("Main", serde_json::Map::new());
        assert_eq!(serde_json::to_value(&reply.state.parts["nav"]).unwrap(), case["part"], "{case}");
    }
}

const RIBBONS: &str = include_str!("../../../conformance/ribbons.json");

#[test]
fn a_ribbons_open_tab_is_as_every_runtime_says() {
    use clarity_runtime::ui::{PartState, Ui, WindowState};
    let file: serde_json::Value = serde_json::from_str(RIBBONS).unwrap();
    for case in file["tab"].as_array().unwrap() {
        let mut state = WindowState::default();
        state.parts.insert("mainRibbon".into(), serde_json::from_value::<PartState>(case["part"].clone()).unwrap());
        assert_eq!(Ui::from_state(&state).tab("mainRibbon"), case["tab"].as_i64().unwrap(), "{case}");
    }
    for case in file["set"].as_array().unwrap() {
        let mut ui = Ui::from_state(&WindowState::default());
        ui.set_tab("mainRibbon", case["tab"].as_i64().unwrap());
        let reply = ui.reply("Main", serde_json::Map::new());
        assert_eq!(serde_json::to_value(&reply.state.parts["mainRibbon"]).unwrap(), case["part"], "{case}");
    }
}
