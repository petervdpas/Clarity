//! A program driven without a window (DESIGN 1.6): the steps a test sends,
//! a click, keys, typed text, a dialog's button by what it says, and what a
//! `read` answers of the window and the screen.

use super::core_windows_fake;

use std::sync::Arc;

use clarity_runtime::program::Program;
use clarity_ui::headless::{Driven, key_named};
use clarity_ui::Key;
use core_windows_fake::*;
use serde_json::{Value, json};

fn driven() -> (Driven, Arc<Fake>) {
    let fake = Arc::new(Fake::default());
    let program: Arc<dyn Program> = fake.clone();
    (Driven::start(program, FILES), fake)
}

fn ok(answer: Value) -> Value {
    assert_eq!(answer["ok"], json!(true), "{answer}");
    answer
}

#[test]
fn a_click_on_a_part_runs_its_handler() {
    let (mut d, fake) = driven();
    fake.calls();
    ok(d.step(&json!({ "do": "click", "part": "save" })));
    assert_eq!(fake.calls(), ["dispatch save"]);
    let read = ok(d.step(&json!({ "do": "read" })));
    assert_eq!(read["window"], json!("Main"));
    assert_eq!(read["windows"], json!(["Main"]));
    assert!(read["shown"]["save"].is_array(), "{read}");
    assert!(read["buttons"].as_array().unwrap().iter().any(|b| b["text"] == json!("Save")), "{read}");
}

