//! A splitter: panes side by side (or one above the other when vertical),
//! each a grid of its parts, with a divider between them the person drags.
//! `pane span 4` gives the first of two side by side panes 4 of the part's
//! columns and the second the rest; one above the other, `rows` on the first
//! starts it at that share of the part's rows. Otherwise each pane has all
//! the part's columns and they share the room alike. A pane whose parts go further down than
//! its room scrolls, so none of them is out of reach in a low window. The
//! dividers and the part share how much of the room each pane has, so a
//! drag changes it without reaching the part; the screen places the panes
//! anew as it draws.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::element::{draw_children, Cx, Element, GridPanel, Node};
use crate::geometry::Rect;
use crate::input::{Cursor, PointerInput};
use crate::markup::{PageSpec, PartSpec};
use crate::painter::Painter;
use crate::part_element;
use crate::parts::{Part, PartBase, PartHost};

use super::container::{self, LookNow};
use super::scroll_view::ScrollView;

/// What the splitter and its dividers share: how much of the room each pane has, and where the splitter is.
struct Split {
    host: Rc<dyn PartHost>,
    name: Option<String>,
    vertical: bool,
    dividers: usize,
    shares: RefCell<Vec<f64>>,
    bounds: Cell<Rect>,
}

impl Split {
    fn enabled(&self) -> bool {
        self.host.state(self.name.as_deref()).and_then(|s| s.enabled).unwrap_or(true)
    }

    /// The room the panes share: the part's length less its dividers.
    fn room(&self) -> f64 {
        let b = self.bounds.get();
        (if self.vertical { b.height } else { b.width }) - SplitterPart::DIVIDER_THICKNESS * self.dividers as f64
    }

    /// The divider before pane `index` dragged to `along`, in the screen's units: the two panes beside it share their room anew, each keeping a tenth of the part at least.
    fn drag(&self, index: usize, along: f64) {
        let room = self.room();
        if room <= 0.0 {
            return;
        }
        let b = self.bounds.get();
        let start = if self.vertical { b.y } else { b.x };
        let mut shares = self.shares.borrow_mut();
        let before = shares[..index - 1].iter().sum::<f64>() * room + (index - 1) as f64 * SplitterPart::DIVIDER_THICKNESS;
        let pair = shares[index - 1] + shares[index];
        let least = (pair / 2.0).min(0.1);
        let first = ((along - start - before) / room).clamp(least, pair - least);
        shares[index - 1] = first;
        shares[index] = pair - first;
    }
}

pub struct SplitterPart {
    base: PartBase,
    split: Rc<Split>,
    look: Cell<LookNow>,
}

pub fn make(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    let vertical = spec.text("orientation") == Some("vertical");
    let columns = SplitterPart::pane_columns(&spec.pages, container::columns_of(&spec), vertical);
    let total: i64 = columns.iter().sum();
    let mut shares: Vec<f64> = columns.iter().map(|c| if total == 0 { 0.0 } else { *c as f64 / total as f64 }).collect();
    // One above the other, a first pane with rows of its own starts at that share of the splitter's rows.
    let rows = spec.number("height").map(|h| h as i64).unwrap_or(0);
    if let (true, 2, Some(first)) = (vertical, spec.pages.len(), spec.pages.first().and_then(|p| p.columns).filter(|f| *f >= 1 && *f < rows)) {
        shares = vec![first as f64 / rows as f64, (rows - first) as f64 / rows as f64];
    }
    let split = Rc::new(Split {
        host: host.clone(),
        name: spec.name().map(str::to_string),
        vertical,
        dividers: spec.pages.len().saturating_sub(1),
        shares: RefCell::new(shares),
        bounds: Cell::new(Rect::default()),
    });
    let mut base = PartBase::new(spec, host.clone());
    for (i, page) in base.spec.pages.clone().iter().enumerate() {
        if i > 0 {
            base.node.add(Box::new(Divider { node: Node::new(), split: split.clone(), index: i, grab: 0.0 }));
        }
        let grid = container::grid_of(&page.children, columns[i], crate::grid::GAP, &host).owned(page.item.as_deref());
        base.node.add(Box::new(ScrollView::new(Box::new(grid)).keeping_bar_room()));
    }
    Box::new(SplitterPart { base, split, look: Cell::new(LookNow::default()) })
}

impl SplitterPart {
    /// How thick a divider is.
    pub const DIVIDER_THICKNESS: f64 = 4.0;

    /// The columns of each pane: `pane span n` on the first of two side by side, else all of the part's for each.
    pub fn pane_columns(pages: &[PageSpec], width: i64, vertical: bool) -> Vec<i64> {
        if !vertical && pages.len() == 2 {
            if let Some(first) = pages[0].columns.filter(|f| *f >= 1 && *f < width) {
                return vec![first, width - first];
            }
        }
        pages.iter().map(|_| width).collect()
    }

