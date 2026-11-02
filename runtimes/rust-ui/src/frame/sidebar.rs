//! A sidebar docked to a window's edge (conformance/sidebars.json): its
//! parts on a grid of its columns. The person drags a resizable one's inner
//! edge between its min and max, folds a collapsible one to a narrow strip
//! with its button and back, and lets a maximizable one fill the window and
//! give it back. Whether it is folded and whether it fills the window are
//! its part state (`collapsed`, `maximized`), so code reads and sets them;
//! an unnamed sidebar keeps them itself.

use std::rc::Rc;

use skia_safe::Color;

use crate::element::{Cx, Element, GridPanel, Node};
use crate::geometry::Rect;
use crate::input::{Modifiers, PointerInput};
use crate::painter::{Painter, TextAlign};
use crate::parts;

use super::spec::SidebarSpec;
use super::{over, plate, rules, Showing, WindowFrame};

/// The space inside it, and how high the row of its buttons is.
pub const PADDING: f64 = 8.0;
pub const BUTTONS_HEIGHT: f64 = 24.0;

/// How wide the edge the person drags is.
pub const EDGE_WIDTH: f64 = 6.0;

#[derive(Clone, Copy, PartialEq)]
enum Pressed {
    Fold,
    Fill,
}

pub struct FrameSidebar {
    node: Node,
    frame: Rc<WindowFrame>,
    left: bool,
    dragged: Option<f64>,
    dragging: Option<(f64, f64)>,
    pressed_on: Option<Pressed>,
    pointer: Option<(f64, f64)>,
}

impl FrameSidebar {
    /// The sidebar on the left (`left`) or right of `frame`, its parts made by their kinds on its grid.
    pub fn new(frame: Rc<WindowFrame>, left: bool) -> FrameSidebar {
        let spec = frame.sidebar(left).expect("a sidebar has its spec").clone();
        let mut grid = GridPanel::new(spec.columns.max(1));
        for part in &spec.parts {
            let made = parts::make(part.clone(), frame.host().clone());
            let place = parts::place_of(made.as_ref());
            grid.add(made, place);
        }
        let shows = frame.clone();
        let holder = Showing::new(Box::new(grid), move || !shows.collapsed(left));
        let mut node = Node::new();
        node.add(Box::new(holder));
        FrameSidebar { node, frame, left, dragged: None, dragging: None, pressed_on: None, pointer: None }
    }

    pub fn sidebar(&self) -> &SidebarSpec {
        self.frame.sidebar(self.left).expect("a sidebar has its spec")
    }

    pub fn is_left(&self) -> bool {
        self.left
    }

    /// Its parts' grid.
    pub fn grid(&self) -> &GridPanel {
        let holder = self.node.children()[0].as_any().downcast_ref::<Showing>().expect("a sidebar holds its parts");
        holder.child().as_any().downcast_ref::<GridPanel>().expect("a sidebar's parts are on a grid")
    }

    pub fn collapsed(&self) -> bool {
        self.frame.collapsed(self.left)
    }

    pub fn maximized(&self) -> bool {
        self.frame.maximized(self.left)
    }

    /// Its width while it is neither folded nor filling the window: as designed, or as the person dragged it.
    pub fn width(&self) -> f64 {
        self.dragged.unwrap_or(self.sidebar().width)
    }

    pub fn wanted(&self) -> f64 {
        if self.collapsed() { rules::STRIP_WIDTH } else { self.width() }
    }

    fn has_buttons(&self) -> bool {
        self.sidebar().collapsible || self.sidebar().maximizable
    }

    /// The edge the person drags, on the side toward the window's parts; none when it cannot be dragged now.
    pub fn edge(&self) -> Option<Rect> {
        let b = self.node.bounds;
        (self.sidebar().resizable && !self.collapsed() && !self.maximized()).then(|| Rect::new(if self.left { b.right() - EDGE_WIDTH } else { b.x }, b.y, EDGE_WIDTH, b.height))
    }

    /// The fold button, at the end toward the window's parts; none for a sidebar that does not fold.
    pub fn fold_button(&self) -> Option<Rect> {
        self.sidebar().collapsible.then(|| self.button_at(0))
    }

    /// The button that lets it fill the window; none while it is folded or cannot.
    pub fn fill_button(&self) -> Option<Rect> {
        (self.sidebar().maximizable && !self.collapsed()).then(|| self.button_at(if self.sidebar().collapsible { 1 } else { 0 }))
    }

    fn button_at(&self, place: usize) -> Rect {
        let inner = self.node.bounds.inset(if self.collapsed() { 2.0 } else { PADDING });
        let step = BUTTONS_HEIGHT + 2.0;
        let x = if self.left { inner.right() - (place as f64 + 1.0) * step + 2.0 } else { inner.x + place as f64 * step };
        Rect::new(if self.collapsed() { inner.x } else { x }, inner.y, BUTTONS_HEIGHT, BUTTONS_HEIGHT)
    }

    /// Folds it to its strip or shows it again; folded, it no longer fills the window.
    pub fn fold(&mut self, folded: bool, cx: &mut Cx) {
        self.frame.fold(self.left, folded);
        cx.invalidate();
    }

    /// Lets it fill the window, or gives the window back.
    pub fn fill(&mut self, filling: bool, cx: &mut Cx) {
        self.frame.fill(self.left, filling);
        cx.invalidate();
    }

