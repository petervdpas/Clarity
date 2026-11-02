//! The chart against the cases every renderer shares (conformance/charts.json),
//! and as a part: a list's rows grouped by a field and drawn as bars, a line
//! or a pie, again when the rows change; a list of numbers drawn as they are;
//! a bar, point or slice telling its group and number when pointed at.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use clarity_runtime::ui::PartState;
use clarity_ui::markup::{HandlerCall, PartSpec};
use clarity_ui::parts::data::chart::{self, ChartPart, ChartShape, MarkKind};
use clarity_ui::parts::{Choice, Part, PartHost};
use clarity_ui::testing::{part_named, window_of, Shot, TestHost};
use clarity_ui::{Rect, Screen, ThemeLook};
use serde_json::{json, Value};
use skia_safe::Color;

fn conformance(name: &str) -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance").join(name);
    serde_json::from_str(&std::fs::read_to_string(path).expect("the conformance file reads")).expect("it is JSON")
}

/// The shared test host with the choices and types a chart reads.
#[derive(Default)]
struct ChartHost {
    inner: TestHost,
    choices: RefCell<BTreeMap<String, Vec<Choice>>>,
}

impl ChartHost {
    fn offer(&self, path: &str, choices: &[(Value, &str)]) {
        let made = choices.iter().map(|(value, label)| Choice { value: value.clone(), label: label.to_string(), cells: None }).collect();
        self.choices.borrow_mut().insert(path.to_string(), made);
    }
}

impl PartHost for ChartHost {
    fn read(&self, path: &str) -> Option<Value> {
        self.inner.read(path)
    }
    fn write(&self, part: &str, path: &str, value: Value) {
        self.inner.write(part, path, value)
    }
    fn edited(&self, part: &str) {
        self.inner.edited(part)
    }
    fn state(&self, name: Option<&str>) -> Option<PartState> {
        self.inner.state(name)
    }
    fn change_state(&self, name: &str, change: &mut dyn FnMut(&mut PartState)) {
        self.inner.change_state(name, change)
    }
    fn run(&self, part: Option<&str>, call: &HandlerCall) -> bool {
        self.inner.run(part, call)
    }
    fn text(&self, written: Option<&str>) -> String {
        self.inner.text(written)
    }
    fn changes(&self) -> u64 {
        self.inner.changes()
    }
    fn choices(&self, path: &str) -> Vec<Choice> {
        self.choices.borrow().get(path).cloned().unwrap_or_default()
    }
    fn offered(&self, _part: &PartSpec, path: &str) -> Vec<Choice> {
        self.choices(path)
    }
}

fn orders(rows: Value) -> Rc<ChartHost> {
    let host = Rc::new(ChartHost::default());
    host.inner.set("orders", rows);
    host
}

const SUMMED: &str = r#"<Chart name="totals" at="1,1" span="6" rows="6" bind="orders" by="status" measure="{&quot;kind&quot;:&quot;sum&quot;,&quot;field&quot;:&quot;total&quot;}" CHART />"#;

fn chart_screen(host: Rc<ChartHost>, shape: &str, markup: &str) -> Screen {
    let markup = markup.replace("CHART", &format!("chart=\"{shape}\""));
    let mut screen = window_of(&markup, host);
    screen.resize(800.0, 400.0, 1.0);
    screen
}

fn the_chart(screen: &Screen) -> &ChartPart {
    screen.find::<ChartPart>().expect("the window has its chart")
}

fn drawn(screen: &mut Screen) -> Shot {
    Shot::of(screen, 800, 400, 1.0)
}

fn has(shot: &mut Shot, place: Rect, color: Color) -> bool {
    (place.x as i32..place.right() as i32).any(|x| (place.y as i32..place.bottom() as i32).any(|y| shot.pixel(x, y) == color))
}

#[test]
fn the_axis_and_the_slices_are_as_on_every_renderer() {
    let file = conformance("charts.json");
    for c in file["scale"].as_array().unwrap() {
        let got = chart::scale(c["low"].as_f64().unwrap(), c["high"].as_f64().unwrap());
        assert_eq!((c["bottom"].as_f64().unwrap(), c["top"].as_f64().unwrap(), c["step"].as_f64().unwrap()), (got.bottom, got.top, got.step), "{c}");
    }
    for c in file["slices"].as_array().unwrap() {
        let values: Vec<f64> = c["values"].as_array().unwrap().iter().map(|v| v.as_f64().unwrap()).collect();
        let want: Vec<Option<&str>> = c["colors"].as_array().unwrap().iter().map(Value::as_str).collect();
        assert_eq!(want, chart::slice_colors(&values));
    }
}

