//! What a table says where the pointer rests, in Clarity's own words (C#
//! TableTipTests): the buttons of its page bar (First page, Next page...)
//! and a tree row's toggle (Expand, Collapse); and the pointer's shape over
//! its header edges and buttons.

use super::data_rows_host;

use std::time::Instant;

use clarity_ui::element::{find_type, Element};
use clarity_ui::input::Cursor;
use clarity_ui::parts::data::table::{PageButton, TablePart};
use clarity_ui::screen::{Screen, TIP_PAUSE};
use clarity_ui::testing::{part_named, window_of};
use data_rows_host::{laid, DataHost};
use serde_json::{json, Value};

fn table(screen: &Screen) -> &TablePart {
    find_type::<TablePart>(screen.root(), |_| true).expect("a table")
}

#[test]
fn the_page_bar_buttons_say_what_they_do_and_show_a_hand() {
    let host = DataHost::new();
    host.set("albums", Value::Array((1..=7).map(|i| json!({ "title": format!("Album {i}") })).collect()));
    let mut screen = window_of(r#"<Table at="1,1" span="12" rows="6" name="albums" bind="albums" columns="title 200" page="3" resizable="true" />"#, host);
    laid(&mut screen, 800, 400);
    let now = Instant::now();
    screen.now = Box::new(move || now);
    let next = table(&screen).pager_box(PageButton::Next).unwrap();
    screen.pointer_moved(next.x + next.width / 2.0, next.y + next.height / 2.0);
    screen.wake_due(now + TIP_PAUSE);
    assert_eq!(Some("Next page"), screen.tip().map(|t| t.text.as_str()));
    let first = table(&screen).pager_box(PageButton::First).unwrap();
    assert_eq!(Some("First page".to_string()), table(&screen).tooltip_at(first.x + 4.0, first.y + 4.0));
    assert_eq!(Some(Cursor::Hand), table(&screen).cursor_at(next.x + 4.0, next.y + 4.0));
    // A header edge of a resizable table shows the pointer that resizes.
    let header = table(&screen).header_box("title").unwrap();
    assert_eq!(Some(Cursor::ResizeLeftRight), table(&screen).cursor_at(header.right() - 2.0, header.y + 5.0));
    assert_eq!(None, table(&screen).cursor_at(header.x + 20.0, header.y + 5.0));
    assert_eq!(None, table(&screen).tooltip_at(header.x + 20.0, header.y + 5.0));
}

#[test]
fn a_tree_toggle_says_whether_it_expands_or_collapses() {
    let host = DataHost::new();
    host.set("folders", json!([{ "id": 1, "name": "Documents", "parent": null }, { "id": 2, "name": "Invoices", "parent": 1 }]));
    let mut screen = window_of(r#"<Tree at="1,1" span="12" rows="8" name="folders" bind="folders" show="name" />"#, host);
    laid(&mut screen, 800, 400);
    let now = Instant::now();
    screen.now = Box::new(move || now);
    let toggle = table(&screen).toggle_box(0).unwrap();
    let (x, y) = (toggle.x + toggle.width / 2.0, toggle.y + toggle.height / 2.0);
    screen.pointer_moved(x, y);
    screen.wake_due(now + TIP_PAUSE);
    assert_eq!(Some("Expand"), screen.tip().map(|t| t.text.as_str()));
    let id = part_named(&screen, "folders");
    screen.act(id, |e, _| e.as_any_mut().downcast_mut::<TablePart>().unwrap().toggle(0));
    laid(&mut screen, 800, 400);
    assert_eq!(Some("Collapse".to_string()), table(&screen).tooltip_at(x, y));
    assert_eq!(Some(Cursor::Hand), table(&screen).cursor_at(x, y));
    let name = table(&screen).cell_box(1, "name").unwrap();
    assert_eq!(None, table(&screen).tooltip_at(name.right() - 10.0, name.y + 5.0));
}
