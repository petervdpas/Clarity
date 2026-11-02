//! A label: its caption in the muted color, with a mark when its input's field is required.

use std::rc::Rc;

use crate::element::Element;
use crate::geometry::Rect;
use crate::markup::PartSpec;
use crate::painter::{Painter, TextAlign};
use crate::part_element;
use crate::parts::{Part, PartBase, PartHost};

pub struct LabelPart {
    base: PartBase,
}

pub fn make(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    Box::new(LabelPart { base: PartBase::new(spec, host) })
}

impl LabelPart {
    /// What it says.
    pub fn text(&self) -> String {
        self.base.caption().unwrap_or_default()
    }

    /// Whether its input's field (the part its `for` names) is required.
    pub fn required(&self) -> bool {
        let host = &self.base.host;
        self.base.spec.text("for").and_then(|target| host.bound_of(target)).and_then(|path| host.field(&path)).is_some_and(|f| f.required)
    }
}

impl Element for LabelPart {
    part_element!();

    fn draw(&self, p: &Painter) {
        let bounds = self.base.node.bounds;
        let size = p.look.size("text");
        let mark = if self.required() { " *" } else { "" };
        let mark_width = if mark.is_empty() { 0.0 } else { p.fonts.measure(mark, "text", size, false) };
        let shown = p.fit(&self.text(), bounds.width - mark_width, size, "text", false);
        p.text(&shown, bounds, p.look.color("muted"), size, "text", false, TextAlign::Left);
        if !mark.is_empty() {
            let x = bounds.x + p.fonts.measure(&shown, "text", size, false);
            p.text(mark, Rect { x, width: mark_width + 1.0, ..bounds }, p.look.color("error"), size, "text", false, TextAlign::Left);
        }
    }
}

impl Part for LabelPart {
    fn base(&self) -> &PartBase {
        &self.base
    }
}