#[test]
fn a_chart_groups_its_rows_and_groups_them_again_when_they_change() {
    let host = orders(json!([{"status": "Sent", "total": 5}, {"status": "Open", "total": 2.5}, {"status": "Sent", "total": 1}]));
    let screen = chart_screen(host.clone(), "bar", SUMMED);
    let chart = the_chart(&screen);
    assert_eq!(ChartShape::Bar, chart.shape());
    let groups = chart.groups();
    assert_eq!(vec!["Open", "Sent"], groups.iter().map(|g| g.label.as_str()).collect::<Vec<_>>());
    assert_eq!(vec![2.5, 6.0], groups.iter().map(|g| g.value).collect::<Vec<_>>());
    assert_eq!("6", groups[1].text);
    host.inner.set("orders", json!([{"status": "Sent", "total": 5}, {"status": "Open", "total": 2.5}, {"status": "Paid", "total": 1}]));
    let labels: Vec<String> = the_chart(&screen).groups().into_iter().map(|g| g.label).collect();
    assert_eq!(vec!["Open", "Paid", "Sent"], labels);
}

#[test]
fn a_group_shows_by_the_label_its_choice_has_and_a_reference_by_its_text() {
    let host = orders(json!([{"status": "Open", "customer": 2}, {"status": "Paid", "customer": 1}, {"status": "Open", "customer": 1}]));
    host.offer("orders.status", &[(json!("Open"), "Still open"), (json!("Paid"), "Paid up"), (json!("Sent"), "On its way")]);
    host.offer("orders.customer", &[(json!(1), "Zeno"), (json!(2), "Ada")]);
    let screen = chart_screen(host.clone(), "bar", r#"<Chart at="1,1" span="6" rows="6" bind="orders" by="status" CHART />"#);
    let groups = the_chart(&screen).groups();
    assert_eq!(vec!["Still open", "Paid up"], groups.iter().map(|g| g.label.as_str()).collect::<Vec<_>>());
    assert_eq!(vec!["2", "1"], groups.iter().map(|g| g.text.as_str()).collect::<Vec<_>>());
    let screen = chart_screen(host, "bar", r#"<Chart at="1,1" span="6" rows="6" bind="orders" by="customer" CHART />"#);
    let groups = the_chart(&screen).groups();
    assert_eq!(vec!["Ada", "Zeno"], groups.iter().map(|g| g.label.as_str()).collect::<Vec<_>>());
    assert_eq!(vec![1.0, 2.0], groups.iter().map(|g| g.value).collect::<Vec<_>>());
}

#[test]
fn a_chart_of_numbers_shows_its_last_points() {
    let shown = chart::numbers_shown(&[json!("1.5"), json!(2), json!("x"), Value::Null, json!(3)], 2);
    assert_eq!(vec![2.0, 3.0], shown.iter().map(|s| s.value).collect::<Vec<_>>());
    assert!(shown.iter().all(|s| s.label.is_empty()));
    assert_eq!(3, chart::numbers_shown(&[json!(1), json!(2), json!(3)], 0).len());

    let host = Rc::new(ChartHost::default());
    host.inner.set("readings", json!([4, 5, 6, 7]));
    let screen = chart_screen(host, "line", r#"<Chart at="1,1" span="6" rows="6" bind="readings" points="3" max="10" CHART />"#);
    let chart = the_chart(&screen);
    assert_eq!(vec![5.0, 6.0, 7.0], chart.groups().iter().map(|g| g.value).collect::<Vec<_>>());
    assert_eq!(Some(10.0), chart.max());
}

#[test]
fn bars_are_drawn_in_the_accent_and_slices_in_the_palette() {
    let host = orders(json!([{"status": "Sent", "total": 5}, {"status": "Open", "total": 2.5}]));
    let accent = ThemeLook::clarity(false).color("accent");
    let mut screen = chart_screen(host.clone(), "bar", SUMMED);
    let mut shot = drawn(&mut screen);
    let chart = the_chart(&screen);
    let bounds = chart.base().node.bounds;
    assert!(has(&mut shot, bounds, accent));
    let marks = chart.marks(bounds);
    assert_eq!(2, marks.len());
    for m in &marks {
        assert_eq!(accent, shot.pixel((m.place.x + m.place.width / 2.0) as i32, (m.place.bottom() - 2.0) as i32));
    }
    shot.save(shots().join("data-chart-bars.png")).unwrap();

    let mut screen = chart_screen(host, "pie", SUMMED);
    let mut shot = drawn(&mut screen);
    let bounds = the_chart(&screen).base().node.bounds;
    assert!(has(&mut shot, bounds, Color::from_rgb(0x64, 0x74, 0x8b)));
    assert!(has(&mut shot, bounds, Color::from_rgb(0xef, 0x44, 0x44)));
    assert!(!has(&mut shot, bounds, accent));
    shot.save(shots().join("data-chart-pie.png")).unwrap();
}

fn shots() -> std::path::PathBuf {
    std::path::PathBuf::from("/tmp/claude-1000/-home-peter-Projects-Clarity/271a7793-55cd-491f-b56c-9578fe570b4e/scratchpad/shots")
}

#[test]
fn a_line_of_many_numbers_has_no_dot_on_each() {
    let host = Rc::new(ChartHost::default());
    host.inner.set("readings", Value::Array((1..=40).map(Value::from).collect()));
    let mut screen = chart_screen(host.clone(), "line", r#"<Chart at="1,1" span="6" rows="6" bind="readings" max="100" CHART />"#);
    let mut shot = drawn(&mut screen);
    shot.save(shots().join("data-chart-line.png")).unwrap();
    let chart = the_chart(&screen);
    let bounds = chart.base().node.bounds;
    assert_eq!(0, chart.marks(bounds).iter().filter(|m| m.dot).count());
    host.inner.set("readings", json!([1, 2, 3]));
    assert_eq!(3, the_chart(&screen).marks(bounds).iter().filter(|m| m.dot).count());
}

#[test]
fn a_bar_point_or_slice_tells_its_group_and_number_when_pointed_at() {
    let host = orders(json!([{"status": "Sent", "total": 5}, {"status": "Open", "total": 2.5}]));
    for shape in ["bar", "line", "pie"] {
        let mut screen = chart_screen(host.clone(), shape, SUMMED);
        drawn(&mut screen);
        let chart = the_chart(&screen);
        let bounds = chart.base().node.bounds;
        let open = &chart.marks(bounds)[0];
        assert_eq!(if shape == "pie" { MarkKind::Slice } else if shape == "bar" { MarkKind::Bar } else { MarkKind::Point }, open.kind);
        let (x, y) = open.center();
        assert_eq!(Some("Open: 2.5".to_string()), chart.tip_at(x, y), "{shape}");
        assert_eq!(None, chart.tip_at(bounds.x + 1.0, bounds.y + 1.0));
        screen.pointer_moved(x, y);
        assert_eq!(Some("Open: 2.5".to_string()), the_chart(&screen).tip(), "{shape}");
    }
}

#[test]
fn a_hidden_chart_draws_nothing() {
    let host = orders(json!([{"status": "Sent", "total": 5}]));
    let mut screen = chart_screen(host.clone(), "bar", SUMMED);
    host.inner.set_state("totals", |s| s.visible = Some(false));
    let mut shot = drawn(&mut screen);
    let id = part_named(&screen, "totals");
    let bounds = screen.element(id).unwrap().node().bounds;
    assert!(!has(&mut shot, Rect::new(0.0, 0.0, 800.0, 400.0).inset(0.0), ThemeLook::clarity(false).color("accent")), "{bounds}");
}

#[test]
fn a_group_s_number_shows_in_the_person_s_format() {
    // C# ChartPart: a sum as ShownText shows a decimal, with its thousands.
    let host = orders(json!([{"status": "Sent", "total": 1234567}, {"status": "Open", "total": 2500.5}]));
    let screen = chart_screen(host, "bar", SUMMED);
    let groups = the_chart(&screen).groups();
    assert_eq!(vec!["2,500.5", "1,234,567"], groups.iter().map(|g| g.text.as_str()).collect::<Vec<_>>());
}
