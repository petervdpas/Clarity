//! The menu bar at the top of a window: its menus by their captions. A
//! press on one opens its menu below it, and another press closes it; while
//! one is open, the pointer over another opens that one. A top item without
//! items runs as a click. The bar hides and is greyed as its part state
//! says, and so does each item.

use std::rc::Rc;

use crate::element::{Cx, Element, Node};
use crate::geometry::Rect;
use crate::input::{Modifiers, PointerInput};
use crate::markup::{CommandSpec, MenuSpec};
use crate::painter::{Painter, TextAlign};

use super::{choices, over, owner, plate, rules, WindowFrame};

/// How high the menu bar is.
pub const HEIGHT: f64 = 30.0;

const PADDING: f64 = 10.0;

pub struct FrameMenuBar {
    node: Node,
    frame: Rc<WindowFrame>,
    placed: Vec<(usize, Rect)>,
    pointer: Option<(f64, f64)>,
}

impl FrameMenuBar {
    pub fn new(frame: Rc<WindowFrame>) -> FrameMenuBar {
        FrameMenuBar { node: Node::new(), frame, placed: Vec::new(), pointer: None }
    }

    pub fn menu(&self) -> &MenuSpec {
        &self.frame.shown().menu.as_ref().expect("a menu bar has its menu").value
    }

    pub fn wanted(&self) -> f64 {
        HEIGHT
    }

    /// Whether it takes clicks: its state is not disabled.
    pub fn enabled(&self) -> bool {
        rules::takes(self.frame.host().as_ref(), self.menu().name.as_deref())
    }

    /// Its top items as they show, each with its place.
    pub fn placed(&self) -> Vec<(&CommandSpec, Rect)> {
        self.placed.iter().map(|(i, r)| (&self.menu().items[*i], *r)).collect()
    }

    /// The place of the top item with that caption, as written.
    pub fn place_of(&self, caption: &str) -> Option<Rect> {
        self.placed().into_iter().find(|(item, _)| item.text("caption") == Some(caption)).map(|(_, r)| r)
    }

    fn item_at(&self, x: f64, y: f64) -> Option<(usize, Rect)> {
        self.placed.iter().copied().find(|(i, r)| r.contains(x, y) && !rules::is_separator(&self.menu().items[*i]))
    }

    fn opens(item: &CommandSpec) -> bool {
        !item.items.is_empty() || rules::picker_of(item).is_some()
    }

    /// Opens the menu of the top item at `at` below it, or closes it when it is the one open.
    pub fn press(&mut self, at: usize, area: Rect, cx: &mut Cx) {
        let item = self.menu().items[at].clone();
        if !self.enabled() || !rules::takes(self.frame.host().as_ref(), item.text("name")) {
            return;
        }
        if !Self::opens(&item) {
            self.frame.run(&item, area, owner(&self.node, at), cx);
            return;
        }
        if self.frame.menus().owner() == Some(owner(&self.node, at)) {
            self.frame.menus().close_all(cx);
            return;
        }
        self.open(at, area, cx);
    }

    fn open(&mut self, at: usize, area: Rect, cx: &mut Cx) {
        let item = &self.menu().items[at];
        let entries = match rules::picker_of(item) {
            Some(target) if item.items.is_empty() => choices(&self.frame.picker_of(target)),
            _ => self.frame.entries_of(&item.items),
        };
        self.frame.menus().open(owner(&self.node, at), entries, area, Some(self.node.id()), cx);
        cx.invalidate();
    }
}

impl Element for FrameMenuBar {
    crate::element_node!();

    /// It shows while its state is not hidden and one of its items shows; with nothing to show it takes no room.
    fn is_shown(&self) -> bool {
        let host = self.frame.host().as_ref();
        rules::shows(host, self.menu().name.as_deref()) && self.menu().items.iter().any(|i| !rules::is_separator(i) && rules::shows(host, i.text("name")))
    }

    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.node.bounds = area;
        self.placed.clear();
        let host = self.frame.host().clone();
        let size = cx.look().size("text");
        let mut x = area.x + 6.0;
        let frame = self.frame.clone();
        let menu = &frame.shown().menu.as_ref().expect("a menu bar has its menu").value;
        for (i, item) in menu.items.iter().enumerate() {
            if !rules::shows(host.as_ref(), item.text("name")) {
                continue;
            }
            let wide = if rules::is_separator(item) { 9.0 } else { cx.fonts().measure(&host.text(item.text("caption")), "text", size, false) + 2.0 * PADDING };
            self.placed.push((i, Rect::new(x, area.y + 2.0, wide, area.height - 4.0)));
            x += wide + 2.0;
        }
    }

    fn on_pointer_pressed(&mut self, at: PointerInput, cx: &mut Cx) {
        if let Some((i, area)) = self.item_at(at.screen_x, at.screen_y) {
            self.press(i, area, cx);
        }
    }

    fn on_pointer_moved(&mut self, at: PointerInput, cx: &mut Cx) {
        self.pointer = Some((at.screen_x, at.screen_y));
        cx.invalidate();
        // While a menu of the bar is open, the pointer over another opens that one.
        let Some((region, open)) = self.frame.menus().owner() else { return };
        if region != self.node.id() {
            return;
        }
        if let Some((i, area)) = self.item_at(at.screen_x, at.screen_y) {
            let item = &self.menu().items[i];
            if i != open && Self::opens(item) && rules::takes(self.frame.host().as_ref(), item.text("name")) {
                self.open(i, area, cx);
            }
        }
    }

    /// A frame does not scroll the window's parts.
    fn on_wheel(&mut self, _dx: f64, _dy: f64, _held: Modifiers, _cx: &mut Cx) -> bool {
        true
    }

    fn draw(&self, p: &Painter) {
        let look = p.look;
        let b = self.node.bounds;
        plate(p, b, Rect::new(b.x, b.bottom() - look.border(), b.width, look.border()));
        let _dimmed = p.dimmed(self.enabled());
        let size = look.size("text");
        let host = self.frame.host().as_ref();
        let open = self.frame.menus().owner();
        for (i, area) in &self.placed {
            let item = &self.menu().items[*i];
            if rules::is_separator(item) {
                p.fill(Rect::new(area.x + 4.0, area.y + 4.0, look.border(), area.height - 8.0), look.color("strong_border"), 0.0);
                continue;
            }
            let takes = rules::takes(host, item.text("name"));
            if takes && (open == Some(owner(&self.node, *i)) || over(&self.node, self.pointer, *area)) {
                p.fill(*area, look.part("menu", Some("hover")).background.unwrap_or_else(|| look.color("accent_soft")), look.radius("button"));
            }
            let color = look.color(if takes { "frame_text" } else { "muted" });
            p.text(&host.text(item.text("caption")), *area, color, size, "text", false, TextAlign::Center);
        }
    }
}
