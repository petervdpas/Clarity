//! A place that scrolls: content taller or wider than the place moves with
//! the wheel and with a thin bar the person drags or presses, and shows only
//! inside the place. Parts use it for whatever needs to scroll: a scroll
//! box's grid, a tab's page, a splitter's pane, a long text. The wheel's
//! turn is in the screen's units, down and right positive; at an end it
//! takes no more, so what is around it may scroll on.

use std::cell::Cell;
use std::rc::Rc;

use crate::element::{draw_children, Cx, Element, GridPanel, Node};
use crate::geometry::Rect;
use crate::input::{Modifiers, PointerInput};
use crate::painter::Painter;

/// How thick a bar is, from the edge of the place.
pub const BAR_THICKNESS: f64 = 8.0;

const LEAST_THUMB: f64 = 24.0;

/// How wide and high content is for a place of a width and height.
pub type Measure = Box<dyn Fn(&dyn Element, f64, f64, &mut Cx) -> (f64, f64)>;

/// Where the view is scrolled to and how big its content is, shared with its bars.
#[derive(Debug, Default)]
struct Scrolled {
    x: Cell<f64>,
    y: Cell<f64>,
    content_width: Cell<f64>,
    content_height: Cell<f64>,
    place: Cell<Rect>,
}

impl Scrolled {
    fn max_x(&self) -> f64 {
        (self.content_width.get() - self.place.get().width).max(0.0)
    }

    fn max_y(&self) -> f64 {
        (self.content_height.get() - self.place.get().height).max(0.0)
    }

    /// Scrolls to `x`, `y` within the ends: true when it moved.
    fn to(&self, x: f64, y: f64) -> bool {
        let x = x.clamp(0.0, self.max_x());
        let y = y.clamp(0.0, self.max_y());
        if x == self.x.get() && y == self.y.get() {
            return false;
        }
        self.x.set(x);
        self.y.set(y);
        true
    }
}

pub struct ScrollView {
    node: Node,
    scrolled: Rc<Scrolled>,
    /// How wide and high the content is for a place; without one, a grid is
    /// as high as its rows and anything else is as big as the place.
    pub measure: Option<Measure>,
    /// Whether the content is made narrower by the bar while the bar down shows, so the bar covers none of it.
    pub keeps_bar_room: bool,
}

impl ScrollView {
    pub fn new(content: Box<dyn Element>) -> ScrollView {
        let scrolled = Rc::new(Scrolled::default());
        let mut node = Node::new();
        node.add(content);
        node.add(Box::new(ScrollBar { node: Node::new(), scrolled: scrolled.clone(), vertical: true, grab: None }));
        node.add(Box::new(ScrollBar { node: Node::new(), scrolled: scrolled.clone(), vertical: false, grab: None }));
        ScrollView { node, scrolled, measure: None, keeps_bar_room: false }
    }

    /// A view that keeps room for its bar down (`keeps_bar_room`).
    pub fn keeping_bar_room(mut self) -> ScrollView {
        self.keeps_bar_room = true;
        self
    }

    pub fn content(&self) -> &dyn Element {
        self.node.children()[0].as_ref()
    }

    pub fn content_mut(&mut self) -> &mut dyn Element {
        self.node.children_mut()[0].as_mut()
    }

    /// Its bars: down, then across.
    pub fn bars(&self) -> [&dyn Element; 2] {
        [self.node.children()[1].as_ref(), self.node.children()[2].as_ref()]
    }

    /// How wide and high the content was when it was last placed.
    pub fn content_width(&self) -> f64 {
        self.scrolled.content_width.get()
    }

    pub fn content_height(&self) -> f64 {
        self.scrolled.content_height.get()
    }

    pub fn max_scroll_x(&self) -> f64 {
        self.scrolled.max_x()
    }

    pub fn max_scroll_y(&self) -> f64 {
        self.scrolled.max_y()
    }

    /// How far the content is scrolled across, from 0 to `max_scroll_x`.
    pub fn scroll_x(&self) -> f64 {
        self.scrolled.x.get()
    }

    /// How far the content is scrolled down, from 0 to `max_scroll_y`.
    pub fn scroll_y(&self) -> f64 {
        self.scrolled.y.get()
    }

    /// Scrolls to `x`, `y`, within its ends, and places the content there.
    pub fn scroll_to(&mut self, x: f64, y: f64, cx: &mut Cx) {
        if self.scrolled.to(x, y) {
            self.place(cx);
            cx.invalidate();
        }
    }

    /// Scrolls as little as it takes to show `area`, a place in the content from its top left.
    pub fn scroll_into(&mut self, area: Rect, cx: &mut Cx) {
        let place = self.node.bounds;
        let mut y = self.scroll_y();
        if area.y < y {
            y = area.y;
        } else if area.bottom() > y + place.height {
            y = area.bottom() - place.height;
        }
        let mut x = self.scroll_x();
        if area.x < x {
            x = area.x;
        } else if area.right() > x + place.width {
            x = area.right() - place.width;
        }
        self.scroll_to(x, y, cx);
    }

    fn measured(&mut self, width: f64, height: f64, cx: &mut Cx) -> (f64, f64) {
        if let Some(measure) = &self.measure {
            return measure(self.node.children()[0].as_ref(), width, height, cx);
        }
        match self.node.children()[0].as_any().downcast_ref::<GridPanel>() {
            Some(grid) => (width, height.max(grid.content_for(width, height))),
            None => (width, height),
        }
    }

