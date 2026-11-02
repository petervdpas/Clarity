//! The toolbar under the menu bar: a row of buttons, each with its icon and
//! caption, and lines between groups of them. A click runs the button's
//! handler or opens the window it targets; the `language` and `theme`
//! targets open their choices below it. The toolbar hides and is greyed as
//! its part state says, and so does each button.

use std::rc::Rc;

use crate::element::{draw_children, Cx, Element, Node};
use crate::geometry::Rect;
use crate::input::Modifiers;
use crate::markup::CommandSpec;
use crate::painter::Painter;

use super::buttons::FrameButton;
use super::spec::ToolbarSpec;
use super::{buttons, owner, plate, rules, WindowFrame};

/// How high the toolbar is.
pub const HEIGHT: f64 = 38.0;

/// A toolbar: its buttons are its children, one per item, in the order written.
pub struct FrameToolbar {
    node: Node,
    frame: Rc<WindowFrame>,
    placed: Vec<(usize, Rect)>,
}

impl FrameToolbar {
    pub fn new(frame: Rc<WindowFrame>) -> FrameToolbar {
        let mut node = Node::new();
        let toolbar = &frame.shown().toolbar.as_ref().expect("a toolbar has its spec").value;
        for (i, item) in toolbar.items.iter().enumerate() {
            node.add(Box::new(FrameButton::new(frame.clone(), item.clone(), false, toolbar.name.clone(), owner(&node, i))));
        }
        FrameToolbar { node, frame, placed: Vec::new() }
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

    /// Its buttons, one per item, shown or not.
    pub fn buttons(&self) -> Vec<&FrameButton> {
        self.node.children().iter().filter_map(|c| c.as_any().downcast_ref::<FrameButton>()).collect()
    }

    /// The button at the place `at` of its row clicked: it runs, unless it or the toolbar is disabled.
    pub fn click(&mut self, at: usize, cx: &mut Cx) {
        let Some((i, _)) = self.placed.get(at).copied() else { return };
        if let Some(button) = self.node.children_mut().get_mut(i).and_then(|c| c.as_any_mut().downcast_mut::<FrameButton>()) {
            button.press(cx);
        }
    }
}

impl Element for FrameToolbar {
    crate::element_node!();

    /// It shows while its state is not hidden and one of its buttons shows; with nothing to show it takes no room.
    fn is_shown(&self) -> bool {
        let host = self.frame.host().as_ref();
        rules::shows(host, self.toolbar().name.as_deref()) && self.toolbar().items.iter().any(|i| !rules::is_separator(i) && rules::shows(host, i.text("name")))
    }

    /// A disabled toolbar's buttons take no press, focus or key.
    fn is_enabled(&self) -> bool {
        self.enabled()
    }

    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.node.bounds = area;
        self.placed.clear();
        let host = self.frame.host().clone();
        let mut x = area.x + 6.0;
        for (i, child) in self.node.children_mut().iter_mut().enumerate() {
            let Some(button) = child.as_any_mut().downcast_mut::<FrameButton>() else { continue };
            if !rules::shows(host.as_ref(), button.item().text("name")) {
                button.place(None, cx);
                continue;
            }
            let wide = buttons::width(host.as_ref(), button.item(), false, cx);
            let place = Rect::new(x, area.y + 4.0, wide, area.height - 8.0);
            button.place(Some(place), cx);
            self.placed.push((i, place));
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

    fn on_wheel(&mut self, _dx: f64, _dy: f64, _held: Modifiers, _cx: &mut Cx) -> bool {
        true
    }

    fn draw(&self, p: &Painter) {
        let b = self.node.bounds;
        plate(p, b, Rect::new(b.x, b.bottom() - p.look.border(), b.width, p.look.border()));
        let _dimmed = p.dimmed(self.enabled());
        draw_children(&self.node, p);
    }
}
