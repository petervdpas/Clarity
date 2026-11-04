//! The footer at the bottom of a window: its status text at the left, in
//! the language shown, and its parts on one row of its columns beside it.

use std::rc::Rc;

use crate::element::{Cx, Element, GridPanel, Node};
use crate::geometry::Rect;
use crate::grid::{row_height, GridItem};
use crate::input::Modifiers;
use crate::painter::{Painter, TextAlign};
use crate::parts;

use super::spec::FooterSpec;
use super::{plate, WindowFrame};

/// How high a footer is with parts: a row of the project's height, and room around it.
pub fn parts_height() -> f64 {
    row_height() + 8.0
}

/// How high a footer is with its status alone.
pub const STATUS_HEIGHT: f64 = 28.0;

pub struct FrameFooter {
    node: Node,
    frame: Rc<WindowFrame>,
    status_box: Rect,
}

impl FrameFooter {
    pub fn new(frame: Rc<WindowFrame>) -> FrameFooter {
        let spec = frame.shown().footer.as_ref().expect("a footer has its spec").value.clone();
        let mut grid = GridPanel::new(spec.columns.max(1));
        for part in &spec.parts {
            let made = parts::make(part.clone(), frame.host().clone());
            // One row: a part that grows keeps the row's height.
            let place = part.place(false);
            grid.add(made, GridItem { row: 1, height: 1, ..place });
        }
        let mut node = Node::new();
        node.add(Box::new(grid));
        FrameFooter { node, frame, status_box: Rect::default() }
    }

    pub fn footer(&self) -> &FooterSpec {
        &self.frame.shown().footer.as_ref().expect("a footer has its spec").value
    }

    /// Its parts' grid.
    pub fn grid(&self) -> &GridPanel {
        self.node.children()[0].as_any().downcast_ref::<GridPanel>().expect("a footer holds its grid")
    }

    /// The status as shown, in the language shown; none without one.
    pub fn status(&self) -> Option<String> {
        self.footer().status.as_deref().map(|s| self.frame.host().text(Some(s)))
    }

    pub fn wanted(&self) -> f64 {
        if self.footer().parts.is_empty() { STATUS_HEIGHT } else { parts_height() }
    }

    /// Where the status text is.
    pub fn status_box(&self) -> Rect {
        self.status_box
    }
}

impl Element for FrameFooter {
    crate::element_node!();

    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.node.bounds = area;
        let width = self.status().map(|s| cx.fonts().measure(&s, "text", cx.look().size("small"), false) + 16.0).unwrap_or(0.0);
        self.status_box = Rect::new(area.x + 4.0, area.y, width, area.height);
        let x = self.status_box.right() + if width > 0.0 { 6.0 } else { 4.0 };
        let row = row_height();
        self.node.children_mut()[0].arrange(Rect::new(x, area.y + (area.height - row) / 2.0, (area.right() - 8.0 - x).max(0.0), row), cx);
    }

    fn on_wheel(&mut self, _dx: f64, _dy: f64, _held: Modifiers, _cx: &mut Cx) -> bool {
        true
    }

    fn draw(&self, p: &Painter) {
        let look = p.look;
        let b = self.node.bounds;
        plate(p, b, Rect::new(b.x, b.y, b.width, look.border()));
        if let Some(status) = self.status() {
            let at = self.status_box;
            p.text(&status, Rect::new(at.x + 4.0, at.y, (at.width - 8.0).max(0.0), at.height), look.color("muted"), look.size("small"), "text", false, TextAlign::Left);
        }
        if !self.footer().parts.is_empty() {
            crate::element::draw_children(&self.node, p);
        }
    }
}
