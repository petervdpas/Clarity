//! The toolbar under the menu bar: a row of buttons, each with its icon and
//! caption, and lines between groups of them. A click runs the button's
//! handler or opens the window it targets; the `language` and `theme`
//! targets open their choices below it. The toolbar hides and is greyed as
//! its part state says, and so does each button.

use std::rc::Rc;

use crate::element::{Cx, Element, Node};
use crate::geometry::Rect;
use crate::input::{Modifiers, PointerInput};
use crate::markup::CommandSpec;
use crate::painter::Painter;

use super::spec::ToolbarSpec;
use super::{buttons, over, owner, plate, rules, WindowFrame};

/// How high the toolbar is.
pub const HEIGHT: f64 = 38.0;

pub struct FrameToolbar {
    node: Node,
    frame: Rc<WindowFrame>,
    placed: Vec<(usize, Rect)>,
    pressed_at: Option<usize>,
    pointer: Option<(f64, f64)>,
}

impl FrameToolbar {
    pub fn new(frame: Rc<WindowFrame>) -> FrameToolbar {
        FrameToolbar { node: Node::new(), frame, placed: Vec::new(), pressed_at: None, pointer: None }
    }

    pub fn toolbar(&self) -> &ToolbarSpec {
        &self.frame.shown().toolbar.as_ref().expect("a toolbar has its spec").value
    }

    fn inherited(&self) -> bool {
        self.frame.shown().toolbar.as_ref().is_some_and(|t| t.inherited)
    }

    pub fn wanted(&self) -> f64 {
        HEIGHT
    }

    pub fn enabled(&self) -> bool {
        rules::takes(self.frame.host().as_ref(), self.toolbar().name.as_deref())
    }

    /// Its buttons as they show, each with its place.
    pub fn placed(&self) -> Vec<(&CommandSpec, Rect)> {
        self.placed.iter().map(|(i, r)| (&self.toolbar().items[*i], *r)).collect()
    }

    /// The place of the button with that caption, as written.
    pub fn place_of(&self, caption: &str) -> Option<Rect> {
        self.placed().into_iter().find(|(item, _)| item.text("caption") == Some(caption)).map(|(_, r)| r)
    }

    /// The button at the place `at` of its row clicked: it runs, unless it or the toolbar is disabled.
    pub fn click(&mut self, at: usize, cx: &mut Cx) {
        let Some((i, area)) = self.placed.get(at).copied() else { return };
        let item = self.toolbar().items[i].clone();
        if !self.enabled() || rules::is_separator(&item) || !rules::takes(self.frame.host().as_ref(), item.text("name")) {
            return;
        }
        self.frame.run(&item, area, owner(&self.node, i), cx);
    }
}

impl Element for FrameToolbar {
    crate::element_node!();

    /// It shows while its state is not hidden and one of its buttons shows; with nothing to show it takes no room.
    fn is_shown(&self) -> bool {
        let host = self.frame.host().as_ref();
        rules::shows(host, self.toolbar().name.as_deref()) && self.toolbar().items.iter().any(|i| !rules::is_separator(i) && rules::shows(host, i.text("name")))
    }

    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.node.bounds = area;
        self.placed.clear();
        let host = self.frame.host().clone();
        let mut x = area.x + 6.0;
        let frame = self.frame.clone();
        let toolbar = &frame.shown().toolbar.as_ref().expect("a toolbar has its spec").value;
        for (i, item) in toolbar.items.iter().enumerate() {
            if !rules::shows(host.as_ref(), item.text("name")) {
                continue;
            }
            let wide = buttons::width(host.as_ref(), item, false, cx);
            self.placed.push((i, Rect::new(x, area.y + 4.0, wide, area.height - 8.0)));
            x += wide + 4.0;
        }
    }

    /// A button says its caption and its keys when the pointer rests on it.
    fn tooltip_at(&self, x: f64, y: f64) -> Option<String> {
        let (i, _) = self.placed.iter().find(|(_, r)| r.contains(x, y))?;
        let item = &self.toolbar().items[*i];
        if rules::is_separator(item) {
            return None;
        }
        rules::tip_of(&self.frame.host().text(item.text("caption")), &self.frame.keys_of(item, self.inherited()))
    }

    fn on_pointer_pressed(&mut self, at: PointerInput, _cx: &mut Cx) {
        self.pressed_at = self.placed.iter().position(|(_, r)| r.contains(at.screen_x, at.screen_y));
    }

    fn on_click(&mut self, cx: &mut Cx) {
        if let Some(at) = self.pressed_at.take() {
            self.click(at, cx);
        }
    }

    fn on_pointer_moved(&mut self, at: PointerInput, cx: &mut Cx) {
        self.pointer = Some((at.screen_x, at.screen_y));
        cx.invalidate();
    }

    fn on_wheel(&mut self, _dx: f64, _dy: f64, _held: Modifiers, _cx: &mut Cx) -> bool {
        true
    }

    fn draw(&self, p: &Painter) {
        let b = self.node.bounds;
        plate(p, b, Rect::new(b.x, b.bottom() - p.look.border(), b.width, p.look.border()));
        let _dimmed = p.dimmed(self.enabled());
        let host = self.frame.host().as_ref();
        for (n, (i, area)) in self.placed.iter().enumerate() {
            let hover = over(&self.node, self.pointer, *area);
            buttons::draw(p, host, &self.toolbar().items[*i], *area, false, hover, self.node.pressed && self.pressed_at == Some(n) && hover);
        }
    }
}
