//! A separator: a line between parts, across the middle of its place, or
//! down it when vertical, in the theme's strong border color.

use std::rc::Rc;

use crate::element::Element;
use crate::geometry::Rect;
use crate::markup::PartSpec;
use crate::painter::Painter;
use crate::part_element;
use crate::parts::{Part, PartBase, PartHost};

pub struct SeparatorPart {
    base: PartBase,
}

pub fn make(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    Box::new(SeparatorPart { base: PartBase::new(spec, host) })
}

impl SeparatorPart {
    pub fn vertical(&self) -> bool {
        self.base.spec.text("orientation") == Some("vertical")
    }
}

impl Element for SeparatorPart {
    part_element!();

    fn draw(&self, p: &Painter) {
        let b = self.base.node.bounds;
        let width = p.look.border().max(1.0);
        let line = if self.vertical() {
            Rect::new((b.x + (b.width - width) / 2.0).round(), b.y, width, b.height)
        } else {
            Rect::new(b.x, (b.y + (b.height - width) / 2.0).round(), b.width, width)
        };
        p.fill(line, p.look.color("strong_border"), 0.0);
    }
}

impl Part for SeparatorPart {
    fn base(&self) -> &PartBase {
        &self.base
    }
}
