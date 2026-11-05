//! The tags part against the cases every renderer shares
//! (conformance/list-fields.json): items typed into its box, picked from an
//! enum's values, removed by their chip. Every change writes the record a
//! new list. A key that runs a command takes what is typed first.

use std::rc::Rc;

use clarity_ui::element::Element;
use clarity_ui::grid::ROW_HEIGHT;
use clarity_ui::input::{Key, KeyInput, PointerButton};
use clarity_ui::parts::inputs::choice_list::LINE;
use clarity_ui::parts::inputs::tags::TagsPart;
use clarity_ui::parts::inputs::testing::InputsHost;
use clarity_ui::parts::PartHost;
use clarity_ui::testing::{part_named, window_of, Shot};
use clarity_ui::{Rect, Screen};
use serde_json::{json, Value};

const MARKUP: &str = r#"<Tags at="1,1" span="8" name="genres" bind="album.genres" onchange="genresChanged" />"#;

fn cases() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../conformance/list-fields.json");
    serde_json::from_str(&std::fs::read_to_string(path).expect("conformance/list-fields.json")).expect("it is JSON")
}

/// A host whose `album.genres` holds `items`, a list of `item`s; an enum's values with `labels`.
fn host_of(item: &str, items: Value) -> Rc<InputsHost> {
    let host = InputsHost::new();
    host.values.borrow_mut().insert("album.genres".into(), items);
    let ty = if item == "enum" { "list<Genre>".to_string() } else { format!("list<{item}>") };
    host.types.borrow_mut().insert("album.genres".into(), ty);
    if item == "enum" {
        host.offer("album.genres", &[(json!("Jazz"), "Jazz"), (json!("Rock"), "Rock"), (json!("Folk"), "Folk")]);
    }
    host
}

fn make(host: Rc<InputsHost>, markup: &str) -> Screen {
    let mut screen = window_of(markup, host);
    screen.resize(800.0, 400.0, 1.0);
    screen
}

fn tags<'a>(screen: &'a Screen) -> &'a TagsPart {
    screen.element(part_named(screen, "genres")).and_then(|e| e.as_any().downcast_ref::<TagsPart>()).expect("a tags part")
}

fn act(screen: &mut Screen, run: impl FnOnce(&mut TagsPart, &mut clarity_ui::Cx)) {
    let id = part_named(screen, "genres");
    screen.act(id, |e, cx| run(e.as_any_mut().downcast_mut::<TagsPart>().unwrap(), cx));
}

fn click(screen: &mut Screen, b: Rect) {
    let (x, y) = (b.x + b.width / 2.0, b.y + b.height / 2.0);
    screen.pointer_pressed(x, y, PointerButton::Left, 1);
    screen.pointer_released(x, y, PointerButton::Left);
}

fn keep(shot: &mut Shot, name: &str) {
    if let Ok(dir) = std::env::var("CLARITY_SHOTS") {
        shot.save(std::path::Path::new(&dir).join(name)).expect("the picture is written");
    }
}

#[test]
fn a_list_of_days_adds_a_day_picked_from_the_calendar() {
    let host = host_of("date", json!(["2026-10-01"]));
    let mut screen = make(host.clone(), MARKUP);
    let editor = tags(&screen).box_id();
    screen.focus(editor);
    assert!(screen.overlays().is_empty());
    // Down in the box opens it; the button inside the box does too.
    screen.key_pressed(KeyInput::plain(Key::Down));
    assert_eq!(1, screen.overlays().len());
    assert!(screen.overlays()[0].content.as_any().is::<clarity_ui::parts::inputs::calendar::DayCalendar>());
    // The box keeps the focus: its keys work the calendar.
    assert_eq!(editor, screen.focused());
    let calendar = tags(&screen).calendar().expect("the calendar is open");
    click(&mut screen, calendar.cell_of(calendar.day()).unwrap());
    assert!(screen.overlays().is_empty());
    assert_eq!(Some(json!(["2026-10-01", calendar.day().iso()])), host.read("album.genres"));
    // Escape closes it and adds nothing.
    screen.focus(None);
    screen.focus(editor);
    let button = tags(&screen).calendar_button().unwrap();
    click(&mut screen, button);
    assert_eq!(1, screen.overlays().len());
    assert_eq!(editor, screen.focused());
    screen.key_pressed(KeyInput::plain(Key::Escape));
    assert!(screen.overlays().is_empty());
    assert_eq!(2, host.read("album.genres").unwrap().as_array().unwrap().len());
}

