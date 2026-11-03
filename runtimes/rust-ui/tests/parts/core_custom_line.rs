//! A custom part hears the values written on its line when its window opens
//! (`Dice name board ... count 2`), before what code set (code wins), and
//! starts once, so it can start a timer or an animation of its own.

use clarity_ui::parts::custom::{self, CustomCx, CustomPart, CustomPlace};
use clarity_ui::testing::{TestHost, Shot, part_named, window_of};
use clarity_ui::{Painter, Rect};
use serde_json::{Value, json};

#[derive(Default)]
struct Probe {
    set: Vec<String>,
    started: usize,
}

impl CustomPart for Probe {
    fn set(&mut self, name: &str, value: &Value, _cx: &mut CustomCx) {
        self.set.push(format!("{name}={value}"));
    }

    fn hear(&mut self, _hear: &str, _values: &[Value], _cx: &mut CustomCx) {}

    fn draw(&self, _p: &Painter, _area: Rect) {}

    fn start(&mut self, _cx: &mut CustomCx) {
        self.started += 1;
    }
}

fn probe<'a>(screen: &'a clarity_ui::Screen) -> &'a Probe {
    screen.element(part_named(screen, "board")).unwrap().as_any().downcast_ref::<CustomPlace>().unwrap().inner::<Probe>().unwrap()
}

#[test]
fn the_values_on_its_line_come_first_and_it_starts_once() {
    custom::register("Probe", || Box::new(Probe::default()));
    let host = TestHost::new();
    host.set_state("board", |s| s.values = Some(json!({ "title": "From code" }).as_object().unwrap().clone()));
    let line = r#"<Custom name="board" part="Probe" at="1,1" span="4" values="{&quot;count&quot;:{&quot;kind&quot;:&quot;number&quot;,&quot;value&quot;:&quot;2&quot;},&quot;title&quot;:{&quot;kind&quot;:&quot;text&quot;,&quot;value&quot;:&quot;On the line&quot;},&quot;loud&quot;:{&quot;kind&quot;:&quot;bool&quot;,&quot;value&quot;:&quot;true&quot;}}" />"#;
    let mut screen = window_of(line, host);
    Shot::of(&mut screen, 400, 120, 1.0);
    let set = &probe(&screen).set;
    assert!(set.contains(&"count=2".to_string()), "{set:?}");
    assert!(set.contains(&"loud=true".to_string()), "{set:?}");
    // Code's value comes after the line's, so it wins.
    let line_title = set.iter().position(|s| s == "title=\"On the line\"").expect("the line's title");
    let code_title = set.iter().position(|s| s == "title=\"From code\"").expect("code's title");
    assert!(line_title < code_title, "{set:?}");
    Shot::of(&mut screen, 400, 120, 1.0);
    assert_eq!(probe(&screen).started, 1);
}

/// Ctrl+Z takes back a value code set (conformance/edit-bus.json, `states`):
/// a value that is no longer set goes back to the one on its line.
#[test]
fn a_value_code_no_longer_sets_goes_back_to_its_line() {
    custom::register("Probe", || Box::new(Probe::default()));
    let host = TestHost::new();
    let line = r#"<Custom name="board" part="Probe" at="1,1" span="4" values="{&quot;count&quot;:{&quot;kind&quot;:&quot;number&quot;,&quot;value&quot;:&quot;2&quot;}}" />"#;
    let mut screen = window_of(line, host.clone());
    Shot::of(&mut screen, 400, 120, 1.0);
    host.set_state("board", |s| s.values = Some(json!({ "count": 3 }).as_object().unwrap().clone()));
    Shot::of(&mut screen, 400, 120, 1.0);
    host.set_state("board", |s| s.values = None);
    Shot::of(&mut screen, 400, 120, 1.0);
    let set = &probe(&screen).set;
    assert_eq!(set.iter().filter(|s| s.starts_with("count=")).cloned().collect::<Vec<_>>(), ["count=2", "count=3", "count=2"], "{set:?}");
}

/// A text on its line written as a key (`title @speedTitle`) arrives in the
/// language shown, not as the key.
#[test]
fn a_text_written_as_a_key_arrives_translated() {
    clarity_runtime::i18n::load(r#"{"project":{"locale":{"language":"en"},"translations":{"en":{"probeSpeedTitle":"Speed"}}}}"#);
    custom::register("Probe", || Box::new(Probe::default()));
    let line = r#"<Custom name="board" part="Probe" at="1,1" span="4" values="{&quot;title&quot;:{&quot;kind&quot;:&quot;text&quot;,&quot;value&quot;:&quot;@probeSpeedTitle&quot;}}" />"#;
    let mut screen = window_of(line, TestHost::new());
    Shot::of(&mut screen, 400, 120, 1.0);
    let set = &probe(&screen).set;
    assert!(set.contains(&"title=\"Speed\"".to_string()), "{set:?}");
}
