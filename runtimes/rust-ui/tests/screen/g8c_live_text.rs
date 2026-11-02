//! The text part: a bound value to read, on one line cut off with an
//! ellipsis, or over several rows wrapped from the top (as C# TextPart).

use clarity_ui::parts::containers::text::TextPart;
use clarity_ui::parts::MissingPart;
use clarity_ui::testing::{near, part_named, window_of, Shot, TestHost};
use clarity_ui::{Element, Rect, Screen};
use serde_json::json;

const LONG: &str = "A note about this customer that is far too long to fit on one narrow line of a window";

fn text_part<'a>(screen: &'a Screen, name: &str) -> &'a TextPart {
    screen.element(part_named(screen, name)).unwrap().as_any().downcast_ref::<TextPart>().expect("a text part")
}

/// The rows of a place that hold a pixel near `color`.
fn inked_rows(shot: &mut Shot, place: Rect, color: skia_safe::Color) -> Vec<i32> {
    (place.y as i32..place.bottom() as i32).filter(|y| (place.x as i32..place.right() as i32).any(|x| near(shot.pixel(x, *y), color, 60))).collect()
}

#[test]
fn markup_text_makes_a_text_part() {
    let host = TestHost::new();
    host.set("customer.notes", json!("Pays on time"));
    let screen = window_of(r#"<Text at="1,1" span="6" name="notesText" bind="customer.notes" />"#, host);
    let element = screen.element(part_named(&screen, "notesText")).unwrap();
    assert!(element.as_any().downcast_ref::<MissingPart>().is_none());
    assert_eq!("Pays on time", text_part(&screen, "notesText").text());
}

#[test]
fn a_value_is_shown_as_the_person_reads_it() {
    let host = TestHost::new();
    host.set("customer.vip", json!(true));
    host.set("customer.count", json!(1250));
    let screen = window_of(
        r#"<Text at="1,1" span="6" name="vip" bind="customer.vip" />
           <Text at="1,2" span="6" name="count" bind="customer.count" />
           <Text at="1,3" span="6" name="none" bind="customer.none" />"#,
        host,
    );
    assert_eq!("Yes", text_part(&screen, "vip").text());
    assert_eq!("1,250", text_part(&screen, "count").text());
    assert_eq!("", text_part(&screen, "none").text());
}

#[test]
fn one_row_cuts_a_long_value_off_with_an_ellipsis() {
    let host = TestHost::new();
    host.set("customer.notes", json!(LONG));
    let mut screen = window_of(r#"<Text at="1,1" span="2" name="notesText" bind="customer.notes" />"#, host);
    let mut shot = Shot::of(&mut screen, 600, 300, 1.0);
    let part = text_part(&screen, "notesText");
    assert!(part.lines().is_none());
    let bounds = part.node().bounds;
    let text = screen.look().color("text");
    let rows = inked_rows(&mut shot, bounds, text);
    assert!(!rows.is_empty(), "the value is drawn");
    // One line: every inked row lies within one text height.
    assert!(rows.last().unwrap() - rows.first().unwrap() < 24);
    let fonts = clarity_ui::fonts::Fonts::new(screen.look().clone());
    let mut surface = skia_safe::surfaces::raster_n32_premul((10, 10)).unwrap();
    let painter = clarity_ui::Painter::new(surface.canvas(), screen.look(), &fonts);
    let cut = painter.fit(LONG, bounds.width, screen.look().size("text"), "text", false);
    assert!(cut.ends_with('\u{2026}'));
    // Nothing is drawn past the part's right edge.
    let right = Rect::new(bounds.right() + 1.0, bounds.y, 40.0, bounds.height);
    assert!(inked_rows(&mut shot, right, text).is_empty());
    shot.save("/tmp/claude-1000/-home-peter-Projects-Clarity/271a7793-55cd-491f-b56c-9578fe570b4e/scratchpad/shots/g8c-core-text-1.png").unwrap();
}

#[test]
fn several_rows_wrap_a_long_value_from_the_top() {
    let host = TestHost::new();
    host.set("customer.notes", json!(LONG));
    let mut screen = window_of(r#"<Text at="1,1" span="3" rows="4" name="notesText" bind="customer.notes" />"#, host);
    let mut shot = Shot::of(&mut screen, 600, 300, 1.0);
    let part = text_part(&screen, "notesText");
    let lines = part.lines().expect("several rows hold wrapped lines");
    assert!(!lines.takes_focus && !lines.framed && lines.read_only);
    assert_eq!(LONG, lines.text());
    let bounds = part.node().bounds;
    let rows = inked_rows(&mut shot, bounds, screen.look().color("text"));
    assert!(rows.last().unwrap() - rows.first().unwrap() > 30, "the text runs over more than one line");
    assert!(*rows.first().unwrap() < bounds.y as i32 + 24, "it starts at the top");
    shot.save("/tmp/claude-1000/-home-peter-Projects-Clarity/271a7793-55cd-491f-b56c-9578fe570b4e/scratchpad/shots/g8c-core-text-2.png").unwrap();
}

#[test]
fn a_text_follows_its_value_and_takes_no_focus() {
    let host = TestHost::new();
    host.set("customer.notes", json!("Old"));
    let mut screen = window_of(r#"<Text at="1,1" span="3" rows="3" name="notesText" bind="customer.notes" />"#, host.clone());
    let _ = Shot::of(&mut screen, 600, 300, 1.0);
    host.set("customer.notes", json!("New note"));
    let _ = Shot::of(&mut screen, 600, 300, 1.0);
    let part = text_part(&screen, "notesText");
    assert_eq!("New note", part.lines().unwrap().text());
    assert!(!part.focusable());
}
