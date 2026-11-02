//! The link (DESIGN 7.9, conformance/small-parts.json): underlined text in
//! the accent color. It opens its address (`to`), or the address its bound
//! field holds (an email address to write to, one with a scheme as it is),
//! or runs its click when it has neither. The host opens an address in the
//! system's program for it (a browser, a mail program). As the shared
//! screen does it.

use std::rc::Rc;

use serde_json::Value;

use crate::element::{Cx, Element};
use crate::geometry::Rect;
use crate::input::{Key, KeyInput};
use crate::markup::PartSpec;
use crate::painter::{Painter, TextAlign};
use crate::part_element;
use crate::parts::{Part, PartBase, PartHost};

pub struct LinkPart {
    base: PartBase,
}

pub fn make(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    Box::new(LinkPart { base: PartBase::new(spec, host) })
}

/// What a link bound to a value opens: an email address to write to, an address with a scheme as it is, else none.
pub fn address(value: Option<&str>) -> Option<String> {
    let text = value.unwrap_or("").trim();
    if text.is_empty() {
        return None;
    }
    if text.contains('@') && !text.contains(':') {
        return Some(format!("mailto:{text}"));
    }
    has_scheme(text).then(|| text.to_string())
}

fn has_scheme(text: &str) -> bool {
    let Some(colon) = text.find(':') else { return false };
    let bytes = text.as_bytes();
    if colon < 1 || !bytes[0].is_ascii_alphabetic() {
        return false;
    }
    bytes[1..colon].iter().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'.' | b'-'))
}

impl LinkPart {
    /// The text it shows: its bound value, else its caption.
    pub fn shown(&self) -> String {
        match self.base.bound() {
            Some(path) => match self.base.host.read(path) {
                None | Some(Value::Null) => String::new(),
                Some(Value::String(s)) => s,
                Some(other) => other.to_string(),
            },
            None => self.base.caption().unwrap_or_default(),
        }
    }

    /// Opens its address, or the one its bound value holds; runs its click when it has neither.
    pub fn follow(&mut self, cx: &mut Cx) {
        if !self.base.enabled() {
            return;
        }
        let to = self.base.spec.text("to").filter(|t| !t.is_empty()).map(str::to_string);
        let held = self.base.bound().and_then(|path| match self.base.host.read(path) {
            Some(Value::String(s)) => address(Some(&s)),
            _ => None,
        });
        match to.clone().or(held) {
            Some(address) => self.base.host.open_address(&address),
            None if to.is_none() && self.base.bound().is_none() => {
                self.base.fire("click");
            }
            None => {}
        }
        cx.invalidate();
    }
}

impl Element for LinkPart {
    part_element!();

    fn focusable(&self) -> bool {
        self.base.enabled()
    }

    fn on_click(&mut self, cx: &mut Cx) {
        self.follow(cx);
    }

    fn on_key(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        if key.key != Key::Enter {
            return false;
        }
        self.follow(cx);
        true
    }

    fn draw(&self, p: &Painter) {
        let look = p.look;
        let b = self.base.node.bounds;
        let color = look.color(if self.base.enabled() { "accent" } else { "muted" });
        let size = look.size("text");
        let text = self.shown();
        p.text(&text, b, color, size, "text", false, TextAlign::Left);
        let width = p.fonts.measure(&text, "text", size, false).min(b.width);
        let y = b.y + b.height / 2.0 + size * 0.55;
        if width > 0.0 {
            p.line((b.x, y), (b.x + width, y), color, 1.0);
        }
        if self.base.node.focused {
            p.focus_ring(Rect::new(b.x - 2.0, b.y + b.height / 2.0 - size, width + 4.0, size * 2.0), 3.0);
        }
    }
}

impl Part for LinkPart {
    fn base(&self) -> &PartBase {
        &self.base
    }

    fn press(&mut self, cx: &mut Cx) {
        self.follow(cx);
    }
}