    /// The person dragged its edge to make it `width` wide: within its limits.
    pub fn drag_to(&mut self, width: f64, cx: &mut Cx) {
        self.dragged = Some(rules::sidebar_width(width, self.sidebar().min, self.sidebar().max));
        cx.invalidate();
    }

    /// A mark on a button, drawn as a single character in the middle of it.
    fn button(p: &Painter, area: Rect, mark: &str, hover: bool, text: Color) {
        let look = p.look;
        let values = look.part("button", if hover { Some("hover") } else { None });
        let radius = values.radius.map(|r| r as f64).unwrap_or_else(|| look.radius("button"));
        p.fill(area, values.background.unwrap_or_else(|| look.color("panel")), radius);
        p.stroke(area, values.border.unwrap_or_else(|| look.color("border")), look.border(), radius);
        p.text(mark, area, values.text.unwrap_or(text), look.size("text"), "text", false, TextAlign::Center);
    }
}

/// The fill button's mark, drawn as lines so every font shows it: a double
/// arrow from the bottom left to the top right to fill the window, from the
/// top left to the bottom right while it fills.
pub fn fill_mark(p: &Painter, area: Rect, filling: bool, color: Color) {
    let side = area.width.min(area.height) * 0.28;
    let (cx, cy) = (area.x + area.width / 2.0, area.y + area.height / 2.0);
    let (dx, dy) = if filling { (side, side) } else { (side, -side) };
    p.line((cx - dx, cy - dy), (cx + dx, cy + dy), color, 1.6);
    let head = side * 0.55;
    for s in [1.0, -1.0] {
        // Each end's arrowhead: two short lines back along the sides of the box.
        let (ex, ey) = (cx + s * dx, cy + s * dy);
        p.line((ex, ey), (ex - s * dx.signum() * head, ey), color, 1.6);
        p.line((ex, ey), (ex, ey - s * dy.signum() * head), color, 1.6);
    }
}

impl Element for FrameSidebar {
    crate::element_node!();

    /// It shows while its state is not hidden, unless the other sidebar fills the window.
    fn is_shown(&self) -> bool {
        self.frame.sidebar_shows(self.left) && self.frame.filling().is_none_or(|f| f == self.left)
    }

    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.node.bounds = area;
        let inner = area.inset(PADDING);
        let top = if self.has_buttons() { BUTTONS_HEIGHT + 6.0 } else { 0.0 };
        self.node.children_mut()[0].arrange(Rect::new(inner.x, inner.y + top, inner.width, (inner.height - top).max(0.0)), cx);
    }

    fn on_pointer_pressed(&mut self, at: PointerInput, _cx: &mut Cx) {
        let (x, y) = (at.screen_x, at.screen_y);
        self.pressed_on = None;
        if self.edge().is_some_and(|e| e.contains(x, y)) {
            self.dragging = Some((x, self.width()));
        } else if self.fold_button().is_some_and(|b| b.contains(x, y)) {
            self.pressed_on = Some(Pressed::Fold);
        } else if self.fill_button().is_some_and(|b| b.contains(x, y)) {
            self.pressed_on = Some(Pressed::Fill);
        }
    }

    fn on_pointer_moved(&mut self, at: PointerInput, cx: &mut Cx) {
        self.pointer = Some((at.screen_x, at.screen_y));
        cx.invalidate();
        if let (true, Some((from, width))) = (self.node.pressed, self.dragging) {
            let moved = if self.left { at.screen_x - from } else { from - at.screen_x };
            self.drag_to(width + moved, cx);
        }
    }

    fn on_pointer_released(&mut self, _at: PointerInput, _cx: &mut Cx) {
        self.dragging = None;
    }

    fn on_click(&mut self, cx: &mut Cx) {
        match self.pressed_on.take() {
            Some(Pressed::Fold) => {
                let folded = self.collapsed();
                self.fold(!folded, cx);
            }
            Some(Pressed::Fill) => {
                let filling = self.maximized();
                self.fill(!filling, cx);
            }
            None => {}
        }
    }

    fn on_wheel(&mut self, _dx: f64, _dy: f64, _held: Modifiers, _cx: &mut Cx) -> bool {
        true
    }

    fn draw(&self, p: &Painter) {
        let look = p.look;
        let b = self.node.bounds;
        let line = if self.left { Rect::new(b.right() - look.border(), b.y, look.border(), b.height) } else { Rect::new(b.x, b.y, look.border(), b.height) };
        plate(p, b, line);
        if let Some(edge) = self.edge().filter(|e| over(&self.node, self.pointer, *e) || self.dragging.is_some()) {
            p.fill(edge, look.color("accent_soft"), 0.0);
        }
        let text = look.color("frame_text");
        // The fold button points the way the sidebar goes; the fill button shows the arrows out, or in while it fills.
        if let Some(fold) = self.fold_button() {
            let mark = if self.left != self.collapsed() { "\u{2039}" } else { "\u{203A}" };
            Self::button(p, fold, mark, over(&self.node, self.pointer, fold), text);
        }
        if let Some(fill) = self.fill_button() {
            let hover = over(&self.node, self.pointer, fill);
            Self::button(p, fill, "", hover, text);
            let mark = look.part("button", if hover { Some("hover") } else { None }).text.unwrap_or(text);
            fill_mark(p, fill, self.maximized(), mark);
        }
        if !self.collapsed() {
            let _clip = p.clip(b);
            crate::element::draw_children(&self.node, p);
        }
    }
}
