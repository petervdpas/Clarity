//! `striped`: a table, list, tree or tree table draws every other row (the
//! 2nd, the 4th, ...) with a faint stripe of the text color, in the light
//! look and the dark one. A chosen row shows the selection as it does
//! without stripes, and a list of cards has no stripes.

use super::data_rows_host;

use clarity_ui::element::find_type;
use clarity_ui::geometry::Rect;
use clarity_ui::parts::data::list::ListPart;
use clarity_ui::parts::data::table::TablePart;
use clarity_ui::screen::Screen;
use clarity_ui::testing::window_of;
use clarity_ui::theme::ThemeLook;
use data_rows_host::{laid, DataHost};
use serde_json::{json, Value};
use skia_safe::Color;

fn folders(count: usize) -> Value {
    Value::Array((1..=count).map(|i| json!({ "id": i, "name": format!("Folder {i}"), "position": i })).collect())
}

/// The window of one part with four rows, the row at `chosen` chosen.
fn made(part: &str, dark: bool, chosen: Option<usize>) -> Screen {
    let host = DataHost::new();
    host.set("folders", folders(4));
    let mut screen = window_of(&format!(r#"{part} name="folders" bind="folders" />"#), host.clone());
    screen.show(ThemeLook::clarity(dark));
    host.choose("folders", chosen);
    laid(&mut screen, 800, 400);
    screen
}

fn row(screen: &Screen, place: usize) -> Rect {
    let found = match find_type::<TablePart>(screen.root(), |_| true) {
        Some(table) => table.row_box(place),
        None => find_type::<ListPart>(screen.root(), |_| true).expect("a table or a list").row_box(place),
    };
    found.unwrap_or_else(|| panic!("row {place} does not show"))
}

/// The color of each of the first two rows past its text.
fn backs(part: &str, dark: bool, chosen: Option<usize>) -> (Color, Color) {
    let mut screen = made(part, dark, chosen);
    let (first, second) = (row(&screen, 0), row(&screen, 1));
    let mut shot = laid(&mut screen, 800, 400);
    let at = |r: Rect| ((r.x + 200.0) as i32, (r.y + r.height / 2.0) as i32);
    let (a, b) = (at(first), at(second));
    (shot.pixel(a.0, a.1), shot.pixel(b.0, b.1))
}

const PARTS: [&str; 4] = [
    r#"<Table at="1,1" span="12" rows="6" columns="name, id""#,
    r#"<List at="1,1" span="6" rows="6" show="name""#,
    r#"<Tree at="1,1" span="6" rows="6" show="name""#,
    r#"<TreeTable at="1,1" span="12" rows="6" columns="name, id""#,
];

fn faint(a: Color, b: Color) -> bool {
    let apart = |x: u8, y: u8| (x as i32 - y as i32).abs();
    a != b && apart(a.r(), b.r()) <= 24 && apart(a.g(), b.g()) <= 24 && apart(a.b(), b.b()) <= 24
}

#[test]
fn every_other_row_has_a_faint_stripe_in_light_and_dark() {
    for part in PARTS {
        for dark in [false, true] {
            let (first, second) = backs(&format!(r#"{part} striped="true""#), dark, None);
            assert!(faint(first, second), "{part} dark {dark}: the 2nd row {second:?} beside the 1st {first:?}");
            let (plain_first, plain_second) = backs(part, dark, None);
            assert_eq!(plain_first, plain_second, "{part} dark {dark}: without striped the rows are alike");
            assert_eq!(plain_first, first, "{part} dark {dark}: the 1st row has no stripe");
        }
    }
}

#[test]
fn a_chosen_striped_row_shows_the_selection_as_without_stripes() {
    for part in PARTS {
        for dark in [false, true] {
            let (_, striped) = backs(&format!(r#"{part} striped="true""#), dark, Some(1));
            let (_, plain) = backs(part, dark, Some(1));
            assert_eq!(plain, striped, "{part} dark {dark}");
        }
    }
}

#[test]
fn cards_have_no_stripes() {
    let part = r#"<List at="1,1" span="6" rows="8" show="name, position" cards="true""#;
    let (first, second) = backs(&format!(r#"{part} striped="true""#), false, None);
    assert_eq!(first, second);
}
