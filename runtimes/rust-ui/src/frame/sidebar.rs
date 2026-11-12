//! A sidebar docked to a window's edge (conformance/sidebars.json): its
//! parts on a grid of its columns. The person drags a resizable one's inner
//! edge between its min and max, folds a collapsible one to a narrow strip
//! with its button and back, and lets a maximizable one fill the window and
//! give it back. Whether it is folded and whether it fills the window are
//! its part state (`collapsed`, `maximized`), so code reads and sets them;
//! an unnamed sidebar keeps them itself. Folding leaves whether it fills the
//! window as it was, as Avalonia's did: shown again, it fills it again. Its
//! buttons take the focus and say what they do; its edge shows the resize
//! pointer.

use std::rc::Rc;

use skia_safe::Color;

use crate::element::{draw_children, Cx, Element, GridPanel, Node};
use crate::geometry::Rect;
use crate::input::{Cursor, Key, KeyInput, Modifiers, PointerInput};
use crate::locale;
use crate::painter::{Painter, TextAlign};
use crate::parts;

use super::spec::SidebarSpec;
use super::{over, plate, rules, Showing, WindowFrame};

/// The space inside it, and how high the row of its buttons is.
pub const PADDING: f64 = 8.0;
pub const BUTTONS_HEIGHT: f64 = 24.0;

/// How wide the edge the person drags is.
pub const EDGE_WIDTH: f64 = 6.0;

pub struct FrameSidebar {
    node: Node,
    frame: Rc<WindowFrame>,
    left: bool,
    dragged: Option<f64>,
    dragging: Option<(f64, f64)>,
    pointer: Option<(f64, f64)>,
}

impl FrameSidebar {
    /// The sidebar on the left (`left`) or right of `frame`, its parts made by their kinds on its grid.
    pub fn new(frame: Rc<WindowFrame>, left: bool) -> FrameSidebar {
        let spec = frame.sidebar(left).expect("a sidebar has its spec").clone();
        let mut grid = GridPanel::new(spec.columns.max(1)).owned(spec.item.as_deref());
        for part in &spec.parts {
            let made = parts::make(part.clone(), frame.host().clone());
            let place = parts::place_of(made.as_ref());
            grid.add(made, place);
        }
        let shows = frame.clone();
        let holder = Showing::new(Box::new(grid), move || !shows.collapsed(left));
        let mut node = Node::new();
        // As Avalonia's header: the fill button, then the fold button, then its parts, in Tab order.
        node.add(Box::new(SidebarButton { node: Node::new(), frame: frame.clone(), left, fills: true }));
        node.add(Box::new(SidebarButton { node: Node::new(), frame: frame.clone(), left, fills: false }));
        node.add(Box::new(holder));
        FrameSidebar { node, frame, left, dragged: None, dragging: None, pointer: None }
    }

    pub fn sidebar(&self) -> &SidebarSpec {
        self.frame.sidebar(self.left).expect("a sidebar has its spec")
    }

    pub fn is_left(&self) -> bool {
        self.left
    }

    /// Its parts' grid.
    pub fn grid(&self) -> &GridPanel {
        let holder = self.node.children()[2].as_any().downcast_ref::<Showing>().expect("a sidebar holds its parts");
        holder.child().as_any().downcast_ref::<GridPanel>().expect("a sidebar's parts are on a grid")
    }

    /// Its fill and fold buttons, the fill button first.
    pub fn buttons(&self) -> Vec<&SidebarButton> {
        self.node.children().iter().filter_map(|c| c.as_any().downcast_ref::<SidebarButton>()).collect()
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

    /// Folds it to its strip or shows it again; whether it fills the window stays as it was, folded it does not show.
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
        let (fill, fold) = (self.fill_button(), self.fold_button());
        for child in self.node.children_mut().iter_mut() {
            let place = match child.as_any().downcast_ref::<SidebarButton>() {
                Some(button) if button.fills => fill.unwrap_or_default(),
                Some(_) => fold.unwrap_or_default(),
                None => Rect::new(inner.x, inner.y + top, inner.width, (inner.height - top).max(0.0)),
            };
            child.arrange(place, cx);
        }
    }