#[test]
fn a_typed_item_is_added_as_on_every_renderer() {
    for case in cases()["added"].as_array().unwrap() {
        let item = case["item"].as_str().unwrap();
        let before = case["items"].clone();
        let host = host_of(item, before.clone());
        let mut screen = make(host.clone(), MARKUP);
        let typed = case["typed"].as_str().unwrap().to_string();
        let refused = case["refused"].as_bool().unwrap_or(false);
        act(&mut screen, |t, cx| {
            t.editor_mut().expect("a box to type in").set_text(&typed);
            t.take(cx);
        });
        let gives = if refused { before.clone() } else { case["gives"].clone() };
        assert_eq!(Some(gives.clone()), host.read("album.genres"), "{item} {typed:?}");
        let t = tags(&screen);
        assert_eq!(refused, t.bad(), "{item} {typed:?}");
        assert_eq!(if refused { typed.as_str() } else { "" }, t.editor().unwrap().text(), "{item} {typed:?}");
        let grew = gives.as_array().unwrap().len() != before.as_array().unwrap().len();
        assert_eq!(if grew { 2 } else { 0 }, host.logged().len(), "{item} {typed:?}");
    }
}

#[test]
fn its_chips_show_the_items_as_on_every_renderer() {
    for case in cases()["shown"].as_array().unwrap() {
        let item = case["item"].as_str().unwrap();
        let host = host_of(item, case["items"].clone());
        if let Some(labels) = case["labels"].as_object() {
            let offered: Vec<(Value, &str)> = labels.iter().map(|(v, l)| (json!(v), l.as_str().unwrap())).collect();
            host.offer("album.genres", &offered);
        }
        let screen = make(host, MARKUP);
        let shown: Vec<String> = tags(&screen).chips().iter().map(|c| c.label.clone()).collect();
        assert_eq!(case["text"].as_str().unwrap(), shown.join(", "), "{item}");
    }
}

