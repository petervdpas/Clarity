//! A part that holds a grid of its own (a splitter's panes, a tabs page under
//! its strip, a panel under its header) adds room around that grid: the
//! window's grid gives its rows that room too, so the inner grid fits and only
//! the window scrolls.

use clarity_ui::element::{Cx, Element, GridPanel, Node};
use clarity_ui::element_node;
use clarity_ui::grid::{self, GridItem, GAP};
use clarity_ui::Rect;

fn around(item: GridItem, around: f64) -> GridItem {
    GridItem { around, ..item }
}

#[test]
fn a_growing_part_too_low_gets_its_rows_and_the_room_around_them() {
    let placed = grid::place(&[around(GridItem::new(1, 1, 12, 3, true), 20.0)], 12, 600.0, 100.0, GAP);
    assert_eq!(placed.boxes[0].height, 3.0 * 36.0 + 2.0 * 8.0 + 20.0);
    assert_eq!(placed.content, 3.0 * 36.0 + 2.0 * 8.0 + 20.0);
}

#[test]
fn a_growing_part_with_room_enough_fills_its_place() {
    let placed = grid::place(&[around(GridItem::new(1, 1, 12, 3, true), 20.0), GridItem::new(1, 4, 12, 1, false)], 12, 600.0, 400.0, GAP);
    assert_eq!(placed.boxes[0].height, 400.0 - 8.0 - 36.0);
    assert_eq!(placed.content, 400.0);
}

#[test]
fn a_part_that_does_not_grow_keeps_its_rows_and_the_room_around_them() {
    let placed = grid::place(&[around(GridItem::new(1, 1, 12, 2, false), 20.0), GridItem::new(1, 3, 12, 1, false)], 12, 600.0, 400.0, GAP);
    assert_eq!(placed.boxes[0].height, 2.0 * 36.0 + 8.0 + 20.0);
    // The part below starts under it.
    assert_eq!(placed.boxes[1].y, 2.0 * 36.0 + 8.0 + 20.0 + 8.0);
}

#[test]
fn rows_with_more_room_keep_it_and_the_others_share_the_rest() {
    // Two growing parts side by side in rows 1-2: the one with room around makes row 2 higher.
    let placed = grid::place(&[around(GridItem::new(1, 1, 6, 2, true), 100.0), GridItem::new(7, 1, 6, 2, true)], 12, 600.0, 120.0, GAP);
    assert_eq!(placed.boxes[0].height, 2.0 * 36.0 + 8.0 + 100.0);
    assert_eq!(placed.content, 2.0 * 36.0 + 8.0 + 100.0);
}

struct Holder {
    node: Node,
    around: f64,
}

impl Element for Holder {
    element_node!();

    fn around(&self) -> f64 {
        self.around
    }
}

#[test]
fn a_grid_panel_asks_each_part_for_the_room_around_it() {
    let mut panel = GridPanel::new(12);
    panel.add(Box::new(Holder { node: Node::new(), around: 30.0 }), GridItem::new(1, 1, 12, 2, true));
    panel.arrange(Rect::new(0.0, 0.0, 600.0, 50.0), &mut Cx::alone());
    assert_eq!(panel.node().children()[0].node().bounds.height, 2.0 * 36.0 + 8.0 + 30.0);
    assert_eq!(panel.content, 2.0 * 36.0 + 8.0 + 30.0);
}
