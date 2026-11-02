//! Custom parts (DESIGN 5.19, 7.9), as the C# CustomPartTests check them: a
//! part a crate of the program brings, found by its declared name, drawn in
//! its place. Values come in (what it binds, its properties, enabled), code's
//! hears come in, the person's change goes out and runs `on change`, and its
//! tells run the handlers wired to them.

use clarity_runtime::ui::Hear;
use clarity_ui::painter::Painter;
use clarity_ui::parts::custom::{self, CustomCx, CustomPart, CustomPlace};
use clarity_ui::parts::PartHost;
use clarity_ui::testing::{part_named, window_of, Shot, TestHost};
use clarity_ui::{Element, Rect, Screen};
use serde_json::{json, Value};

/// The Gauge as a crate brings it: what it was set and told, and a click that changes its value and tells `reached`.
#[derive(Default)]
struct Gauge {
    got: Vec<String>,
}

impl CustomPart for Gauge {
    fn set(&mut self, name: &str, value: &Value, _cx: &mut CustomCx) {
        self.got.push(format!("set {name} {value}"));
    }

    fn hear(&mut self, hear: &str, values: &[Value], _cx: &mut CustomCx) {
        let values: Vec<String> = values.iter().map(|v| v.to_string()).collect();
        self.got.push(format!("hear {hear} {}", values.join(",")));
    }

    fn draw(&self, p: &Painter, area: Rect) {
        p.fill(area, p.look.color("accent"), 0.0);
    }

    fn focusable(&self) -> bool {
        true
    }

    fn click(&mut self, cx: &mut CustomCx) {
        cx.change(json!(9));
        cx.tell("reached", vec![json!(3), json!("high")]);
        cx.tell("dropped", vec![json!(1)]);
    }
}

const LINE: &str = r#"<Custom name="gauge" at="1,1" span="4" rows="3" part="Gauge" bind="speed" onchange="gaugeChanged" tells='[{"tell":"reached","handler":"gaugeReached"}]' />"#;

fn window(host: std::rc::Rc<TestHost>) -> Screen {
    custom::register("Gauge", || Box::new(Gauge::default()));
    let mut screen = window_of(LINE, host);
    screen.resize(800.0, 400.0, 1.0);
    screen
}

fn place(screen: &Screen) -> &CustomPlace {
    screen.element(part_named(screen, "gauge")).unwrap().as_any().downcast_ref::<CustomPlace>().unwrap()
}

fn got(screen: &Screen) -> Vec<String> {
    place(screen).inner::<Gauge>().unwrap().got.clone()
}

fn clear(screen: &mut Screen) {
    let id = part_named(screen, "gauge");
    screen.act(id, |e, _| e.as_any_mut().downcast_mut::<CustomPlace>().unwrap().inner_mut::<Gauge>().unwrap().got.clear());
}

fn hear(host: &TestHost, hear: &str, values: Vec<Value>) {
    host.hears.borrow_mut().push(Hear { part: "gauge".to_string(), hear: hear.to_string(), values });
    host.change();
}

#[test]
fn it_is_the_crates_part_found_by_its_declared_name_in_its_place() {
    let mut screen = window(TestHost::new());
    let b = place(&screen).node().bounds;
    assert!(b.width > 0.0);
    assert!(place(&screen).inner::<Gauge>().is_some());
    assert_eq!(Some(place(&screen).node().id()), screen.at(b.x + 4.0, b.y + 4.0));
    // It draws itself.
    let accent = screen.look().color("accent");
    let mut shot = Shot::of(&mut screen, 800, 400, 1.0);
    assert_eq!(accent, shot.pixel(b.x as i32 + 10, b.y as i32 + 10));
}

#[test]
fn its_values_come_in_from_its_line_its_binding_and_code() {
    let host = TestHost::new();
    host.set("speed", json!(3));
    host.set_state("gauge", |s| s.values = Some(json!({ "title": "Speed" }).as_object().unwrap().clone()));
    hear(&host, "reset", vec![json!(1)]);
    let mut screen = window(host.clone());
    assert_eq!(vec!["set title \"Speed\"", "set value 3", "set enabled true", "hear reset 1"], got(&screen));
    clear(&mut screen);
    host.set_state("gauge", |s| {
        s.values.get_or_insert_with(Default::default).insert("speed".into(), json!(7));
    });
    hear(&host, "reset", vec![json!(2)]);
    host.set("speed", json!(4));
    host.set_state("gauge", |s| s.enabled = Some(false));
    screen.resize(800.0, 400.0, 1.0);
    // The screen gives what changed as it lays the window out, each kind in its own order (C# gives each as it happens).
    let mut given = got(&screen);
    given.sort();
    assert_eq!(vec!["hear reset 2", "set enabled false", "set speed 7", "set value 4"], given);
    // Laid out again with nothing changed: nothing is given again.
    clear(&mut screen);
    screen.resize(800.0, 400.0, 1.0);
    assert!(got(&screen).is_empty());
}

#[test]
fn the_persons_change_goes_to_what_it_binds_and_its_tells_run_their_handlers() {
    let host = TestHost::new();
    let mut screen = window(host.clone());
    let b = place(&screen).node().bounds;
    screen.click(b.x + 4.0, b.y + 4.0);
    assert_eq!(vec!["write gauge speed 9", "run gauge change gaugeChanged"], host.logged());
    // Only the wired tell reaches the window; one no handler is wired to runs nothing.
    assert_eq!(vec!["gauge reached 3 \"high\""], host.told.borrow().clone());
    // Its own change is not given back to it as a new value.
    clear(&mut screen);
    screen.resize(800.0, 400.0, 1.0);
    assert!(got(&screen).is_empty());
    assert_eq!(Some(json!(9)), host.read("speed"));
}

#[test]
fn a_part_no_crate_brings_says_so_in_its_place() {
    let mut screen = window_of(r#"<Custom name="map" at="1,1" span="4" part="Map" />"#, TestHost::new());
    screen.resize(800.0, 400.0, 1.0);
    let place = screen.element(part_named(&screen, "map")).unwrap().as_any().downcast_ref::<CustomPlace>().unwrap();
    assert!(place.why.as_deref().unwrap_or_default().contains("Map"));
}

#[test]
fn a_part_added_by_hand_is_found_too() {
    custom::register("HandGauge", || Box::new(Gauge::default()));
    let mut screen = window_of(r#"<Custom name="hand" at="1,1" span="4" part="HandGauge" />"#, TestHost::new());
    screen.resize(800.0, 400.0, 1.0);
    let place = screen.element(part_named(&screen, "hand")).unwrap().as_any().downcast_ref::<CustomPlace>().unwrap();
    assert!(place.inner::<Gauge>().is_some());
}

#[test]
fn values_are_read_as_the_types_the_declaration_gives() {
    assert_eq!(Some(5i64), custom::value_as::<i64>(&json!(5)));
    assert_eq!(Some(5i64), custom::value_as::<i64>(&json!(5.0)));
    assert_eq!(Some(5i64), custom::value_as::<i64>(&json!("5")));
    assert_eq!(Some("high".to_string()), custom::value_as::<String>(&json!("high")));
}