#[test]
fn an_enum_offers_the_values_the_list_has_not() {
    let host = host_of("enum", json!(["Rock"]));
    host.offer("album.genres", &[(json!("Jazz"), "Jazz music"), (json!("Rock"), "Rock"), (json!("Folk"), "Folk")]);
    let mut screen = make(host.clone(), MARKUP);
    let t = tags(&screen);
    assert!(t.editor().is_none());
    assert_eq!(vec!["Rock"], t.chips().iter().map(|c| c.label.as_str()).collect::<Vec<_>>());
    assert_eq!(vec!["Jazz music", "Folk"], t.offered().into_iter().map(|c| c.label).collect::<Vec<_>>());
    let adder = t.adder().expect("a place to pick a value");
    click(&mut screen, adder);
    assert_eq!(1, screen.overlays().len());
    let place = screen.overlays()[0].place;
    click(&mut screen, Rect::new(place.x, place.y + 1.0 + LINE, place.width, LINE));
    assert!(screen.overlays().is_empty());
    assert_eq!(Some(json!(["Rock", "Folk"])), host.read("album.genres"));
    assert_eq!(vec![r#"write genres album.genres ["Rock","Folk"]"#, "run genres change genresChanged"], host.logged());
    assert_eq!(vec!["Jazz music"], tags(&screen).offered().into_iter().map(|c| c.label).collect::<Vec<_>>());
}

#[test]
fn keys_open_an_enums_values_and_pick_one() {
    let host = host_of("enum", json!([]));
    let mut screen = make(host.clone(), MARKUP);
    screen.key_pressed(KeyInput::plain(Key::Tab));
    screen.key_pressed(KeyInput::plain(Key::Down));
    assert_eq!(1, screen.overlays().len());
    screen.key_pressed(KeyInput::plain(Key::Down));
    screen.key_pressed(KeyInput::plain(Key::Enter));
    assert!(screen.overlays().is_empty());
    assert_eq!(Some(json!(["Rock"])), host.read("album.genres"));
}

#[test]
fn a_chip_removes_its_item_and_a_read_only_one_changes_nothing() {
    let host = host_of("text", json!(["jazz", "live"]));
    let mut screen = make(host.clone(), MARKUP);
    let first = tags(&screen).chips()[0].clone();
    assert_eq!(Some("Remove".to_string()), first.remove_name);
    click(&mut screen, first.remove.expect("a remove button"));
    assert_eq!(Some(json!(["live"])), host.read("album.genres"));
    assert_eq!(vec!["live"], tags(&screen).chips().iter().map(|c| c.label.as_str()).collect::<Vec<_>>());

    let host = host_of("text", json!(["live"]));
    let mut screen = make(host.clone(), r#"<Tags at="1,1" span="8" name="genres" bind="album.genres" readonly="true" />"#);
    assert!(tags(&screen).chips()[0].remove.is_none());
    assert!(tags(&screen).editor().is_none());
    act(&mut screen, |t, cx| t.take(cx));
    assert!(host.logged().is_empty());
}

#[test]
fn enter_adds_what_is_typed_and_it_draws() {
    let host = host_of("text", json!(["jazz"]));
    let mut screen = make(host.clone(), MARKUP);
    let id = tags(&screen).box_id();
    screen.focus(id);
    screen.text_typed("live");
    screen.key_pressed(KeyInput::plain(Key::Enter));
    assert_eq!(Some(json!(["jazz", "live"])), host.read("album.genres"));
    assert_eq!("", tags(&screen).editor().unwrap().text());
    let mut shot = Shot::of(&mut screen, 800, 400, 1.0);
    keep(&mut shot, "inputs-tags.png");
    let t = tags(&screen);
    assert!(t.chips()[1].bounds.x > t.chips()[0].bounds.right());
    assert!(t.editor().unwrap().node().bounds.x > t.chips()[1].bounds.right());
}

#[test]
fn the_place_to_add_is_as_high_as_an_input_box_and_the_chips_sit_in_its_middle() {
    let screen = make(host_of("text", json!(["jazz"])), MARKUP);
    let t = tags(&screen);
    let place = t.editor().unwrap().node().bounds;
    assert_eq!(ROW_HEIGHT, place.height);
    assert_eq!(t.node().bounds.y, place.y);
    let chip = t.chips()[0].bounds;
    assert!((place.y + place.height / 2.0 - (chip.y + chip.height / 2.0)).abs() < 3.0);
    let screen = make(host_of("enum", json!(["Jazz"])), MARKUP);
    assert_eq!(ROW_HEIGHT, tags(&screen).adder().unwrap().height);
}

#[test]
fn its_box_shows_its_own_placeholder_else_clarity_s_word_for_adding() {
    let screen = make(host_of("text", json!([])), MARKUP);
    assert_eq!(Some("Add".to_string()), tags(&screen).editor().and_then(|e| e.placeholder.clone()));
    let own = r#"<Tags at="1,1" span="8" name="genres" bind="album.genres" placeholder="A genre, then Enter" />"#;
    let screen = make(host_of("text", json!([])), own);
    assert_eq!(Some("A genre, then Enter".to_string()), tags(&screen).editor().and_then(|e| e.placeholder.clone()));
}
