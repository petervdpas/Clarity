//! The widths a person gives a table's columns by dragging a header edge are
//! kept for the next start, per window and table (C# TableWidthTests; key
//! `columns.<Window>.<table>` among the kept choices), and a double click on
//! an edge gives the designed widths back and forgets the person's.

use super::data_rows_host;

use std::rc::Rc;

use clarity_ui::choices;
use clarity_ui::element::find_type;
use clarity_ui::input::PointerButton;
use clarity_ui::parts::data::table::TablePart;
use clarity_ui::screen::Screen;
use clarity_ui::testing::window_of;
use data_rows_host::{laid, DataHost};
use serde_json::json;

const GRID: &str = r#"<Table at="1,1" span="12" rows="6" name="grid" bind="orders" columns="title 150, hours" resizable="true" />"#;

fn table(screen: &Screen) -> &TablePart {
    find_type::<TablePart>(screen.root(), |_| true).expect("a table")
}

fn host(window: Option<&str>) -> Rc<DataHost> {
    let host = DataHost::new();
    host.set("orders", json!([{ "title": "One", "hours": 1 }]));
    *host.window.borrow_mut() = window.map(str::to_string);
    host
}

fn shown(window: Option<&str>, markup: &str) -> Screen {
    let mut screen = window_of(markup, host(window));
    laid(&mut screen, 800, 400);
    screen
}

fn title_width(screen: &Screen) -> f64 {
    table(screen).header_box("title").unwrap().width
}

#[test]
fn the_persons_widths_are_remembered_per_window_and_table_and_a_reset_gives_the_designed_ones() {
    choices::forget("columns.Other.grid");
    choices::keep_value("columns.Main.grid", r#"{"title":220,"hours":90}"#);
    let mut screen = shown(Some("Main"), GRID);
    assert_eq!(220.0, title_width(&screen));
    assert_eq!(90.0, table(&screen).header_box("hours").unwrap().width);
    assert_eq!(150.0, title_width(&shown(Some("Other"), GRID)));

    // A double click on an edge: the designed widths, and none kept.
    let edge = table(&screen).header_box("title").unwrap();
    screen.pointer_pressed(edge.right() - 2.0, edge.y + 10.0, PointerButton::Left, 2);
    screen.pointer_released(edge.right() - 2.0, edge.y + 10.0, PointerButton::Left);
    laid(&mut screen, 800, 400);
    assert_eq!(150.0, title_width(&screen));
    assert_eq!(150.0, title_width(&shown(Some("Main"), GRID)));
    assert!(choices::kept("columns.Main.grid").unwrap_or_default().is_empty());
}

#[test]
fn widths_the_person_drags_are_kept_when_the_edge_is_let_go() {
    choices::forget("columns.Drag.grid");
    let mut screen = shown(Some("Drag"), GRID);
    let title = table(&screen).header_box("title").unwrap();
    screen.pointer_pressed(title.right() - 2.0, title.y + 10.0, PointerButton::Left, 1);
    screen.pointer_moved(title.right() + 30.0, title.y + 10.0);
    // Kept only once the edge is let go.
    assert!(choices::kept("columns.Drag.grid").unwrap_or_default().is_empty());
    screen.pointer_released(title.right() + 30.0, title.y + 10.0, PointerButton::Left);
    assert_eq!(182.0, title_width(&shown(Some("Drag"), GRID)));
    // A table of a window without a name, or without a name of its own, keeps nothing.
    let unnamed = r#"<Table at="1,1" span="12" rows="6" bind="orders" columns="title 150, hours" resizable="true" />"#;
    for (window, markup) in [(None, GRID), (Some("Drag"), unnamed)] {
        let before = choices::kept("columns.Drag.grid");
        let mut other = shown(window, markup);
        let b = table(&other).header_box("title").unwrap();
        other.pointer_pressed(b.right() - 2.0, b.y + 10.0, PointerButton::Left, 1);
        other.pointer_moved(b.right() + 60.0, b.y + 10.0);
        other.pointer_released(b.right() + 60.0, b.y + 10.0, PointerButton::Left);
        laid(&mut other, 800, 400);
        assert_eq!(212.0, title_width(&other));
        assert_eq!(before, choices::kept("columns.Drag.grid"));
    }
}
