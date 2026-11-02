//! The display part, as the C# DisplayPartTests check it: its text and
//! segments against the cases every renderer shares
//! (conformance/display.json), and the digits drawn in its place.

use std::rc::Rc;

use clarity_ui::parts::containers::display::{self, DisplayPart};
use clarity_ui::testing::{near, part_named, window_of, Shot, TestHost};
use clarity_ui::{Rect, Screen};
use serde_json::{json, Value};

fn host() -> Rc<TestHost> {
    TestHost::new()
}

fn conformance(file: &str) -> Value {
    let path = format!("{}/../../conformance/{file}", env!("CARGO_MANIFEST_DIR"));
    serde_json::from_str(&std::fs::read_to_string(path).expect("the conformance file reads")).expect("the conformance file is JSON")
}

fn window(markup: &str, host: Rc<TestHost>, width: f64, height: f64) -> Screen {
    let mut screen = window_of(markup, host);
    screen.resize(width, height, 1.0);
    screen
}

fn part<'a, T: 'static>(screen: &'a Screen, name: &str) -> &'a T {
    screen.element(part_named(screen, name)).and_then(|e| e.as_any().downcast_ref::<T>()).expect("a part of that kind")
}

fn bounds(screen: &Screen, name: &str) -> Rect {
    screen.element(part_named(screen, name)).unwrap().node().bounds
}

fn keep(shot: &mut Shot, name: &str) {
    if let Ok(dir) = std::env::var("CLARITY_SHOTS") {
        shot.save(std::path::Path::new(&dir).join(name)).expect("the picture is written");
    }
}

#[test]
fn a_value_shows_rounded_to_fit_its_digits_or_as_e() {
    let file = conformance("display.json");
    for case in file["shows"].as_array().unwrap() {
        let shown = display::shown_text(case["value"].as_str().unwrap(), case["digits"].as_u64().unwrap() as usize);
        assert_eq!(case["text"].as_str().unwrap(), shown, "{case}");
    }
}

#[test]
fn the_text_fills_the_cells_from_the_right_a_point_lit_on_its_digit() {
    let file = conformance("display.json");
    for case in file["cells"].as_array().unwrap() {
        let expected: Vec<(char, bool)> =
            case["cells"].as_array().unwrap().iter().map(|c| (c[0].as_str().unwrap().chars().next().unwrap(), c[1].as_bool().unwrap())).collect();
        assert_eq!(expected, display::cells(case["text"].as_str().unwrap(), case["digits"].as_u64().unwrap() as usize), "{case}");
    }
}

#[test]
fn each_character_lights_its_segments() {
    let file = conformance("display.json");
    for (character, lit) in file["segments"].as_object().unwrap() {
        assert_eq!(lit.as_str().unwrap(), display::segments(character.chars().next().unwrap()), "{character}");
    }
}

#[test]
fn a_display_shows_what_it_binds_in_its_place() {
    let host = host();
    host.set("total", json!(12.5));
    let mut screen = window(r#"<Display name="shown" at="1,1" span="4" rows="2" bind="total" digits="4" />"#, host.clone(), 808.0, 400.0);
    assert_eq!("12.5", part::<DisplayPart>(&screen, "shown").text());
    host.set("total", json!(123456));
    assert_eq!("E", part::<DisplayPart>(&screen, "shown").text());
    host.set("total", Value::Null);
    assert_eq!("0", part::<DisplayPart>(&screen, "shown").text());
    host.set("total", json!("8"));
    let mut shot = Shot::of(&mut screen, 808, 400, 1.0);
    let (left, top, scale) = display::fit(bounds(&screen, "shown"), 5);
    let text = screen.look().color("text");
    // The last cell's middle segment is lit in the text's color, on the right of its place.
    let middle = shot.pixel((left + (6.0 + 4.0 * 13.0) * scale) as i32, (top + 10.0 * scale) as i32);
    assert!(near(middle, text, 8), "{middle:?}");
    // The first cell is dark: only faintly drawn.
    let dark = shot.pixel((left + 6.0 * scale) as i32, (top + 10.0 * scale) as i32);
    assert!(!near(dark, text, 60), "{dark:?}");
    keep(&mut shot, "display.png");
}
