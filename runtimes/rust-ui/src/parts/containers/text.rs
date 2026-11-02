//! The text part: a value to read. What it binds, as the person reads it
//! (yes or no, a choice by its label, numbers and dates in the person's
//! format), on one line cut off with an ellipsis; over several rows wrapped
//! from the top, keeping its line breaks, and scrolling with the wheel when
//! it is longer than its rows. It takes no focus and edits nothing.

use std::rc::Rc;

use crate::element::{Cx, Element};
use crate::geometry::Rect;
use crate::input::Modifiers;
use crate::markup::PartSpec;
use crate::painter::{Painter, TextAlign};
use crate::part_element;
use crate::parts::containers::small::shown_value;
use crate::parts::text_editor::TextEditor;
use crate::parts::{Part, PartBase, PartHost};

pub struct TextPart {
    base: PartBase,
    /// Over several rows: the wrapped lines, read-only and frameless, which scroll.
    lines: Option<TextEditor>,
}

pub fn make(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    let base = PartBase::new(spec, host);
    let lines = (base.spec.number("height").unwrap_or(1.0) > 1.0).then(|| {
        let mut lines = TextEditor::new(true);
        lines.framed = false;
        lines.takes_focus = false;
        lines.read_only = true;
        lines
    });
    let mut part = TextPart { base, lines };
    part.refresh();
    Box::new(part)
}

impl TextPart {
    /// What it shows: its bound value as the person reads it; empty unbound or without a value.
    pub fn text(&self) -> String {
        match self.base.bound() {
            Some(path) => shown_value(self.base.host.as_ref(), path, self.base.host.read(path).as_ref()),
            None => String::new(),
        }
    }

    /// Over several rows: the wrapped lines; none on one row.
    pub fn lines(&self) -> Option<&TextEditor> {
        self.lines.as_ref()
    }

    /// Puts the value into the wrapped lines when it changed.
    fn refresh(&mut self) {
        let text = self.text();
        if let Some(lines) = self.lines.as_mut() {
            if lines.text() != text {
                lines.set_text(&text);
            }
        }
    }
}

impl Element for TextPart {
    part_element!();

    fn focusable(&self) -> bool {
        false
    }

    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.base.node.bounds = area;
        self.refresh();
        if let Some(lines) = self.lines.as_mut() {
            lines.place(area, cx);
        }
    }

    fn on_wheel(&mut self, _dx: f64, dy: f64, _held: Modifiers, cx: &mut Cx) -> bool {
        match self.lines.as_mut() {
            Some(lines) => lines.wheel(dy, cx),
            None => false,
        }
    }

    fn draw(&self, p: &Painter) {
        if let Some(lines) = self.lines.as_ref() {
            lines.paint(p);
            return;
        }
        let bounds = self.base.node.bounds;
        let size = p.look.size("text");
        let shown = p.fit(&self.text(), bounds.width, size, "text", false);
        p.text(&shown, bounds, p.look.color("text"), size, "text", false, TextAlign::Left);
    }
}

impl Part for TextPart {
    fn base(&self) -> &PartBase {
        &self.base
    }
}
