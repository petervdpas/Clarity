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
        assert_eq!((ui.from_x("board"), ui.from_y("board")), (case["fromX"].as_i64().unwrap(), case["fromY"].as_i64().unwrap()), "{case}");
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

const STEPS: &str = include_str!("../../../conformance/steps.json");

#[test]
fn a_wizards_step_is_as_every_runtime_says() {
    use clarity_runtime::ui::{PartState, Ui, WindowState};
    let file: serde_json::Value = serde_json::from_str(STEPS).unwrap();
    for case in file["step"].as_array().unwrap() {
        let mut state = WindowState::default();
        state.parts.insert("wizard".into(), serde_json::from_value::<PartState>(case["part"].clone()).unwrap());
        assert_eq!(Ui::from_state(&state).step("wizard"), case["step"].as_i64().unwrap(), "{case}");
    }
    for case in file["set"].as_array().unwrap() {
        let mut ui = Ui::from_state(&WindowState::default());
        ui.set_step("wizard", case["step"].as_i64().unwrap());
        let reply = ui.reply("Main", serde_json::Map::new());
        assert_eq!(serde_json::to_value(&reply.state.parts["wizard"]).unwrap(), case["part"], "{case}");
    }
}

const ACCORDIONS: &str = include_str!("../../../conformance/accordions.json");

#[test]
fn an_accordions_open_section_is_as_every_runtime_says() {
    use clarity_runtime::ui::{PartState, Ui, WindowState};
    let file: serde_json::Value = serde_json::from_str(ACCORDIONS).unwrap();
    for case in file["open"].as_array().unwrap() {
        let mut state = WindowState::default();
        state.parts.insert("faq".into(), serde_json::from_value::<PartState>(case["part"].clone()).unwrap());
        assert_eq!(Ui::from_state(&state).open_section("faq"), case["open"].as_i64().unwrap(), "{case}");
    }
    for case in file["set"].as_array().unwrap() {
        let mut ui = Ui::from_state(&WindowState::default());
        ui.set_open_section("faq", case["open"].as_i64().unwrap());
        let reply = ui.reply("Main", serde_json::Map::new());
        assert_eq!(serde_json::to_value(&reply.state.parts["faq"]).unwrap(), case["part"], "{case}");
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

const CUSTOM_PARTS: &str = include_str!("../../../conformance/custom-parts.json");

#[test]
fn a_custom_parts_values_and_hears_are_as_every_runtime_says() {
    use clarity_runtime::ui::{PartState, Ui, WindowState};
    let file: serde_json::Value = serde_json::from_str(CUSTOM_PARTS).unwrap();
    for case in file["read"].as_array().unwrap() {
        let mut state = WindowState::default();
        state.parts.insert("arena".into(), serde_json::from_value::<PartState>(case["part"].clone()).unwrap());
        let read = Ui::from_state(&state).part_value("arena", case["name"].as_str().unwrap());
        assert_eq!(read.unwrap_or(serde_json::Value::Null), case["value"], "{case}");
    }
    for case in file["set"].as_array().unwrap() {
        let mut ui = Ui::from_state(&WindowState::default());
        for set in case["sets"].as_array().unwrap() {
            ui.set_part_value("arena", set[0].as_str().unwrap(), set[1].clone());
        }
        let reply = ui.reply("Main", serde_json::Map::new());
        assert_eq!(serde_json::to_value(&reply.state.parts["arena"]).unwrap(), case["part"], "{case}");
    }
    for case in file["hear"].as_array().unwrap() {
        let mut ui = Ui::from_state(&WindowState::default());
        for hear in case["hears"].as_array().unwrap() {
            ui.hear(hear[0].as_str().unwrap(), hear[1].as_str().unwrap(), hear[2].as_array().unwrap().clone());
        }
        let reply = serde_json::to_value(ui.reply("Main", serde_json::Map::new())).unwrap();
        assert_eq!(reply.get("hears").cloned().unwrap_or(serde_json::Value::Null), case["reply"], "{case}");
    }
}

/// `snapped(value, step)`: the multiple of step nearest to value, as a canvas's grid snaps (conformance/drawing.json, grids).
#[test]
fn snapped_rounds_to_its_step_as_every_runtime_does() {
    let file: serde_json::Value = serde_json::from_str(DRAWING).unwrap();
    for case in file["grids"]["snapped"].as_array().unwrap() {
        let (value, step) = (case["value"].as_i64().unwrap(), case["step"].as_i64().unwrap());
        assert_eq!(clarity_runtime::snapped(value, step), case["result"].as_i64().unwrap(), "{case}");
    }
}

/// A canvas's grid and snap are as its line says until code sets them.
#[test]
fn a_canvas_keeps_the_grid_and_snap_code_sets() {
    use clarity_runtime::ui::{Ui, WindowState};
    let mut ui = Ui::from_state(&WindowState::default());
    assert_eq!((ui.grid("sheet", 20), ui.snap("sheet", true)), (20, true));
    ui.set_grid("sheet", 0);
    ui.set_snap("sheet", false);
    assert_eq!((ui.grid("sheet", 20), ui.snap("sheet", true)), (0, false));
    assert_eq!(ui.background("sheet", "panel"), "panel");
    ui.set_background("sheet", "#f5f0e1".into());
    assert_eq!(ui.background("sheet", "panel"), "#f5f0e1");
    ui.set_dot_size("sheet", 3);
    ui.set_dot_color("sheet", "muted".into());
    assert_eq!((ui.dot_size("sheet", 2), ui.dot_color("sheet", "")), (3, "muted".to_string()));
}