    /// Places the content where it is scrolled to, and the bars at the edges.
    fn place(&mut self, cx: &mut Cx) {
        let area = self.node.bounds;
        let s = self.scrolled.clone();
        let down_shows = s.max_y() > 0.0;
        let across_shows = s.max_x() > 0.0;
        let content = Rect::new(area.x - s.x.get(), area.y - s.y.get(), s.content_width.get(), s.content_height.get());
        let children = self.node.children_mut();
        children[0].arrange(content, cx);
        let down = Rect::new(area.right() - BAR_THICKNESS, area.y, BAR_THICKNESS, area.height - if across_shows { BAR_THICKNESS } else { 0.0 });
        children[1].arrange(down, cx);
        let across = Rect::new(area.x, area.bottom() - BAR_THICKNESS, area.width - if down_shows { BAR_THICKNESS } else { 0.0 }, BAR_THICKNESS);
        children[2].arrange(across, cx);
    }
}

impl Element for ScrollView {
    crate::element_node!();

    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.node.bounds = area;
        self.scrolled.place.set(area);
        let (mut width, mut height) = self.measured(area.width, area.height, cx);
        let mut room = area.width;
        if self.keeps_bar_room && height > area.height {
            room = (area.width - BAR_THICKNESS).max(0.0);
            (width, height) = self.measured(room, area.height, cx);
        }
        let s = &self.scrolled;
        s.content_width.set(width.max(room));
        s.content_height.set(height.max(area.height));
        s.x.set(s.x.get().clamp(0.0, s.max_x()));
        s.y.set(s.y.get().clamp(0.0, s.max_y()));
        self.place(cx);
    }

    fn on_wheel(&mut self, dx: f64, dy: f64, _held: Modifiers, cx: &mut Cx) -> bool {
        let (x, y) = (self.scroll_x(), self.scroll_y());
        self.scroll_to(x + dx, y + dy, cx);
        x != self.scroll_x() || y != self.scroll_y()
    }

    fn draw(&self, p: &Painter) {
        let _clip = p.clip(self.node.bounds);
        draw_children(&self.node, p);
    }
}

/// A bar at an edge: its thumb shows how much of the content is in view and where, and moves it.
struct ScrollBar {
    node: Node,
    scrolled: Rc<Scrolled>,
    vertical: bool,
    /// Where on the thumb the pointer took it; none for a press beside the thumb.
    grab: Option<f64>,
}

impl ScrollBar {
    fn track(&self) -> f64 {
        if self.vertical { self.node.bounds.height } else { self.node.bounds.width }
    }

    fn content(&self) -> f64 {
        if self.vertical { self.scrolled.content_height.get() } else { self.scrolled.content_width.get() }
    }

    fn max(&self) -> f64 {
        if self.vertical { self.scrolled.max_y() } else { self.scrolled.max_x() }
    }

    fn scroll(&self) -> f64 {
        if self.vertical { self.scrolled.y.get() } else { self.scrolled.x.get() }
    }

    fn thumb_length(&self) -> f64 {
        let track = self.track();
        if self.content() <= 0.0 { track } else { track.min(LEAST_THUMB.max(track * track / self.content())) }
    }

    fn thumb_at(&self) -> f64 {
        let max = self.max();
        if max <= 0.0 { 0.0 } else { (self.track() - self.thumb_length()) * self.scroll() / max }
    }

    fn scroll_to(&self, to: f64, cx: &mut Cx) {
        let s = &self.scrolled;
        let moved = if self.vertical { s.to(s.x.get(), to) } else { s.to(to, s.y.get()) };
        if moved {
            // The view places its content anew as the screen lays it out before drawing.
            cx.invalidate();
        }
    }
}

impl Element for ScrollBar {
    crate::element_node!();

    fn is_shown(&self) -> bool {
        self.max() > 0.0
    }

    fn on_pointer_pressed(&mut self, at: PointerInput, cx: &mut Cx) {
        let along = if self.vertical { at.y } else { at.x };
        let thumb = self.thumb_at();
        if along >= thumb && along < thumb + self.thumb_length() {
            self.grab = Some(along - thumb);
            return;
        }
        // A press beside the thumb moves a place's length that way.
        let place = self.scrolled.place.get();
        let page = if self.vertical { place.height } else { place.width };
        let to = self.scroll() + if along < thumb { -page } else { page };
        self.grab = None;
        self.scroll_to(to, cx);
    }

    fn on_pointer_moved(&mut self, at: PointerInput, cx: &mut Cx) {
        let Some(grab) = self.grab.filter(|_| self.node.pressed) else { return };
        let along = if self.vertical { at.y } else { at.x };
        let travel = self.track() - self.thumb_length();
        let to = if travel <= 0.0 { 0.0 } else { (along - grab) / travel * self.max() };
        self.scroll_to(to, cx);
    }

    fn on_pointer_released(&mut self, _at: PointerInput, _cx: &mut Cx) {
        self.grab = None;
    }

    fn draw(&self, p: &Painter) {
        if !self.is_shown() {
            return;
        }
        let b = self.node.bounds;
        let (at, length) = (self.thumb_at(), self.thumb_length());
        let thumb = if self.vertical { Rect::new(b.x + 2.0, b.y + at, b.width - 4.0, length) } else { Rect::new(b.x + at, b.y + 2.0, length, b.height - 4.0) };
        let color = if self.node.pressed || self.node.hovered { "muted" } else { "strong_border" };
        let radius = if self.vertical { thumb.width } else { thumb.height } / 2.0;
        p.fill(thumb, p.look.color(color), radius);
    }
}
