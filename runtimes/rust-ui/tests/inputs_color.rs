//! The color part against the cases every renderer shares
//! (conformance/colors.json): colors typed into its box, picked from the
//! palette its swatch opens. Only a color that differs is a change; text
//! that is no color stays as typed and is marked, and the value stays.

use std::rc::Rc;

use clarity_ui::input::{Key, KeyInput, PointerButton};
use clarity_ui::parts::inputs::color::{read_color, ColorPart, PaletteGrid};
use clarity_ui::parts::inputs::testing::InputsHost;
use clarity_ui::parts::PartHost;
use clarity_ui::testing::{part_named, window_of, Shot};
use clarity_ui::{Element, Screen};
use serde_json::{json, Value};

const MARKUP: &str = r#"<Color at="1,1" span="4" name="tint" bind="tag.color" onchange="tintChanged" />"#;

fn colors() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../conformance/colors.json");
    serde_json::from_str(&std::fs::read_to_string(path).expect("conformance/colors.json")).expect("it is JSON")
}

fn make(value: &str, markup: &str) -> (Screen, Rc<InputsHost>) {
    let host = InputsHost::new();
    host.set("tag.color", json!(value));
    let mut screen = window_of(markup, host.clone());
    screen.resize(800.0, 400.0, 1.0);
    (screen, host)
}

fn part<'a>(screen: &'a Screen) -> &'a ColorPart {
    screen.element(part_named(screen, "tint")).and_then(|e| e.as_any().downcast_ref::<ColorPart>()).expect("a color part")
}

fn with_part<R>(screen: &mut Screen, act: impl FnOnce(&mut ColorPart) -> R) -> R {
    let id = part_named(screen, "tint");
    screen.act(id, |e, _| act(e.as_any_mut().downcast_mut::<ColorPart>().expect("a color part"))).expect("it is there")
}

fn click(screen: &mut Screen, x: f64, y: f64) {
    screen.pointer_pressed(x, y, PointerButton::Left, 1);
    screen.pointer_released(x, y, PointerButton::Left);
}

fn click_swatch(screen: &mut Screen) {
    let s = part(screen).swatch();
    click(screen, s.x + 4.0, s.y + 4.0);
}

fn palette(screen: &Screen) -> &PaletteGrid {
    screen.overlays()[0].content.as_any().downcast_ref::<PaletteGrid>().expect("the palette")
}

fn keep(shot: &mut Shot, name: &str) {
    if let Ok(dir) = std::env::var("CLARITY_SHOTS") {
        shot.save(std::path::Path::new(&dir).join(name)).expect("the picture is written");
    }
}

#[test]
fn a_typed_color_is_read_as_on_every_renderer() {
    let cases = colors();
    for case in cases["read"].as_array().unwrap() {
        let typed = case["typed"].as_str().unwrap();
        let color = case["color"].as_str();
        assert_eq!(color.map(str::to_string), read_color(typed), "{typed:?}");
        let (mut screen, host) = make("#123456", MARKUP);
        let bad = with_part(&mut screen, |p| {
            p.editor_mut().set_text(typed);
            p.take();
            p.is_bad()
        });
        assert_eq!(Some(json!(color.unwrap_or("#123456"))), host.read("tag.color"), "{typed:?}");
        assert_eq!(color.is_none(), bad, "{typed:?}");
        let wanted: Vec<String> = match color {
            None => Vec::new(),
            Some(c) => vec![format!("write tint tag.color {}", json!(c)), "run tint change tintChanged".to_string()],
        };
        assert_eq!(wanted, host.logged(), "{typed:?}");
        if color.is_none() {
            assert_eq!(typed, part(&screen).editor().text());
        }
    }
}

#[test]
fn the_palette_is_the_one_every_renderer_offers_and_opens_below_the_swatch() {
    let cases = colors();
    let rows: Vec<String> = cases["palette"].as_array().unwrap().iter().flat_map(|r| r.as_array().unwrap().iter().map(|c| c.as_str().unwrap().to_string())).collect();
    let (mut screen, _) = make("#123456", MARKUP);
    click_swatch(&mut screen);
    assert_eq!(1, screen.overlays().len());
    assert_eq!(rows, PaletteGrid::colors());
    assert_eq!(60, PaletteGrid::colors().len());
    assert!(screen.overlays()[0].place.y >= part(&screen).node().bounds.bottom());
    click_swatch(&mut screen);
    assert!(screen.overlays().is_empty());
}

