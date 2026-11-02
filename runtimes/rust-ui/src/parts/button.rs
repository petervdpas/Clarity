//! A button (DESIGN 7.9): its click runs its handlers and opens the window
//! its `opens` names. Drawn in the theme's `button` look, with its hover and
//! pressed looks, a style over it, the ring of a default button, the focus
//! ring in the focus color and the dimmed look of a disabled part, as the
//! shared screen draws it. A split button's menu comes with the menus (G8b).

use std::rc::Rc;

use crate::element::{Cx, Element};
use crate::geometry::Rect;
use crate::input::{Key, KeyInput};
use crate::markup::PartSpec;
use crate::painter::{Corners, Painter, TextAlign};
use crate::part_element;
use crate::parts::{Part, PartBase, PartHost};
use crate::theme::LookValues;

pub struct ButtonPart {
    base: PartBase,
    /// Its round corners: all, or only the outer ones in a button group.
    pub corners: Corners,
}

pub fn make(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    Box::new(ButtonPart { base: PartBase::new(spec, host), corners: Corners::ALL })
}

impl ButtonPart {
    /// Its caption as shown: what code set, else as the design wrote it, translated.
    pub fn caption(&self) -> String {
        self.base.caption().unwrap_or_default()
    }

    /// Clicked, as a click on it, Enter or Space, or its shortcut does: its handlers run and the window it opens opens.
    pub fn click(&mut self, cx: &mut Cx) {
        if self.base.enabled() {
            self.base.fire("click");
            cx.invalidate();
        }
    }

    /// How a push looks now: the theme's button look, its style over it, and the hover or pressed look over both.
    fn push_look(&self, p: &Painter, hover: bool, pressed: bool) -> LookValues {
        let plain = p.look.part("button", None);
        let styled = match self.base.spec.text("style").filter(|s| !s.is_empty()) {
            Some(style) => p.look.style(style).over(&plain),
            None => plain,
        };
        if !self.base.enabled() || !(hover || pressed) {
            return styled;
        }
        p.look.part_own("button", Some(if pressed { "pressed" } else { "hover" })).over(&styled)
    }
}

impl Element for ButtonPart {
    part_element!();

    fn focusable(&self) -> bool {
        self.base.enabled()
    }

    fn on_click(&mut self, cx: &mut Cx) {
        self.click(cx);
    }

    fn on_key(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        if !self.base.enabled() || !matches!(key.key, Key::Enter | Key::Space) {
            return false;
        }
        self.click(cx);
        true
    }

    fn draw(&self, p: &Painter) {
        let node = &self.base.node;
        let bounds: Rect = node.bounds;
        let _dimmed = p.dimmed(self.base.enabled());
        let values = self.push_look(p, node.hovered, node.pressed && node.hovered);
        let radius = values.radius.map(|r| r as f64).unwrap_or_else(|| p.look.radius("button"));
        let fill = values.background.unwrap_or_else(|| p.look.color("raised"));
        let line = if self.base.spec.flag("default") { p.look.color("accent") } else { values.border.unwrap_or_else(|| p.look.color("border")) };
        p.plate(bounds, fill, line, p.look.border(), radius, self.corners);
        if node.focused {
            p.focus_ring(bounds, radius);
        }
        let size = values.size.map(|s| s as f64).unwrap_or_else(|| p.look.size("text"));
        let font = values.font.clone().unwrap_or_else(|| "text".to_string());
        p.text(&self.caption(), bounds.inset(p.look.padding()), values.text.unwrap_or_else(|| p.look.color("text")), size, &font, values.bold(), TextAlign::Center);
    }
}

impl Part for ButtonPart {
    fn base(&self) -> &PartBase {
        &self.base
    }

    fn press(&mut self, cx: &mut Cx) {
        self.click(cx);
    }
}