    /// Over its edge, while the person may drag it, the pointer shows it resizes.
    fn cursor_at(&self, x: f64, y: f64) -> Option<Cursor> {
        self.edge().filter(|e| e.contains(x, y)).map(|_| Cursor::ResizeLeftRight)
    }

    fn on_pointer_pressed(&mut self, at: PointerInput, _cx: &mut Cx) {
        if self.edge().is_some_and(|e| e.contains(at.screen_x, at.screen_y)) {
            self.dragging = Some((at.screen_x, self.width()));
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
        let _clip = p.clip(b);
        draw_children(&self.node, p);
    }
}

/// A sidebar's fold or fill button: a click, Enter or Space folds it (or
/// shows it again) or lets it fill the window (or gives the window back).
/// It takes the focus, shows the focus ring, and says what it does.
pub struct SidebarButton {
    node: Node,
    frame: Rc<WindowFrame>,
    left: bool,
    fills: bool,
}

impl SidebarButton {
    /// Whether it is the fill button; else the fold button.
    pub fn fills(&self) -> bool {
        self.fills
    }

    fn sidebar(&self) -> &SidebarSpec {
        self.frame.sidebar(self.left).expect("a sidebar has its spec")
    }

    fn press(&mut self, cx: &mut Cx) {
        if self.fills {
            self.frame.fill(self.left, !self.frame.maximized(self.left));
        } else {
            self.frame.fold(self.left, !self.frame.collapsed(self.left));
        }
        cx.invalidate();
    }
}

impl Element for SidebarButton {
    crate::element_node!();

    /// The fold button shows on a sidebar that folds; the fill button on one that fills the window, while it is not folded.
    fn is_shown(&self) -> bool {
        if self.fills { self.sidebar().maximizable && !self.frame.collapsed(self.left) } else { self.sidebar().collapsible }
    }

    fn focusable(&self) -> bool {
        true
    }

    fn on_click(&mut self, cx: &mut Cx) {
        self.press(cx);
    }

    fn on_key(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        if !matches!(key.key, Key::Enter | Key::Space) || key.modifiers != Modifiers::NONE {
            return false;
        }
        self.press(cx);
        true
    }

    fn tooltip_at(&self, _x: f64, _y: f64) -> Option<String> {
        Some(match (self.fills, self.frame.maximized(self.left), self.frame.collapsed(self.left)) {
            (true, true, _) => locale::translate("sidebarGiveBack", "Give the window back"),
            (true, false, _) => locale::translate("sidebarFill", "Fill the window"),
            (false, _, true) => locale::translate("sidebarShow", "Show the sidebar"),
            (false, _, false) => locale::translate("sidebarFold", "Fold the sidebar away"),
        })
    }

    fn on_pointer_moved(&mut self, _at: PointerInput, cx: &mut Cx) {
        cx.invalidate();
    }

    fn draw(&self, p: &Painter) {
        let look = p.look;
        let b = self.node.bounds;
        let text = look.color("frame_text");
        let hover = self.node.hovered;
        // The fold button points the way the sidebar goes; the fill button shows the arrows out, or in while it fills.
        if self.fills {
            FrameSidebar::button(p, b, "", hover, text);
            let mark = look.part("button", if hover { Some("hover") } else { None }).text.unwrap_or(text);
            fill_mark(p, b, self.frame.maximized(self.left), mark);
        } else {
            let mark = if self.left != self.frame.collapsed(self.left) { "\u{2039}" } else { "\u{203A}" };
            FrameSidebar::button(p, b, mark, hover, text);
        }
        if self.node.focused {
            p.focus_ring(b, look.radius("button"));
        }
    }
}