#[test]
fn a_picked_color_is_a_change_only_when_it_differs() {
    let (mut screen, host) = make("#3b82f6", MARKUP);
    click_swatch(&mut screen);
    assert_eq!(Some("#3b82f6"), palette(&screen).chosen());
    let blue = palette(&screen).place("#3b82f6").unwrap();
    click(&mut screen, blue.x + blue.width / 2.0, blue.y + blue.height / 2.0);
    assert!(screen.overlays().is_empty());
    assert!(host.logged().is_empty());
    click_swatch(&mut screen);
    let red = palette(&screen).place("#ef4444").unwrap();
    click(&mut screen, red.x + red.width / 2.0, red.y + red.height / 2.0);
    assert!(screen.overlays().is_empty());
    assert_eq!(Some(json!("#ef4444")), host.read("tag.color"));
    let writes: Vec<String> = host.logged().into_iter().filter(|l| !l.contains(" enter ") && !l.contains(" leave ")).collect();
    assert_eq!(vec![r##"write tint tag.color "#ef4444""##, "run tint change tintChanged"], writes);
    assert_eq!("#ef4444", part(&screen).editor().text());
}

#[test]
fn what_code_sets_is_no_change_and_a_read_only_one_opens_nothing() {
    let (mut screen, host) = make("#123456", MARKUP);
    host.set("tag.color", json!("#000000"));
    Shot::of(&mut screen, 100, 40, 1.0);
    assert_eq!("#000000", part(&screen).editor().text());
    assert!(host.logged().is_empty());

    let (mut screen, host) = make("#123456", r#"<Color at="1,1" span="4" name="tint" bind="tag.color" readonly="true" />"#);
    click_swatch(&mut screen);
    assert!(screen.overlays().is_empty());
    with_part(&mut screen, |p| {
        p.editor_mut().set_text("#ffffff");
        p.take();
    });
    assert!(host.logged().iter().all(|l| !l.starts_with("write")));
}

#[test]
fn enter_takes_the_typed_color_and_escape_closes_the_palette() {
    let (mut screen, host) = make("#123456", MARKUP);
    let id = part_named(&screen, "tint");
    screen.focus(Some(id));
    with_part(&mut screen, |p| p.editor_mut().select_all());
    screen.text_typed("f0a");
    screen.key_pressed(KeyInput::plain(Key::Enter));
    assert_eq!(Some(json!("#ff00aa")), host.read("tag.color"));
    click_swatch(&mut screen);
    assert_eq!(1, screen.overlays().len());
    screen.key_pressed(KeyInput::plain(Key::Escape));
    assert!(screen.overlays().is_empty());
    // Down in the input opens it too.
    screen.key_pressed(KeyInput::plain(Key::Down));
    assert_eq!(1, screen.overlays().len());
}

#[test]
fn an_unbound_named_color_keeps_its_color_in_its_state() {
    let host = InputsHost::new();
    let mut screen = window_of(r#"<Color at="1,1" span="4" name="tint" onchange="tintChanged" />"#, host.clone());
    screen.resize(800.0, 400.0, 1.0);
    with_part(&mut screen, |p| {
        p.editor_mut().set_text("#abc");
        p.take();
    });
    assert_eq!(Some("#aabbcc".to_string()), host.state(Some("tint")).unwrap().text);
    assert_eq!(vec!["tint".to_string()], host.edits.borrow().clone());
}

#[test]
fn the_swatch_shows_the_color_picked() {
    let (mut screen, _) = make("#ff0000", MARKUP);
    let mut shot = Shot::of(&mut screen, 800, 400, 1.0);
    let s = part(&screen).swatch();
    let middle = shot.pixel((s.x + s.width / 2.0) as i32, (s.y + s.height / 2.0) as i32);
    assert_eq!((255, 0, 0), (middle.r(), middle.g(), middle.b()));
}

#[test]
fn it_draws_closed_and_with_its_palette_open() {
    let (mut screen, _) = make("#3b82f6", MARKUP);
    click_swatch(&mut screen);
    let mut shot = Shot::of(&mut screen, 400, 260, 1.0);
    keep(&mut shot, "inputs-color.png");
}
