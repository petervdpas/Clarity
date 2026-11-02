//! A heading: its caption in the theme's heading font and size.

use std::rc::Rc;

use crate::element::Element;
use crate::markup::PartSpec;
use crate::painter::{Painter, TextAlign};
use crate::part_element;
use crate::parts::{Part, PartBase, PartHost};

pub struct HeadingPart {
    base: PartBase,
}

pub fn make(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    Box::new(HeadingPart { base: PartBase::new(spec, host) })
}

impl HeadingPart {
    /// What it says.
    pub fn text(&self) -> String {
        self.base.caption().unwrap_or_default()
    }
}

impl Element for HeadingPart {
    part_element!();

    fn draw(&self, p: &Painter) {
        let bounds = self.base.node.bounds;
        let size = p.look.size("heading");
        let shown = p.fit(&self.text(), bounds.width, size, "heading", true);
        p.text(&shown, bounds, p.look.color("text"), size, "heading", true, TextAlign::Left);
    }
}

impl Part for HeadingPart {
    fn base(&self) -> &PartBase {
        &self.base
    }
}
