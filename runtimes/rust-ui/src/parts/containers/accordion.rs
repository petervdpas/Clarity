//! An accordion (U1b, conformance/accordions.json): sections that fold, one
//! open at a time. Each section's header is a button; the open section's
//! parts fill the room under it. The open section, counted from 1 and 0 for
//! none, is its part's state's `open`, so code reads and sets it. A press on
//! a header opens that section, or folds it when it was the open one, and
//! runs the part's handler of `on change`.

use std::cell::Cell;
use std::rc::Rc;

use crate::element::{self, Cx, Element, GridPanel};
use crate::geometry::Rect;
use crate::grid::GAP;
use crate::markup::PartSpec;
use crate::painter::Painter;
use crate::part_element;
use crate::parts::{Part, PartBase, PartHost};

use super::container::{self, ContainerButton, LookNow, Page};

/// How high a section's header is.
pub const SECTION_HEIGHT: f64 = 32.0;

/// The room between headers.
const BETWEEN: f64 = 2.0;

/// The section shown open when `open` is asked of `count`: it, when it is there; else none.
pub fn shown(open: i64, count: usize) -> i64 {
    if open >= 1 && open as usize <= count { open } else { 0 }
}

/// The open section after a press on the header of `press`: it, or none when it was the open one.
pub fn after_press(open: i64, press: i64, count: usize) -> i64 {
    if shown(open, count) == press { 0 } else { press }
}

/// What the accordion shares with its headers: the part and its own open section.
struct Shared {
    part: PartBase,
    here: Cell<i64>,
    count: usize,
}

impl Shared {
    fn open(&self) -> i64 {
        self.part.state().and_then(|s| s.open).unwrap_or(self.here.get())
    }

    fn set_open(&self, open: i64) {
        match self.part.name() {
            Some(name) if self.part.state().is_some() => crate::parts::change_state(&*self.part.host, name, |s| s.open = Some(open)),
            _ => self.here.set(open),
        }
    }

    /// A press on the header of section `press`, from 1.
    fn press(&self, press: i64) {
        self.set_open(after_press(self.open(), press, self.count));
        self.part.fire("change");
    }
}

pub struct AccordionPart {
    base: PartBase,
    shared: Rc<Shared>,
    look: Cell<LookNow>,
}

pub fn make(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    let columns = container::columns_of(&spec);
    let count = spec.pages.len();
    let shared = Rc::new(Shared { part: PartBase::new(spec.clone(), host.clone()), here: Cell::new(crate::markup::design_shown(&spec)), count });
    let mut base = PartBase::new(spec.clone(), host.clone());
    for (i, page) in spec.pages.iter().enumerate() {
        let number = i as i64 + 1;
        let (text_of, press_of, usable_of) = (shared.clone(), shared.clone(), shared.clone());
        let title = page.title.clone();
        let mut header = ContainerButton::new(
            move || title.as_deref().filter(|t| !t.is_empty()).map(|t| text_of.part.host.text(Some(t))).unwrap_or_default(),
            move |_cx| press_of.press(number),
        )
        .usable(move || usable_of.part.enabled());
        header.chevron = Some(false);
        base.node.add(Box::new(header));
    }
    for page in &spec.pages {
        base.node.add(Box::new(Page::new(Box::new(container::grid_of(&page.children, columns, GAP, &host).owned(page.item.as_deref())))));
    }
    Box::new(AccordionPart { base, shared, look: Cell::new(LookNow::default()) })
}

impl AccordionPart {
    /// The section asked to be open, from 1, 0 for none: the part's state's, or its own without a name.
    pub fn open(&self) -> i64 {
        self.shared.open()
    }

    /// The headers of its sections, in order.
    pub fn headers(&self) -> Vec<&ContainerButton> {
        self.base.node.children()[..self.shared.count].iter().filter_map(|c| c.as_any().downcast_ref::<ContainerButton>()).collect()
    }

    /// The pages of its sections; only the open one shows.
    pub fn pages(&self) -> Vec<&Page> {
        self.base.node.children()[self.shared.count..].iter().filter_map(|c| c.as_any().downcast_ref::<Page>()).collect()
    }

    pub fn grids(&self) -> Vec<&GridPanel> {
        self.pages().into_iter().map(Page::grid).collect()
    }

    /// A press on the header of section `press`, from 1.
    pub fn press(&mut self, press: i64, cx: &mut Cx) {
        self.shared.press(press);
        cx.invalidate();
    }
}

impl Element for AccordionPart {
    part_element!();

    /// The room it needs beyond its rows: its headers and the open section's padding over its highest section.
    fn around(&self) -> f64 {
        let count = self.shared.count;
        let headers = count as f64 * SECTION_HEIGHT + count.saturating_sub(1) as f64 * BETWEEN;
        let highest = self.grids().into_iter().map(container::least_of).fold(0.0, f64::max);
        container::around_for(&self.base.spec, headers + 4.0 * self.look.get().padding, highest)
    }

    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.base.node.bounds = area;
        let look = LookNow::of(cx.look());
        self.look.set(look);
        let pad = look.padding;
        let count = self.shared.count;
        let open = shown(self.shared.open(), count) as usize;
        let headers = count as f64 * SECTION_HEIGHT + count.saturating_sub(1) as f64 * BETWEEN;
        let room = (area.height - headers - 4.0 * pad).max(0.0);
        let mut y = area.y;
        for i in 0..count {
            let on = i + 1 == open;
            let children = self.base.node.children_mut();
            if let Some(header) = children[i].as_any_mut().downcast_mut::<ContainerButton>() {
                header.chevron = Some(on);
            }
            element::arrange_safely(children[i].as_mut(), Rect::new(area.x, y, area.width, SECTION_HEIGHT), cx);
            y += SECTION_HEIGHT + BETWEEN;
            if let Some(page) = children[count + i].as_any_mut().downcast_mut::<Page>() {
                page.shown = on;
            }
            if on {
                element::arrange_safely(children[count + i].as_mut(), Rect::new(area.x, y + 2.0 * pad - BETWEEN, area.width, room), cx);
                y += room + 4.0 * pad;
            }
        }
    }

    fn draw(&self, p: &Painter) {
        let _framed = container::framing();
        element::draw_children(&self.base.node, p);
    }
}

impl Part for AccordionPart {
    fn base(&self) -> &PartBase {
        &self.base
    }

    fn grows(&self) -> bool {
        self.grids().into_iter().any(container::grid_grows)
    }
}
