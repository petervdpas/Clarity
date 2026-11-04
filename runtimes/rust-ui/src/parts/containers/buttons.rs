//! A button group: the buttons and toggles it holds, joined in one row with
//! no gap; only its outer corners are round.

use std::rc::Rc;

use crate::element::{self, Cx, Element, GridPanel};
use crate::geometry::Rect;
use crate::markup::PartSpec;
use crate::painter::{Corners, Painter};
use crate::part_element;
use crate::parts::{Part, PartBase, PartHost};

use super::container;

pub struct ButtonsPart {
    base: PartBase,
}

pub fn make(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    let mut grid = container::grid_of(&spec.children, container::columns_of(&spec), 0.0, &host).owned(spec.text(crate::markup::DESIGN_ITEM));
    // Its pushes in the order they stand, left to right: the first is round on the left, the last on the right.
    let mut order: Vec<(f64, usize)> = spec.children.iter().enumerate().map(|(i, c)| (c.number("col").unwrap_or(1.0), i)).collect();
    order.sort_by(|a, b| a.0.total_cmp(&b.0));
    let count = order.len();
    for (at, (_, index)) in order.into_iter().enumerate() {
        let corners = Corners { left: at == 0, right: at + 1 == count };
        // A button or a toggle rounds only the corners it is given; any other part lets them go.
        if let Some(push) = grid.node_mut().children_mut()[index].as_part_mut() {
            push.set_corners(corners);
        }
    }
    let mut base = PartBase::new(spec, host);
    base.node.add(Box::new(grid));
    Box::new(ButtonsPart { base })
}

impl ButtonsPart {
    /// The grid of its pushes.
    pub fn inner(&self) -> &GridPanel {
        self.base.node.children()[0].as_any().downcast_ref::<GridPanel>().expect("a button group holds its grid")
    }
}

impl Element for ButtonsPart {
    part_element!();

    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.base.node.bounds = area;
        element::arrange_safely(self.base.node.children_mut()[0].as_mut(), area, cx);
    }

    fn draw(&self, p: &Painter) {
        element::draw_children(&self.base.node, p);
    }
}

impl Part for ButtonsPart {
    fn base(&self) -> &PartBase {
        &self.base
    }
}
