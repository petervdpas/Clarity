//! A toggle (DESIGN 7.9): a button that stays pressed, its value bound or
//! kept in its state's `checked`; the person pressing it is a change, what
//! code sets is not, and a read-only one is not pressed. Drawn as a button
//! in the theme's `button` look, in the soft accent with an accent line
//! while it is on, as the shared screen draws it; its icon before its
//! caption, or alone in the middle.

use std::rc::Rc;

use serde_json::Value;

use crate::element::{Cx, Element};
use crate::input::{Key, KeyInput};
use crate::markup::PartSpec;
use crate::painter::{Corners, Painter};
use crate::part_element;
use crate::parts::button::{draw_label, icon_of};
use crate::parts::{Part, PartBase, PartHost};
use crate::theme::LookValues;

pub struct TogglePart {
    base: PartBase,
    /// What an unbound part without a name keeps.
    kept: bool,
    /// Its round corners: all, or only the outer ones in a button group.
    pub corners: Corners,
}

pub fn make(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    Box::new(TogglePart { base: PartBase::new(spec, host), kept: false, corners: Corners::ALL })
}

impl TogglePart {
    /// Its caption as shown: what code set, else as the design wrote it, translated.
    pub fn caption(&self) -> String {
        self.base.caption().unwrap_or_default()
    }

    /// Whether it is pressed: its bound value, else its state's, else its own.
    pub fn is_on(&self) -> bool {
        match self.base.bound() {
            Some(path) => self.base.host.read(path) == Some(Value::Bool(true)),
            None => self.base.state().and_then(|s| s.checked).unwrap_or(self.kept),
        }
    }

    /// The person pressed it: it changes and tells its change.
    pub fn flip(&mut self, cx: &mut Cx) {
        if !self.base.enabled() || self.base.read_only() {
            return;
        }
        let next = !self.is_on();
        let name = self.base.name().map(str::to_string);
        if let Some(path) = self.base.bound().map(str::to_string) {
            self.base.host.write(name.as_deref().unwrap_or(""), &path, Value::Bool(next));
        } else if let Some(name) = name {
            crate::parts::change_state(&*self.base.host, &name, |s| s.checked = Some(next));
            self.base.host.edited(&name);
        } else {
            self.kept = next;
        }
        self.base.fire("change");
        cx.invalidate();
    }

    /// How its push looks now: the theme's button look, its style over it, and the hover or pressed look over both.
    fn push_look(&self, p: &Painter, hover: bool, pressed: bool) -> LookValues {
        let state = if !self.base.enabled() || !(hover || pressed) {
            None
        } else if pressed {
            Some("pressed")
        } else {
            Some("hover")
        };
        p.look.pressable("button", self.base.spec.text("style"), state)
    }

    /// Whether its style fills it: such a toggle that is on shows its style pressed, not the soft accent.
    fn style_fills(&self, p: &Painter) -> bool {
        self.base.spec.text("style").filter(|s| !s.is_empty()).is_some_and(|s| p.look.style(s).background.is_some())
    }
}

impl Element for TogglePart {
    part_element!();

    fn focusable(&self) -> bool {
        self.base.enabled()
    }

    fn on_click(&mut self, cx: &mut Cx) {
        self.flip(cx);
    }

    fn on_key(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        if !matches!(key.key, Key::Enter | Key::Space) {
            return false;
        }
        self.flip(cx);
        true
    }

    fn draw(&self, p: &Painter) {
        let node = &self.base.node;
        let bounds = node.bounds;
        let _dimmed = p.dimmed(self.base.enabled() && !self.base.read_only());
        let on = self.is_on();
        let styled_on = on && self.style_fills(p);
        let values = self.push_look(p, node.hovered || styled_on, (node.pressed && node.hovered) || styled_on);
        let radius = values.radius.map(|r| r as f64).unwrap_or_else(|| p.look.radius("button"));
        let fill = if on && !styled_on { p.look.color("accent_soft") } else { values.background.unwrap_or_else(|| p.look.color("raised")) };
        let line = if on || self.base.spec.flag("default") { p.look.color("accent") } else { values.border.unwrap_or_else(|| p.look.color("border")) };
        p.plate(bounds, fill, line, p.look.border(), radius, self.corners);
        if node.focused {
            p.focus_ring(bounds, radius);
        }
        let size = values.size.map(|s| s as f64).unwrap_or_else(|| p.look.size("text"));
        let font = values.font.clone().unwrap_or_else(|| "text".to_string());
        let icon = icon_of(&self.base);
        draw_label(p, icon.as_deref(), &self.caption(), bounds, values.text.unwrap_or_else(|| p.look.color("text")), size, &font, values.bold());
    }
}

impl Part for TogglePart {
    fn base(&self) -> &PartBase {
        &self.base
    }

    fn press(&mut self, cx: &mut Cx) {
        self.flip(cx);
    }

    /// In a button group, only its outer corners are round.
    fn set_corners(&mut self, corners: Corners) {
        self.corners = corners;
    }
}
