//! The line between two headers is grabbed from either side of it, as a
//! grid's resize grip straddles it (C# TableEdgeTests): a press just right
//! of the line resizes the column left of it, not sorts the one it is in.

use super::data_rows_host;

use clarity_ui::element::find_type;
use clarity_ui::input::{Cursor, PointerButton};
use clarity_ui::parts::data::table::TablePart;
use clarity_ui::screen::Screen;
use clarity_ui::testing::window_of;
use data_rows_host::{laid, DataHost};
use serde_json::json;

fn table(screen: &Screen) -> &TablePart {
    find_type::<TablePart>(screen.root(), |_| true).expect("a table")
}

#[test]
fn a_press_just_right_of_a_header_line_resizes_the_column_left_of_it() {
    let host = DataHost::new();
    host.set("orders", json!([{ "title": "b", "hours": 2 }, { "title": "a", "hours": 1 }]));
    let mut screen = window_of(r#"<Table at="1,1" span="12" rows="6" name="orders" bind="orders" columns="title 150, hours" resizable="true" sortable="true" />"#, host.clone());
    laid(&mut screen, 800, 400);
    let title = table(&screen).header_box("title").unwrap();
    screen.pointer_moved(title.right() + 3.0, title.y + 10.0);
    assert_eq!(Cursor::ResizeLeftRight, screen.cursor());
    screen.pointer_pressed(title.right() + 3.0, title.y + 10.0, PointerButton::Left, 1);
    screen.pointer_moved(title.right() + 53.0, title.y + 10.0);
    screen.pointer_released(title.right() + 53.0, title.y + 10.0, PointerButton::Left);
    assert_eq!(200.0, table(&screen).header_box("title").unwrap().width);
    // No sort: the rows keep their order.
    assert_eq!(json!("b"), host.get("orders[0].title"));
    assert!(table(&screen).order().is_none());
    assert_eq!(vec![0, 1], table(&screen).shown_places());
}
