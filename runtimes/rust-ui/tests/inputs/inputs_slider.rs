//! The slider against the cases every renderer shares (conformance/ranges.json):
//! its keys move a step, ten or to an end; let go, it takes the nearest step.
//! Only a value that differs is a change; a drag writes nothing until it is
//! let go; unbound it keeps its value in its state's text.

use std::rc::Rc;

use clarity_ui::input::{Key, KeyInput, PointerButton};
use clarity_ui::parts::inputs::slider::SliderPart;
use clarity_ui::parts::inputs::testing::InputsHost;
use clarity_ui::parts::PartHost;
use clarity_ui::testing::{near, part_named, window_of, Shot};
use clarity_ui::{Element, Screen};
use serde_json::{json, Value};

fn cases() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../conformance/ranges.json");
    serde_json::from_str(&std::fs::read_to_string(path).expect("conformance/ranges.json")).expect("it is JSON")
}

fn text(case: &Value, name: &str) -> Option<String> {
    case[name].as_str().map(str::to_string)
}

fn window(markup: &str, host: Rc<InputsHost>) -> Screen {
    let mut screen = window_of(markup, host);
    Shot::of(&mut screen, 800, 400, 1.0);
    screen
}

fn slider<'a>(screen: &'a Screen) -> &'a SliderPart {
    screen.element(part_named(screen, "level")).and_then(|e| e.as_any().downcast_ref::<SliderPart>()).expect("a slider")
}

fn slider_of(case: &Value, value: Option<String>) -> (Rc<InputsHost>, Screen) {
    let host = InputsHost::new();
    let whole = case["whole"].as_bool().unwrap();
    host.set(
        "v",
        match value {
            Some(v) if whole => json!(v.parse::<i64>().unwrap()),
            Some(v) => json!(v),
            None => Value::Null,
        },
    );
    let range: String = ["min", "max", "step"].iter().filter_map(|n| text(case, n).map(|v| format!(" {n}=\"{v}\""))).collect();
    let screen = window(&format!(r#"<Slider at="1,1" span="6" name="level" bind="v"{range} onchange="moved" />"#), host.clone());
    (host, screen)
}

fn same(a: &str, b: &str) -> bool {
    a.parse::<f64>().unwrap() == b.parse::<f64>().unwrap()
}

#[test]
fn a_key_moves_it_as_on_every_renderer() {
    let all = cases();
    let mut run = 0;
    for case in all["keys"].as_array().unwrap().iter().filter(|c| c["min"].is_string() && c["max"].is_string()) {
        run += 1;
        let (host, mut screen) = slider_of(case, text(case, "value"));
        let key = match case["key"].as_str().unwrap() {
            "up" => Key::Up,
            "down" => Key::Down,
            "pageUp" => Key::PageUp,
            "pageDown" => Key::PageDown,
            "home" => Key::Home,
            _ => Key::End,
        };
        screen.focus(Some(part_named(&screen, "level")));
        screen.key_pressed(KeyInput::plain(key));
        let after = text(case, "after").unwrap();
        assert!(same(&after, &slider(&screen).value().unwrap()), "{case}");
        assert_eq!(case["whole"].as_bool().unwrap(), host.read("v").is_some_and(|v| v.is_i64()), "{case}");
        let changed = text(case, "value").is_none_or(|before| !same(&before, &after));
        assert_eq!(usize::from(changed), host.logged().iter().filter(|l| l.starts_with("run level change")).count(), "{case}");
    }
    assert!(run > 5);
}

#[test]
fn let_go_it_takes_the_nearest_step_as_on_every_renderer() {
    let all = cases();
    for case in all["drag"].as_array().unwrap() {
        let (_, mut screen) = slider_of(case, None);
        let id = part_named(&screen, "level");
        let at = text(case, "at").unwrap();
        screen.act(id, |e, _| e.as_any_mut().downcast_mut::<SliderPart>().unwrap().drop_at(&at));
        assert!(same(&text(case, "value").unwrap(), &slider(&screen).value().unwrap()), "{case}");
    }
}

#[test]
fn a_drag_writes_nothing_until_it_is_let_go() {
    let host = InputsHost::new();
    host.set("v", json!(0));
    let mut screen = window(r#"<Slider at="1,1" span="6" name="level" bind="v" min="0" max="10" onchange="moved" />"#, host.clone());
    let b = slider(&screen).node().bounds;
    let y = b.y + b.height / 2.0;
    screen.pointer_pressed(b.x + 2.0, y, PointerButton::Left, 1);
    screen.pointer_moved(b.x + b.width / 2.0, y);
    assert_eq!(5.0, slider(&screen).shown().round());
    assert!(host.logged().is_empty());
    screen.pointer_moved(b.right() + 50.0, y);
    screen.pointer_released(b.right() + 50.0, y, PointerButton::Left);
    assert_eq!(Some(json!(10)), host.read("v"));
    assert_eq!(vec!["write level v 10", "run level change moved"], host.logged());
}

#[test]
fn unbound_it_keeps_its_value_in_its_state() {
    let host = InputsHost::new();
    let mut screen = window(r#"<Slider at="1,1" span="6" name="level" min="0" max="1" step="0.1" />"#, host.clone());
    screen.focus(Some(part_named(&screen, "level")));
    screen.key_pressed(KeyInput::plain(Key::Right));
    screen.key_pressed(KeyInput::plain(Key::Right));
    assert_eq!(Some("0.2".to_string()), host.state(Some("level")).unwrap().text);
}

#[test]
fn a_read_only_slider_does_not_move_and_focus_tells_enter_and_leave() {
    let host = InputsHost::new();
    host.set("v", json!(3));
    let mut screen = window(r#"<Slider at="1,1" span="6" name="level" bind="v" readonly="true" onenter="came" onleave="went" />"#, host.clone());
    screen.focus(Some(part_named(&screen, "level")));
    screen.key_pressed(KeyInput::plain(Key::Up));
    screen.focus(None);
    assert_eq!(Some(json!(3)), host.read("v"));
    assert_eq!(vec!["run level enter came", "run level leave went"], host.logged());
}

#[test]
fn the_track_is_filled_up_to_the_thumb() {
    let host = InputsHost::new();
    host.set("v", json!(50));
    let mut screen = window(r#"<Slider at="1,1" span="6" name="level" bind="v" min="0" max="100" />"#, host);
    let (accent, line) = (screen.look().color("accent"), screen.look().color("strong_border"));
    let mut shot = Shot::of(&mut screen, 800, 400, 1.0);
    if let Ok(dir) = std::env::var("CLARITY_SHOTS") {
        shot.save(std::path::Path::new(&dir).join("inputs-slider.png")).expect("the picture is written");
    }
    let b = slider(&screen).node().bounds;
    let y = (b.y + b.height / 2.0) as i32;
    assert!(near(accent, shot.pixel((b.x + b.width * 0.25) as i32, y), 10));
    assert!(near(line, shot.pixel((b.x + b.width * 0.75) as i32, y), 10));
}
