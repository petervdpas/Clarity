//! A record keeps the keys no part binds when a person edits it in a window
//! (docs/APP-CAPABILITIES-PLAN.md step 5): every stack sends a record's
//! version as `rowVersion` beside its fields, and a save is refused when the
//! version that comes back is not the one read. So typing in a bound box
//! must leave `rowVersion` (and `id`) as the program sent them.

use super::inputs_program as program;

use clarity_ui::element::{self, ElementId};
use clarity_ui::input::{Key, KeyInput, Modifiers};
use clarity_ui::window::WindowStack;
use clarity_ui::Screen;
use serde_json::{json, Map, Value};

fn named(screen: &Screen, name: &str) -> ElementId {
    element::in_order(screen.root()).into_iter().find(|e| e.as_part().and_then(|p| p.base().spec.name()) == Some(name)).map(|e| e.node().id()).unwrap_or_else(|| panic!("no part named {name}"))
}

/// The window's record `client` as it would go back to the program now.
fn client(screen: &Screen) -> Value {
    let stack = screen.root().as_any().downcast_ref::<WindowStack>().unwrap();
    let state = stack.current().unwrap().state_now();
    state.data.get("client").cloned().unwrap_or(Value::Null)
}

#[test]
fn typing_in_a_bound_box_keeps_the_record_s_version_and_id() {
    let mut data = Map::new();
    data.insert("client".into(), json!({"id": 7, "name": "Ada", "city": "Rome", "tint": "#3b82f6", "rating": 0, "rowVersion": 3}));
    data.insert("settings".into(), json!({"showToolbar": true}));
    let parts = r#"<Button name="elsewhere" at="1,1" span="2" caption="Elsewhere" />
    <Textbox name="nameBox" at="1,2" span="6" bind="client.name" />"#;
    let (mut screen, _) = program::shown("Typed", parts, program::MODEL, data);
    let id = named(&screen, "nameBox");
    screen.focus(Some(id));
    screen.key_pressed(KeyInput::new(Key::A, Modifiers::CONTROL));
    screen.text_typed("Ann");
    let elsewhere = named(&screen, "elsewhere");
    screen.focus(Some(elsewhere));
    screen.act_root::<WindowStack, _>(|stack, _| {
        if let Some(host) = stack.current() {
            host.flush();
        }
    });
    let record = client(&screen);
    assert_eq!(record["name"], json!("Ann"), "{record}");
    assert_eq!((record["rowVersion"].clone(), record["id"].clone()), (json!(3), json!(7)), "{record}");
}
