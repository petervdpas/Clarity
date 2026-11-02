//! Columns wider together than the table scroll sideways (C#
//! TableSidewaysTests): a bar across, the wheel turned sideways (or with
//! Shift), the header going with the rows, and the cell typed into
//! scrolled into sight.

use super::data_rows_host;

use clarity_ui::element::find_type;
use clarity_ui::input::{Key, KeyInput, Modifiers};
use clarity_ui::parts::data::table::TablePart;
use clarity_ui::screen::Screen;
use clarity_ui::testing::window_of;
use data_rows_host::{click, laid, DataHost};
use serde_json::json;

fn table(screen: &Screen) -> &TablePart {
    find_type::<TablePart>(screen.root(), |_| true).expect("a table")
}

fn wide(columns: &str) -> Screen {
    let host = DataHost::new();
    host.set("orders", json!([{ "title": "One", "hours": 1, "owner": 3 }, { "title": "Two", "hours": 2, "owner": null }]));
    host.types.borrow_mut().insert("orders.owner".into(), "integer".into());
    let markup = format!(r#"<Table at="1,1" span="12" rows="6" name="orders" bind="orders" columns="{columns}" edit="title, owner" />"#);
    let mut screen = window_of(&markup, host);
    laid(&mut screen, 800, 400);
    screen
}

#[test]
fn columns_wider_than_the_table_scroll_sideways_with_their_header() {
    let mut screen = wide("title 400, hours 400, owner 300");
    let view = table(&screen).scroller().clone();
    assert_eq!(1100.0, view.content_width);
    assert!(view.max_x() > 0.0);
    assert!(table(&screen).header_box("owner").unwrap().right() > view.bounds.right());
    let title = table(&screen).header_box("title").unwrap();
    // Shift and the wheel scroll sideways, and the header goes with the rows.
    screen.wheel(title.x + 20.0, title.bottom() + 20.0, 0.0, 100.0, Modifiers::SHIFT);
    assert_eq!(100.0, table(&screen).scroller().scrolled_x());
    assert_eq!(title.x - 100.0, table(&screen).header_box("title").unwrap().x);
    assert_eq!(table(&screen).header_box("title").unwrap().x, table(&screen).cell_box(0, "title").unwrap().x);
    // A wheel that turns sideways by itself too, over the header as over the rows.
    screen.wheel(title.x + 200.0, title.y + 5.0, 50.0, 0.0, Modifiers::NONE);
    assert_eq!(150.0, table(&screen).scroller().scrolled_x());
    laid(&mut screen, 800, 400);
}

#[test]
fn the_cell_typed_into_scrolls_into_sight() {
    let mut screen = wide("title 400, hours 400, owner 300");
    let b = table(&screen).cell_box(0, "title").unwrap();
    click(&mut screen, b);
    screen.key_pressed(KeyInput::new(Key::Tab, Modifiers::NONE));
    laid(&mut screen, 800, 400);
    let editor = clarity_ui::element::Element::node(table(&screen).cell_editor().expect("the owner is typed into")).bounds;
    let view = table(&screen).scroller().bounds;
    assert!(table(&screen).scroller().scrolled_x() > 0.0);
    assert!(editor.right() <= view.right() + 0.5 && editor.x >= view.x - 0.5, "editor at {editor:?}, view {view:?}");
}

#[test]
fn columns_that_fit_do_not_scroll_sideways() {
    let screen = wide("title 200, hours");
    assert_eq!(0.0, table(&screen).scroller().max_x());
    let right = table(&screen).header_box("hours").unwrap().right();
    assert!((table(&screen).scroller().bounds.right() - right).abs() <= 3.0, "{right} against {:?}", table(&screen).scroller().bounds);
}
