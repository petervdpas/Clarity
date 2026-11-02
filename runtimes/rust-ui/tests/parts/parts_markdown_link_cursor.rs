//! The pointer over a Markdown link is the hand, as Avalonia's Markdown view
//! showed it and the C# MarkdownLinkCursorTests check; over plain text it is
//! the arrow.

use clarity_ui::input::Cursor;
use clarity_ui::parts::containers::markdown::{MarkdownPart, MarkdownRun};
use clarity_ui::testing::{part_named, window_of, Shot, TestHost};
use serde_json::json;

#[test]
fn the_hand_shows_over_a_link_and_not_over_its_text() {
    let host = TestHost::new();
    host.set("notes", json!("See [the site](https://example.com) now."));
    let mut screen = window_of(r#"<Markdown name="rules" at="1,1" span="6" rows="4" bind="notes" />"#, host);
    Shot::of(&mut screen, 800, 400, 1.0);
    let id = part_named(&screen, "rules");
    let part = screen.element(id).unwrap().as_any().downcast_ref::<MarkdownPart>().unwrap();
    let page = part.page();
    let on = |run: &MarkdownRun| (page.x + run.place.x + 2.0, page.y + run.place.y + run.place.height / 2.0);
    let runs = part.runs();
    let (lx, ly) = on(runs.iter().find(|r| r.link.is_some()).unwrap());
    let (tx, ty) = on(runs.iter().find(|r| r.text.starts_with("See")).unwrap());
    screen.pointer_moved(lx, ly);
    assert_eq!(Cursor::Hand, screen.cursor());
    screen.pointer_moved(tx, ty);
    assert_eq!(Cursor::Default, screen.cursor());
}
