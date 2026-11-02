//! Undo of typed inputs in a real window (DESIGN 5.29), as a person types:
//! typing taken when a text box is left is the window's edit, so Ctrl+Z
//! there walks back the form, while typing not yet taken is the box's own
//! to take back first; an input that wraps its text box (a number, a color,
//! a typed dropdown) records what was typed when it is left; a slider moved
//! with a key and a setting changed by a part are taken back too. Each part
//! carries the undoScope the generator writes.
//!
//! The record of edits is the program's (DESIGN 5.29: the runtime holds
//! it), so the program of these tests keeps one with the runtime's own
//! edit bus, as a generated program does.

use super::inputs_program as program;

use std::sync::Arc;

use clarity_ui::element::{self, ElementId};
use clarity_ui::input::{Key, KeyInput, Modifiers};
use clarity_ui::parts::text_box::TextBoxPart;
use clarity_ui::window::WindowStack;
use clarity_ui::Screen;
use program::Typed;
use serde_json::{json, Map, Value};

/// The window with `parts` under a button elsewhere, shown on a screen.
fn shown(parts: &str) -> (Screen, Arc<Typed>) {
    let mut data = Map::new();
    data.insert("client".into(), json!({"name": "Ada", "city": "Rome", "tint": "#3b82f6", "rating": 0}));
    data.insert("settings".into(), json!({"showToolbar": true}));
    program::shown("Typed", &format!(r#"<Button name="elsewhere" at="1,1" span="2" caption="Elsewhere" />{parts}"#), program::MODEL, data)
}

/// The window's data now, at a path like `client.name`.
fn data(screen: &Screen, path: &str) -> Value {
    let stack = screen.root().as_any().downcast_ref::<WindowStack>().unwrap();
    let state = stack.current().unwrap().state_now();
    path.split('.').fold(Value::Object(state.data), |v, name| v.get(name).cloned().unwrap_or(Value::Null))
}

/// The same number, written as a number or as text.
fn number(value: &Value) -> f64 {
    match value {
        Value::String(s) => s.parse().unwrap_or(f64::NAN),
        other => other.as_f64().unwrap_or(f64::NAN),
    }
}

fn named(screen: &Screen, name: &str) -> ElementId {
    element::in_order(screen.root()).into_iter().find(|e| e.as_part().and_then(|p| p.base().spec.name()) == Some(name)).map(|e| e.node().id()).unwrap_or_else(|| panic!("no part named {name}"))
}

/// The window takes what the person did: the edit that is not sent yet goes to the program.
fn tick(screen: &mut Screen) {
    screen.act_root::<WindowStack, _>(|stack, _| {
        if let Some(host) = stack.current() {
            host.flush();
        }
    });
}

fn press(screen: &mut Screen, key: Key, modifiers: Modifiers) {
    screen.key_pressed(KeyInput::new(key, modifiers));
}

/// Types `text` over what the input has, then moves the focus elsewhere.
fn type_and_leave(screen: &mut Screen, name: &str, text: &str) {
    let id = named(screen, name);
    screen.focus(Some(id));
    press(screen, Key::A, Modifiers::CONTROL);
    screen.text_typed(text);
    let elsewhere = named(screen, "elsewhere");
    screen.focus(Some(elsewhere));
    tick(screen);
}

fn undo_from(screen: &mut Screen, name: &str) {
    let id = named(screen, name);
    screen.focus(Some(id));
    tick(screen);
    press(screen, Key::Z, Modifiers::CONTROL);
}

fn text_of(screen: &Screen, name: &str) -> String {
    screen.element(named(screen, name)).and_then(|e| e.as_any().downcast_ref::<TextBoxPart>()).unwrap().editor().text()
}

const FORM: &str = r#"<Textbox name="nameBox" at="1,2" span="6" bind="client.name" undoScope="customerForm" />
    <Textbox name="cityBox" at="1,3" span="6" bind="client.city" undoScope="customerForm" />"#;

#[test]
fn typing_taken_when_a_box_is_left_is_the_forms_to_take_back() {
    let (mut screen, _) = shown(FORM);
    type_and_leave(&mut screen, "nameBox", "Ann");
    type_and_leave(&mut screen, "cityBox", "Oslo");
    undo_from(&mut screen, "nameBox");
    assert_eq!((json!("Ann"), json!("Rome")), (data(&screen, "client.name"), data(&screen, "client.city")));
    press(&mut screen, Key::Z, Modifiers::CONTROL);
    assert_eq!(json!("Ada"), data(&screen, "client.name"));
}

/// Ctrl+Z while typing takes back the box's own typing, a word at a time,
/// before any edit of the form; Ctrl+Y does it again.
#[test]
fn typing_not_yet_taken_is_the_boxs_own_to_take_back_first() {
    let (mut screen, _) = shown(FORM);
    type_and_leave(&mut screen, "cityBox", "Oslo");
    let name = named(&screen, "nameBox");
    screen.focus(Some(name));
    tick(&mut screen);
    press(&mut screen, Key::End, Modifiers::NONE);
    screen.text_typed(" Lovelace");
    assert_eq!("Ada Lovelace", text_of(&screen, "nameBox"));
    press(&mut screen, Key::Z, Modifiers::CONTROL);
    assert_eq!("Ada", text_of(&screen, "nameBox"));
    assert_eq!(json!("Oslo"), data(&screen, "client.city"));
    press(&mut screen, Key::Y, Modifiers::CONTROL);
    assert_eq!("Ada Lovelace", text_of(&screen, "nameBox"));
    press(&mut screen, Key::Z, Modifiers::CONTROL);
    // No typing of its own left: the next one is the form's.
    press(&mut screen, Key::Z, Modifiers::CONTROL);
    assert_eq!(json!("Rome"), data(&screen, "client.city"));
    assert_eq!(json!("Ada"), data(&screen, "client.name"));
}

/// `no undo`: nothing, not even the typing.
#[test]
fn a_box_with_no_undo_takes_back_not_even_its_typing() {
    let (mut screen, program) = shown(r#"<Textbox name="nameBox" at="1,2" span="6" bind="client.name" noUndo="true" />"#);
    let name = named(&screen, "nameBox");
    screen.focus(Some(name));
    press(&mut screen, Key::End, Modifiers::NONE);
    screen.text_typed("x");
    press(&mut screen, Key::Z, Modifiers::CONTROL);
    assert_eq!("Adax", text_of(&screen, "nameBox"));
    let elsewhere = named(&screen, "elsewhere");
    screen.focus(Some(elsewhere));
    tick(&mut screen);
    // Its edit goes on no scope: the program keeps none of it.
    assert!(program.edits.lock().unwrap().iter().all(|e| !e.starts_with("nameBox")));
}

/// Two parts show the same field: what the person typed in one is that part's edit, not the other's, which only follows.
#[test]
fn a_part_that_shows_the_same_data_does_not_take_the_edit() {
    let (mut screen, program) = shown(
        r#"<Textbox name="nameBox" at="1,2" span="6" bind="client.name" undoScope="customerForm" />
        <Textbox name="nameShown" at="7,2" span="6" bind="client.name" />
        <Textbox name="cityBox" at="1,3" span="6" bind="client.city" undoScope="customerForm" />"#,
    );
    type_and_leave(&mut screen, "nameBox", "Ann");
    type_and_leave(&mut screen, "cityBox", "Oslo");
    clarity_ui::testing::Shot::of(&mut screen, 800, 400, 1.0);
    assert_eq!("Ann", text_of(&screen, "nameShown"));
    undo_from(&mut screen, "nameBox");
    assert_eq!((json!("Ann"), json!("Rome")), (data(&screen, "client.name"), data(&screen, "client.city")));
    press(&mut screen, Key::Z, Modifiers::CONTROL);
    assert_eq!(json!("Ada"), data(&screen, "client.name"));
    assert!(program.edits.lock().unwrap().iter().all(|e| !e.starts_with("nameShown")));
}

/// Every input's editor is inside its part, which the window finds by walking up.
#[test]
fn an_input_that_wraps_its_text_box_records_what_was_typed_when_it_is_left() {
    let (mut screen, program) = shown(r#"<Number name="ratingBox" undoScope="ratingBox" at="1,2" span="6" bind="client.rating" />"#);
    type_and_leave(&mut screen, "ratingBox", "4");
    assert_eq!(4.0, number(&data(&screen, "client.rating")));
    assert_eq!(vec!["ratingBox ratingBox"], program.edits.lock().unwrap().clone());
    undo_from(&mut screen, "ratingBox");
    assert_eq!(0.0, number(&data(&screen, "client.rating")));
}

#[test]
fn a_color_typed_and_left_is_taken_back() {
    let (mut screen, _) = shown(r#"<Color name="tintBox" undoScope="tintBox" at="1,2" span="6" bind="client.tint" />"#);
    type_and_leave(&mut screen, "tintBox", "#ff0000");
    assert_eq!(json!("#ff0000"), data(&screen, "client.tint"));
    undo_from(&mut screen, "tintBox");
    assert_eq!(json!("#3b82f6"), data(&screen, "client.tint"));
}

#[test]
fn a_typed_dropdown_typed_and_left_is_taken_back() {
    let (mut screen, _) = shown(r#"<Dropdown name="cityBox" undoScope="cityBox" at="1,2" span="6" bind="client.city" typed="true" />"#);
    type_and_leave(&mut screen, "cityBox", "Oslo");
    assert_eq!(json!("Oslo"), data(&screen, "client.city"));
    undo_from(&mut screen, "cityBox");
    assert_eq!(json!("Rome"), data(&screen, "client.city"));
}

#[test]
fn a_slider_moved_with_a_key_is_taken_back() {
    let (mut screen, _) = shown(r#"<Slider name="ratingSlider" undoScope="ratingSlider" at="1,2" span="6" bind="client.rating" min="0" max="5" step="0.5" />"#);
    let slider = named(&screen, "ratingSlider");
    screen.focus(Some(slider));
    tick(&mut screen);
    press(&mut screen, Key::Right, Modifiers::NONE);
    assert_eq!(0.5, number(&data(&screen, "client.rating")));
    press(&mut screen, Key::Z, Modifiers::CONTROL);
    assert_eq!(0.0, number(&data(&screen, "client.rating")));
}

#[test]
fn a_setting_a_part_changed_is_taken_back() {
    let (mut screen, _) = shown(r#"<Checkbox name="toolbarSwitch" undoScope="toolbarSwitch" at="1,2" span="6" bind="settings.showToolbar" caption="Toolbar" />"#);
    let check = named(&screen, "toolbarSwitch");
    screen.focus(Some(check));
    tick(&mut screen);
    press(&mut screen, Key::Space, Modifiers::NONE);
    assert_eq!(json!(false), data(&screen, "settings.showToolbar"));
    press(&mut screen, Key::Z, Modifiers::CONTROL);
    assert_eq!(json!(true), data(&screen, "settings.showToolbar"));
}