    pub fn vertical(&self) -> bool {
        self.split.vertical
    }

    /// How much of the room each pane has, from 0 to 1, as designed or as the person dragged it.
    pub fn shares(&self) -> Vec<f64> {
        self.split.shares.borrow().clone()
    }

    /// The place of each pane, which scrolls its grid when the grid is higher.
    pub fn views(&self) -> Vec<&ScrollView> {
        self.base.node.children().iter().filter_map(|c| c.as_any().downcast_ref::<ScrollView>()).collect()
    }

    /// The grid of each pane.
    pub fn panes(&self) -> Vec<&GridPanel> {
        self.views().into_iter().filter_map(|v| container::grid_in(v)).collect()
    }

    /// The lines between the panes, which the person drags.
    pub fn dividers(&self) -> Vec<&dyn Element> {
        self.base.node.children().iter().filter(|c| c.as_any().is::<Divider>()).map(|c| c.as_ref()).collect()
    }
}

impl Element for SplitterPart {
    part_element!();

    /// The room it needs beyond its rows: each pane's padding and grid, side by side the highest, one above the other all of them with the dividers.
    fn around(&self) -> f64 {
        let pad = 4.0 * self.look.get().padding;
        let needs: Vec<f64> = self.panes().iter().map(|g| pad + container::least_of(g)).collect();
        if needs.is_empty() {
            return 0.0;
        }
        let needed = if self.split.vertical { needs.iter().sum::<f64>() + self.split.dividers as f64 * Self::DIVIDER_THICKNESS } else { needs.iter().copied().fold(0.0, f64::max) };
        container::around_for(&self.base.spec, 0.0, needed)
    }

    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.base.node.bounds = area;
        self.split.bounds.set(area);
        self.look.set(LookNow::of(cx.look()));
        let pad = 2.0 * cx.look().padding();
        let vertical = self.split.vertical;
        let room = self.split.room();
        let shares = self.shares();
        let mut at = if vertical { area.y } else { area.x };
        let mut pane = 0;
        for child in self.base.node.children_mut().iter_mut() {
            if child.as_any().is::<Divider>() {
                let place = if vertical { Rect::new(area.x, at, area.width, Self::DIVIDER_THICKNESS) } else { Rect::new(at, area.y, Self::DIVIDER_THICKNESS, area.height) };
                crate::element::arrange_safely(child.as_mut(), place, cx);
                at += Self::DIVIDER_THICKNESS;
                continue;
            }
            let length = (room * shares.get(pane).copied().unwrap_or(0.0)).max(0.0);
            let place = if vertical { Rect::new(area.x, at, area.width, length) } else { Rect::new(at, area.y, length, area.height) };
            crate::element::arrange_safely(child.as_mut(), place.inset(pad), cx);
            at += length;
            pane += 1;
        }
    }

    fn draw(&self, p: &Painter) {
        let bounds = self.base.node.bounds;
        {
            let _clip = p.clip(bounds);
            let _framing = container::framing();
            draw_children(&self.base.node, p);
        }
        p.stroke(bounds, p.look.color("strong_border"), p.look.border(), p.look.radius("panel"));
    }
}

impl Part for SplitterPart {
    fn base(&self) -> &PartBase {
        &self.base
    }

    /// A splitter grows when a part in one of its panes does.
    fn grows(&self) -> bool {
        self.panes().iter().any(|g| container::grid_grows(g))
    }
}

/// The line between two panes, which the person drags.
struct Divider {
    node: Node,
    split: Rc<Split>,
    /// The pane after it.
    index: usize,
    /// Where on it the pointer took it.
    grab: f64,
}

impl Element for Divider {
    crate::element_node!();

    /// Over a divider the pointer shows the way it drags.
    fn cursor_at(&self, _x: f64, _y: f64) -> Option<Cursor> {
        Some(if self.split.vertical { Cursor::ResizeUpDown } else { Cursor::ResizeLeftRight })
    }

    fn on_pointer_pressed(&mut self, at: PointerInput, _cx: &mut Cx) {
        self.grab = if self.split.vertical { at.y } else { at.x };
    }

    fn on_pointer_moved(&mut self, at: PointerInput, cx: &mut Cx) {
        if self.node.pressed && self.split.enabled() {
            let along = if self.split.vertical { at.screen_y } else { at.screen_x };
            self.split.drag(self.index, along - self.grab);
            cx.invalidate();
        }
    }

    fn draw(&self, p: &Painter) {
        let hot = self.node.pressed || self.node.hovered;
        p.fill(self.node.bounds, p.look.color(if hot { "accent" } else { "strong_border" }), 0.0);
    }
}
