//! A wizard (DESIGN 5.15) against the cases every runtime shares
//! (conformance/steps.json): `step_event` walks it as the person presses
//! Next, Back and Finish.

use clarity_runtime::steps::{self, StepButton};
use clarity_runtime::ui::{PartState, Reply, Ui, WindowState};
use clarity_runtime::{Db, Error, Result};
use serde_json::{Map, Value, json};

const CASES: &str = include_str!("../../../conformance/steps.json");

fn cases(part: &str) -> Vec<Value> {
    let all: Value = serde_json::from_str(CASES).unwrap();
    all[part].as_array().unwrap().clone()
}

fn drawn(state: WindowState) -> Result<Reply> {
    let data = state.data.clone();
    Ok(Ui::from_state(&state).reply(&state.window, data))
}

#[test]
fn a_wizard_reads_its_step_from_the_part() {
    for case in cases("step") {
        let part: PartState = serde_json::from_value(case["part"].clone()).unwrap();
        let mut state = WindowState::new("Order", Map::new());
        state.parts.insert("wizard".into(), part);
        assert_eq!(Ui::from_state(&state).step("wizard"), case["step"].as_i64().unwrap(), "{case}");
    }
    for case in cases("set") {
        let mut ui = Ui::from_state(&WindowState::new("Order", Map::new()));
        ui.set_step("wizard", case["step"].as_i64().unwrap());
        let reply = ui.reply("Order", Map::new());
        assert_eq!(serde_json::to_value(&reply.state.parts["wizard"]).unwrap(), case["part"], "{case}");
    }
}

#[test]
fn a_wizard_walks_as_every_runtime_does() {
    let db = Db::memory().unwrap();
    for case in cases("walk") {
        let name = case["name"].as_str().unwrap();
        let count = case["steps"].as_i64().unwrap();
        let handler = &case["handler"];
        let wired = (!handler.is_null()).then_some(());
        let mut state = WindowState::new("Order", Map::new());
        if let Some(start) = case["start"].as_i64() {
            state.parts.entry("wizard".into()).or_default().step = Some(start);
        }
        let mut ran: Vec<String> = vec![];
        for button in case["do"].as_array().unwrap() {
            let button: StepButton = serde_json::from_value(button.clone()).unwrap();
            let run = |state: WindowState, which: &str| -> Result<Reply> {
                let on = Ui::from_state(&state).step("wizard");
                ran.push(format!("{} {on}", if which == "checkStep" { "next" } else { "finish" }));
                if handler["refuses"].as_array().is_some_and(|r| r.contains(&json!(on))) {
                    return Err(Error::Refused(clarity_runtime::ui::Text::plain("not yet")));
                }
                let mut ui = Ui::from_state(&state);
                if handler["jumps"]["from"].as_i64() == Some(on) {
                    ui.set_step("wizard", handler["jumps"]["to"].as_i64().unwrap());
                }
                Ok(ui.reply(&state.window, state.data.clone()))
            };
            let (next, finish) = (wired.map(|_| "checkStep"), wired.map(|_| "done"));
            if let Ok(reply) = steps::event(&db, state.clone(), "wizard", button, count, next, finish, run, drawn) {
                state = reply.state;
            }
        }
        let shown = steps::shown(state.parts.get("wizard").and_then(|p| p.step), count);
        let expect = &case["expect"];
        assert_eq!(json!(shown), expect["shown"], "{name}: shown");
        assert_eq!(json!(steps::buttons(shown, count)), expect["buttons"], "{name}: buttons");
        assert_eq!(json!(ran), expect["ran"], "{name}: ran");
    }
}
