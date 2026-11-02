//! The tags part by keyboard and its words, as Avalonia's tags input had
//! them: a chip's remove button says what it does, is a Tab stop that Enter
//! and Space press, and draws a focus ring; Enter in the typing box takes the
//! item and then submits the form; the box shows its word to add; a list of
//! days has a calendar button inside the box; an unbound tags part keeps its
//! own list.

use std::rc::Rc;

use clarity_ui::element::{draw_children, find, take_typed_from, Cx, Element, GridPanel, Node};
use clarity_ui::input::{Key, KeyInput, PointerButton};
use clarity_ui::painter::Painter;
use clarity_ui::parts::inputs::calendar::DayCalendar;
use clarity_ui::parts::inputs::tags::TagsPart;
use clarity_ui::parts::inputs::testing::InputsHost;
use clarity_ui::parts::PartHost;
use clarity_ui::testing::{part_named, window_of, Shot};
use clarity_ui::{Rect, Screen, ThemeLook};
use serde_json::{json, Value};
use skia_safe::Color;

const MARKUP: &str = r#"<Tags at="1,1" span="8" name="genres" bind="album.genres" onchange="genresChanged" />"#;

fn host_of(item: &str, items: Value) -> Rc<InputsHost> {
    let host = InputsHost::new();
    host.values.borrow_mut().insert("album.genres".into(), items);
    host.types.borrow_mut().insert("album.genres".into(), format!("list<{item}>"));
    if item == "Genre" {
        host.offer("album.genres", &[(json!("Jazz"), "Jazz"), (json!("Rock"), "Rock"), (json!("Folk"), "Folk")]);
    }
    host
}

fn make(host: Rc<InputsHost>, markup: &str) -> Screen {
    let mut screen = window_of(markup, host);
    screen.resize(800.0, 400.0, 1.0);
    screen
}

fn tags<'a>(screen: &'a Screen, name: &str) -> &'a TagsPart {
    screen.element(part_named(screen, name)).and_then(|e| e.as_any().downcast_ref::<TagsPart>()).expect("a tags part")
}

fn click(screen: &mut Screen, b: Rect) {
    let (x, y) = (b.x + b.width / 2.0, b.y + b.height / 2.0);
    screen.pointer_pressed(x, y, PointerButton::Left, 1);
    screen.pointer_released(x, y, PointerButton::Left);
}

fn press(screen: &mut Screen, key: Key) {
    screen.key_pressed(KeyInput::plain(key));
}

fn items(host: &InputsHost) -> Value {
    host.read("album.genres").unwrap_or(Value::Null)
}

/// Whether some pixel in `area` is near `color`.
fn inked(shot: &mut Shot, area: Rect, color: Color) -> bool {
    for y in area.y as i32..(area.y + area.height) as i32 {
        for x in area.x as i32..(area.x + area.width) as i32 {
            let p = shot.pixel(x, y);
            let near = |a: u8, b: u8| (a as i32 - b as i32).abs() < 6;
            if near(p.r(), color.r()) && near(p.g(), color.g()) && near(p.b(), color.b()) {
                return true;
            }
        }
    }
    false
}

#[test]
fn the_remove_button_says_what_it_does() {
    let screen = make(host_of("text", json!(["jazz"])), MARKUP);
    let remove = tags(&screen, "genres").remove_id(0).expect("a remove button");
    let at = tags(&screen, "genres").chips()[0].remove.unwrap();
    assert_eq!(Some("Remove".to_string()), screen.element(remove).unwrap().tooltip_at(at.x + 2.0, at.y + 2.0));
}

#[test]
fn the_remove_buttons_are_tab_stops_before_the_box() {
    let mut screen = make(host_of("text", json!(["jazz", "live"])), MARKUP);
    let first = tags(&screen, "genres").remove_id(0).unwrap();
    assert!(screen.element(first).unwrap().focusable());
    screen.focus(Some(first));
    press(&mut screen, Key::Tab);
    assert_eq!(tags(&screen, "genres").remove_id(1), screen.focused());
    press(&mut screen, Key::Tab);
    assert_eq!(tags(&screen, "genres").box_id(), screen.focused());
}

#[test]
fn enter_and_space_press_the_remove_button_and_the_focus_goes_on_to_the_next_chip_else_the_box() {
    let host = host_of("text", json!(["jazz", "live", "folk"]));
    let mut screen = make(host.clone(), MARKUP);
    let second = tags(&screen, "genres").remove_id(1);
    screen.focus(second);
    press(&mut screen, Key::Enter);
    assert_eq!(json!(["jazz", "folk"]), items(&host));
    assert_eq!("run genres change genresChanged", host.logged().last().unwrap());
    assert_eq!(tags(&screen, "genres").remove_id(1), screen.focused());
    press(&mut screen, Key::Space);
    assert_eq!(json!(["jazz"]), items(&host));
    // The last chip gone, the box is next.
    assert_eq!(tags(&screen, "genres").box_id(), screen.focused());
    let first = tags(&screen, "genres").remove_id(0);
    screen.focus(first);
    press(&mut screen, Key::Enter);
    assert_eq!(json!([]), items(&host));
    assert_eq!(tags(&screen, "genres").box_id(), screen.focused());
}

#[test]
fn a_focused_remove_button_draws_its_focus_ring() {
    let mut screen = make(host_of("text", json!(["jazz"])), MARKUP);
    let focus = screen.look().color("focus");
    let remove = tags(&screen, "genres").remove_id(0).unwrap();
    let area = tags(&screen, "genres").chips()[0].remove.unwrap();
    let mut shot = Shot::of(&mut screen, 800, 400, 1.0);
    assert!(!inked(&mut shot, area, focus));
    screen.focus(Some(remove));
    let mut shot = Shot::of(&mut screen, 800, 400, 1.0);
    assert!(inked(&mut shot, area, focus));
}