#[test]
fn a_question_is_answered_by_its_buttons_text() {
    let (mut d, fake) = driven();
    ok(d.step(&json!({ "do": "click", "part": "ask" })));
    let read = ok(d.step(&json!({ "do": "read" })));
    assert_eq!(read["overlays"], json!(1), "{read}");
    fake.calls();
    ok(d.step(&json!({ "do": "click", "text": "Delete" })));
    assert_eq!(fake.calls(), [r#"answer confirmed [1]"#]);
}

#[test]
fn keys_and_a_window_in_place_and_back() {
    let (mut d, _fake) = driven();
    ok(d.step(&json!({ "do": "click", "part": "openConnection" })));
    assert_eq!(ok(d.step(&json!({ "do": "read" })))["windows"], json!(["Main", "Connection"]));
    // Escape presses its Cancel button, which closes it.
    ok(d.step(&json!({ "do": "key", "key": "Escape" })));
    assert_eq!(ok(d.step(&json!({ "do": "read" })))["windows"], json!(["Main"]));
    ok(d.step(&json!({ "do": "click", "part": "openConnection" })));
    ok(d.step(&json!({ "do": "click", "part": "saveConnection" })));
    let read = ok(d.step(&json!({ "do": "read" })));
    assert_eq!(read["windows"], json!(["Main"]));
    assert_eq!(read["toasts"], json!(["Saved"]));
    // A step it does not know, or a part that is not there, says so.
    assert_eq!(d.step(&json!({ "do": "fly" }))["ok"], json!(false));
    assert_eq!(d.step(&json!({ "do": "click", "part": "nothing" }))["ok"], json!(false));
}

#[test]
fn a_shot_is_a_png() {
    let (mut d, _fake) = driven();
    let path = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("headless-shot.png");
    ok(d.step(&json!({ "do": "shot", "path": path })));
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(&bytes[1..4], b"PNG");
    std::fs::remove_file(path).unwrap();
}

#[test]
fn keys_are_named_as_a_step_names_them() {
    assert_eq!(key_named("Enter"), Some(Key::Enter));
    assert_eq!(key_named("ArrowDown"), Some(Key::Down));
    assert_eq!(key_named("z"), Some(Key::Z));
    assert_eq!(key_named("7"), Some(Key::D7));
    assert_eq!(key_named("F2"), Some(Key::F2));
    assert_eq!(key_named("Nothing"), None);
}

#[test]
fn a_frame_button_is_clicked_by_its_name() {
    const APP: &str = r#"<App name="Orders"><Toolbar name="mainToolbar"><Item kind="button" caption="Connect" name="connectButton" opens="Connection" /></Toolbar></App>"#;
    let files = clarity_ui::AppFiles { apps: &[("Orders", APP)], ..FILES };
    let fake = Arc::new(Fake::default());
    let program: Arc<dyn Program> = fake.clone();
    let mut d = Driven::start(program, files);
    let read = ok(d.step(&json!({ "do": "read" })));
    assert!(read["shown"]["connectButton"].is_array(), "{read}");
    ok(d.step(&json!({ "do": "click", "part": "connectButton" })));
    assert_eq!(ok(d.step(&json!({ "do": "read" })))["windows"], json!(["Main", "Connection"]));
}

#[test]
fn a_part_takes_the_focus_and_the_top_overlay_names_its_parts() {
    let (mut d, _fake) = driven();
    assert_eq!(ok(d.step(&json!({ "do": "read" })))["top"], json!([]));
    ok(d.step(&json!({ "do": "focus", "part": "warn" })));
    assert_eq!(ok(d.step(&json!({ "do": "read" })))["focused"], json!("warn"));
    ok(d.step(&json!({ "do": "click", "part": "openPick" })));
    let read = ok(d.step(&json!({ "do": "read" })));
    let top = read["top"].as_array().unwrap();
    assert!(top.contains(&json!("choose")) && !top.contains(&json!("save")), "{read}");
    ok(d.step(&json!({ "do": "focus", "part": "cancelPick" })));
    assert_eq!(ok(d.step(&json!({ "do": "read" })))["focused"], json!("cancelPick"));
    assert_eq!(d.step(&json!({ "do": "focus", "part": "nothing" }))["ok"], json!(false));
}

#[test]
fn a_read_gives_the_rows_and_headers_of_a_table() {
    const MAIN: &str = r#"<Window name="Main">
  <Table name="grid" at="1,1" span="12" rows="6" bind="items" columns="name, size" />
  <List name="names" at="1,7" span="12" rows="3" bind="items" show="name" />
</Window>"#;
    let files = clarity_ui::AppFiles { windows: &[("Main", MAIN)], ..FILES };
    let fake = Arc::new(Fake::default());
    let program: Arc<dyn Program> = fake.clone();
    let mut d = Driven::start(program, files);
    let read = ok(d.step(&json!({ "do": "read" })));
    assert_eq!(read["rows"]["grid"], json!([]), "{read}");
    assert_eq!(read["rows"]["names"], json!([]), "{read}");
    assert!(read["rows"]["grid.header"]["name"].is_array(), "{read}");
    assert!(read["rows"]["grid.header"]["size"].is_array(), "{read}");
}

#[test]
fn a_part_below_the_screen_is_scrolled_into_sight_and_clicked() {
    let (mut d, _fake) = driven();
    ok(d.step(&json!({ "do": "size", "width": 800, "height": 160 })));
    let read = ok(d.step(&json!({ "do": "read" })));
    assert!(read["shown"]["openConnection"][1].as_f64().unwrap() + 30.0 > 160.0, "{read}");
    ok(d.step(&json!({ "do": "click", "part": "openConnection" })));
    assert_eq!(ok(d.step(&json!({ "do": "read" })))["windows"], json!(["Main", "Connection"]));
}

#[test]
fn the_items_of_a_split_buttons_menu_are_buttons_by_what_they_say() {
    const MAIN: &str = r#"<Window name="Main">
  <Button name="saveButton" at="1,1" span="4" caption="Save" split="true" onclick="save">
    <Menu>
      <Item caption="Save a copy" name="saveCopyItem" onclick="saveCopy" />
    </Menu>
  </Button>
</Window>"#;
    let files = clarity_ui::AppFiles { windows: &[("Main", MAIN)], ..FILES };
    let fake = Arc::new(Fake::default());
    let program: Arc<dyn Program> = fake.clone();
    let mut d = Driven::start(program, files);
    let at = ok(d.step(&json!({ "do": "read" })))["shown"]["saveButton"].clone();
    let (x, y, w, h) = (at[0].as_f64().unwrap(), at[1].as_f64().unwrap(), at[2].as_f64().unwrap(), at[3].as_f64().unwrap());
    ok(d.step(&json!({ "do": "click", "x": x + w - 8.0, "y": y + h / 2.0 })));
    let read = ok(d.step(&json!({ "do": "read" })));
    assert!(read["buttons"].as_array().unwrap().iter().any(|b| b["text"] == json!("Save a copy")), "{read}");
    fake.calls();
    ok(d.step(&json!({ "do": "click", "text": "Save a copy" })));
    assert_eq!(fake.calls(), ["dispatch saveCopy"]);
}

#[test]
fn a_part_is_scrolled_into_sight_on_its_own() {
    let (mut d, _fake) = driven();
    ok(d.step(&json!({ "do": "size", "width": 800, "height": 160 })));
    let before = ok(d.step(&json!({ "do": "read" })))["shown"]["openConnection"][1].as_f64().unwrap();
    let at = ok(d.step(&json!({ "do": "show", "part": "openConnection" })))["at"].clone();
    let y = at[1].as_f64().unwrap();
    assert!(y < before && y + at[3].as_f64().unwrap() <= 160.0, "{before} {at}");
    assert_eq!(d.step(&json!({ "do": "show", "part": "nothing" }))["ok"], json!(false));
}

#[test]
fn a_drawings_tools_are_buttons_and_its_units_map_onto_the_screen() {
    const MAIN: &str = r#"<Window name="Main">
  <Canvas name="drawing" at="1,1" span="12" rows="4" bind="notes" mode="vector" units="{&quot;height&quot;:100,&quot;width&quot;:200}" />
</Window>"#;
    let files = clarity_ui::AppFiles { windows: &[("Main", MAIN)], ..FILES };
    let fake = Arc::new(Fake::default());
    let program: Arc<dyn Program> = fake.clone();
    let mut d = Driven::start(program, files);
    let read = ok(d.step(&json!({ "do": "read" })));
    assert!(read["buttons"].as_array().unwrap().iter().any(|b| b["text"] == json!("Rectangle")), "{read}");
    let canvas = &read["canvases"]["drawing"];
    assert!(canvas["scale"].as_f64().unwrap() > 0.0, "{read}");
    assert_eq!(canvas["at"], read["shown"]["drawing"], "{read}");
}
