//! A popover (DESIGN 5.16): parts on a grid of their own, shown over the
//! window while its part's state is visible, next to the part named in its
//! `for` (below it, or above when there is no room under it), else at its
//! own place. In the window's grid it takes no room and no pointer. A press
//! outside it or Escape closes it, and its part's state hears that it is no
//! longer visible, so handlers open and close it with `visible`. A popover
//! without a name has no state and never opens. The screen lays the window
//! out on every draw, and the popover follows its state then: it opens its
//! overlay, moves it, or closes it.

use std::cell::Cell;
use std::rc::Rc;

use crate::element::{draw_children, Cx, Element, ElementId, Node};
use crate::geometry::Rect;
use crate::grid::{GAP, ROW_HEIGHT};
use crate::markup::PartSpec;
use crate::overlay::Overlay;
use crate::painter::Painter;
use crate::parts::{Part, PartBase, PartHost};
use crate::theme::ThemeLook;

use super::container::{self, LookNow};

pub struct PopoverPart {
    base: PartBase,
    /// The id of the overlay it lies in while open; shared with the overlay's `closed`, which empties it.
    opened: Rc<Cell<Option<ElementId>>>,
    /// Where it lies while open, shared with what lies over the window.
    place: Rc<Cell<Rect>>,
}

pub fn make(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    Box::new(PopoverPart { base: PartBase::new(spec, host), opened: Rc::new(Cell::new(None)), place: Rc::new(Cell::new(Rect::default())) })
}

impl PopoverPart {
    /// How wide a column of a popover's grid is.
    pub const COLUMN: f64 = 80.0;

    /// The id of what lies over the window while it is open; none while closed.
    pub fn opened(&self) -> Option<ElementId> {
        self.opened.get()
    }

    /// Whether it is open: its part's state is visible.
    pub fn is_open(&self) -> bool {
        self.base.state().and_then(|s| s.visible).unwrap_or(false)
    }

    /// How big it is: its columns and rows, with its padding and border around them.
    pub fn size(&self, look: &ThemeLook) -> (f64, f64) {
        let columns = container::columns_of(&self.base.spec) as f64;
        let rows = self.base.spec.number("height").unwrap_or(1.0).max(1.0).floor();
        let around = 2.0 * (2.0 * look.padding() + look.border());
        (Self::COLUMN * columns + GAP * (columns - 1.0) + around, ROW_HEIGHT * rows + GAP * (rows - 1.0) + around)
    }

    /// Where it lies on the screen `whole`: under the part it is for (`near`),
    /// above it when there is no room below, else at its own place; kept off
    /// the screen's left and right edges.
    pub fn place(near: Option<Rect>, own: Rect, size: (f64, f64), whole: Rect) -> Rect {
        let (width, height) = size;
        let (x, mut y) = match near {
            Some(at) => (at.x, at.bottom() + 4.0),
            None => (own.x, own.y),
        };
        if let Some(at) = near {
            if y + height > whole.bottom() && at.y - height - 4.0 > whole.y {
                y = at.y - height - 4.0;
            }
        }
        let x = x.min(whole.right() - width - 4.0).max(whole.x + 4.0);
        Rect::new(x, y, width, height)
    }

    /// Opens or closes it as its part's state says, at its place.
    fn follow(&mut self, cx: &mut Cx) {
        if !self.is_open() {
            if let Some(open) = self.opened.take() {
                cx.close(open);
            }
            return;
        }
        let Some(name) = self.base.name().map(str::to_string) else { return };
        // The part its `for` names is not reachable from here yet (g8-shared.md popover-anchor): it lies at its own place.
        let place = Self::place(None, self.base.node.bounds, self.size(cx.look()), cx.whole());
        self.place.set(place);
        if self.opened.get().is_some() {
            return;
        }
        let grid = container::grid_of(&self.base.spec.children, container::columns_of(&self.base.spec), GAP, &self.base.host);
        let mut frame = PopoverFrame { node: Node::new(), place: self.place.clone() };
        frame.node.add(Box::new(grid));
        let mut overlay = Overlay::new(Box::new(frame), place);
        let (opened, host) = (self.opened.clone(), self.base.host.clone());
        overlay.closed = Some(Box::new(move || {
            // Closed by a press outside or Escape: its state hears it is no longer visible.
            if opened.take().is_some() {
                host.change_state(&name, &mut |s| s.visible = Some(false));
            }
        }));
        self.opened.set(Some(cx.open(overlay)));
    }
}

impl Element for PopoverPart {
    fn node(&self) -> &Node {
        &self.base.node
    }

    fn node_mut(&mut self) -> &mut Node {
        &mut self.base.node
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn as_part(&self) -> Option<&dyn Part> {
        Some(self)
    }

    fn as_part_mut(&mut self) -> Option<&mut dyn Part> {
        Some(self)
    }

    /// In the window's grid it shows nothing: what it holds lies over the window.
    fn is_shown(&self) -> bool {
        false
    }

    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.base.node.bounds = area;
        self.follow(cx);
    }

    fn draw(&self, _p: &Painter) {}

    /// Leaving the screen, what it opened goes with it; its state stays as it is.
    fn on_detached(&mut self, cx: &mut Cx) {
        if let Some(open) = self.opened.take() {
            cx.close(open);
        }
    }
}

impl Part for PopoverPart {
    fn base(&self) -> &PartBase {
        &self.base
    }

    /// It takes no room in the window's grid, so it grows by none.
    fn grows(&self) -> bool {
        false
    }
}

/// What lies over the window: the panel's color with a strong border, the popover's grid inside, at the place the popover keeps.
struct PopoverFrame {
    node: Node,
    place: Rc<Cell<Rect>>,
}

impl Element for PopoverFrame {
    crate::element_node!();

    fn arrange(&mut self, _area: Rect, cx: &mut Cx) {
        let place = self.place.get();
        self.node.bounds = place;
        let look = LookNow::of(cx.look());
        let inner = place.inset(2.0 * look.padding + look.border);
        self.node.children_mut()[0].arrange(inner, cx);
    }

    fn draw(&self, p: &Painter) {
        let radius = p.look.radius("panel");
        p.fill(self.node.bounds, p.look.color("panel"), radius);
        p.stroke(self.node.bounds, p.look.color("strong_border"), p.look.border(), radius);
        let _framing = container::framing();
        draw_children(&self.node, p);
    }
}