/// A form around a grid of parts: Enter in its one-line inputs takes what is typed and submits it.
struct TestForm {
    node: Node,
    host: Rc<InputsHost>,
}

impl Element for TestForm {
    clarity_ui::element_node!();

    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.node.bounds = area;
        for child in self.node.children_mut() {
            child.arrange(area, cx);
        }
    }

    fn draw(&self, p: &Painter) {
        draw_children(&self.node, p);
    }

    fn window_key(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        if key.key != Key::Enter {
            return false;
        }
        let focused = cx.focused();
        if focused.and_then(|f| find(self, f)).is_none_or(|e| e.multiline()) {
            return false;
        }
        take_typed_from(self, focused, cx);
        self.host.log.borrow_mut().push("submit".to_string());
        true
    }
}

#[test]
fn enter_in_the_box_takes_the_item_and_then_the_form_submits() {
    let host = host_of("text", json!(["jazz"]));
    let spec = clarity_ui::markup::read(&format!("<Window name=\"Test\">{MARKUP}</Window>")).expect("the markup reads");
    let mut grid = GridPanel::new(12);
    for p in spec.parts {
        let made = clarity_ui::parts::make(p, host.clone());
        let place = clarity_ui::parts::place_of(made.as_ref());
        grid.add(made, place);
    }
    let mut form = TestForm { node: Node::new(), host: host.clone() };
    form.node.add(Box::new(grid));
    let mut screen = Screen::new(Box::new(form), ThemeLook::clarity(false));
    screen.resize(800.0, 400.0, 1.0);
    let editor = tags(&screen, "genres").box_id();
    screen.focus(editor);
    screen.text_typed("live");
    press(&mut screen, Key::Enter);
    assert_eq!(json!(["jazz", "live"]), items(&host));
    let log = host.logged();
    assert_eq!("run genres change genresChanged", log[log.len() - 2]);
    assert_eq!("submit", log[log.len() - 1]);
}

#[test]
fn the_box_shows_the_word_to_add() {
    let screen = make(host_of("text", json!(["jazz"])), MARKUP);
    assert_eq!(Some("Add".to_string()), tags(&screen, "genres").editor().unwrap().placeholder.clone());
    let screen = make(host_of("Genre", json!(["Jazz"])), MARKUP);
    assert!(tags(&screen, "genres").adder().is_some());
    assert_eq!("Add", TagsPart::adder_name());
}

#[test]
fn a_list_of_days_opens_its_calendar_from_the_button_inside_the_box() {
    let host = host_of("date", json!(["2026-10-01"]));
    let mut screen = make(host.clone(), MARKUP);
    let place = tags(&screen, "genres").editor().unwrap().node().bounds;
    let button = tags(&screen, "genres").calendar_button().expect("a calendar button");
    assert!(button.x > place.x && button.right() <= place.right());
    assert!(tags(&screen, "genres").editor().unwrap().right_inset >= button.width);
    let id = clarity_ui::element::at(screen.root(), button.x + 2.0, button.y + 2.0).unwrap();
    assert_eq!(Some("Calendar".to_string()), screen.element(id).unwrap().tooltip_at(button.x + 2.0, button.y + 2.0));
    // The focus alone opens nothing.
    let editor = tags(&screen, "genres").box_id();
    screen.focus(editor);
    assert!(screen.overlays().is_empty());
    click(&mut screen, button);
    assert_eq!(1, screen.overlays().len());
    assert!(screen.overlays()[0].content.as_any().is::<DayCalendar>());
    let calendar = tags(&screen, "genres").calendar().unwrap();
    click(&mut screen, calendar.cell_of(calendar.day()).unwrap());
    assert!(screen.overlays().is_empty());
    assert_eq!(json!(["2026-10-01", calendar.day().iso()]), items(&host));
    // The button closes it again.
    click(&mut screen, button);
    assert_eq!(1, screen.overlays().len());
    click(&mut screen, button);
    assert!(screen.overlays().is_empty());
    // A list of words has no calendar button.
    let screen = make(host_of("text", json!(["jazz"])), MARKUP);
    assert!(tags(&screen, "genres").calendar_button().is_none());
}

#[test]
fn an_unbound_tags_part_keeps_its_own_list() {
    let host = InputsHost::new();
    let mut screen = make(host.clone(), r#"<Tags at="1,1" span="8" name="moods" onchange="moodsChanged" />"#);
    let editor = tags(&screen, "moods").box_id();
    assert!(editor.is_some());
    screen.focus(editor);
    screen.text_typed("calm");
    press(&mut screen, Key::Enter);
    screen.text_typed("bright");
    press(&mut screen, Key::Enter);
    assert_eq!(vec![json!("calm"), json!("bright")], tags(&screen, "moods").items());
    assert_eq!(vec!["calm", "bright"], tags(&screen, "moods").chips().iter().map(|c| c.label.clone()).collect::<Vec<_>>());
    let _ = Shot::of(&mut screen, 800, 400, 1.0);
    let first = tags(&screen, "moods").chips()[0].remove.unwrap();
    click(&mut screen, first);
    assert_eq!(vec![json!("bright")], tags(&screen, "moods").items());
    assert_eq!(vec!["run moods change moodsChanged", "run moods change moodsChanged", "run moods change moodsChanged"], host.logged());
    // A named one keeps it in its part state.
    assert_eq!(Some(&json!(["bright"])), host.state(Some("moods")).unwrap().values.as_ref().and_then(|v| v.get("tags")));
}
