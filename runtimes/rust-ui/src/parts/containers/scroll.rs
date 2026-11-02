//! A scroll box: a grid of its parts as wide as it spans, in a place of its
//! own that scrolls with the wheel and its bars when the parts go further
//! down than its rows.

use std::rc::Rc;

use crate::element::{self, Cx, Element, GridPanel};
use crate::geometry::Rect;
use crate::grid::GAP;
use crate::markup::PartSpec;
use crate::painter::Painter;
use crate::part_element;
use crate::parts::{Part, PartBase, PartHost};

use super::container;
use super::scroll_view::ScrollView;

pub struct ScrollPart {
    base: PartBase,
}

pub fn make(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    let grid = container::grid_of(&spec.children, container::columns_of(&spec), GAP, &host);
    let mut base = PartBase::new(spec, host);
    base.node.add(Box::new(ScrollView::new(Box::new(grid))));
    Box::new(ScrollPart { base })
}

impl ScrollPart {
    pub fn view(&self) -> &ScrollView {
        self.base.node.children()[0].as_any().downcast_ref::<ScrollView>().expect("a scroll box holds its view")
    }

    /// The grid of its parts.
    pub fn inner(&self) -> &GridPanel {
        container::grid_in(self.view()).expect("a scroll box scrolls its grid")
    }
}

impl Element for ScrollPart {
    part_element!();

    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.base.node.bounds = area;
        self.base.node.children_mut()[0].arrange(area, cx);
    }

    fn draw(&self, p: &Painter) {
        let _framed = container::framing();
        element::draw_children(&self.base.node, p);
    }
}

impl Part for ScrollPart {
    fn base(&self) -> &PartBase {
        &self.base
    }

    fn grows(&self) -> bool {
        container::grid_grows(self.inner())
    }
}
