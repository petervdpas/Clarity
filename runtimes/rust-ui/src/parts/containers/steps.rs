//! A wizard, the steps part (DESIGN 5.15, conformance/steps.json): one step
//! at a time, the titles of all steps above with the current one marked, and
//! Clarity's own Back, Next and Finish below. The step it is on, counted from
//! 1, is its part's state's `step`, so code reads and sets it; a wizard
//! without a name keeps its own. Next runs the handler of `on next` on the
//! step shown, then moves one step on, unless the handler refused (the step
//! stays) or set the step itself (that step wins). Back goes one step back
//! and runs nothing. Finish runs `on finish`.

use std::cell::Cell;
use std::rc::Rc;

use crate::element::{draw_children, Cx, Element};
use crate::geometry::Rect;
use crate::locale;
use crate::markup::PartSpec;
use crate::painter::{Painter, TextAlign};
use crate::part_element;
use crate::parts::{Part, PartBase, PartHost};

use super::container::{self, ContainerButton, LookNow, Page};

/// What the wizard and its buttons share: its spec and host, and its own step when it has no name.
struct Wizard {
    spec: PartSpec,
    host: Rc<dyn PartHost>,
    count: usize,
    step_here: Cell<i64>,
}

impl Wizard {
    fn step(&self) -> i64 {
        match self.host.state(self.spec.name()) {
            Some(state) => state.step.unwrap_or(1),
            None => self.step_here.get(),
        }
    }

    fn set_step(&self, step: i64) {
        match self.spec.name() {
            Some(name) if self.host.state(Some(name)).is_some() => crate::parts::change_state(&*self.host, name, |s| s.step = Some(step)),
            _ => self.step_here.set(step),
        }
    }

    fn shown(&self) -> i64 {
        clarity_runtime::steps::shown(Some(self.step()), self.count as i64)
    }

    fn enabled(&self) -> bool {
        self.host.state(self.spec.name()).and_then(|s| s.enabled).unwrap_or(true)
    }

    /// Runs the handlers of `event`: false when one refused.
    fn ask(&self, event: &str) -> bool {
        self.spec.on(event).all(|call| self.host.run(self.spec.name(), call))
    }

    /// Next: the handler runs on the step shown, then the wizard moves on unless it refused or chose the step.
    fn go_next(&self) {
        let before = self.shown();
        self.set_step(before);
        if !self.ask("next") {
            return;
        }
        if self.shown() == before && before < self.count as i64 {
            self.set_step(before + 1);
        }
    }

    /// Back: one step back; nothing runs and what was entered stays.
    fn go_back(&self) {
        let shown = self.shown();
        if shown > 1 {
            self.set_step(shown - 1);
        }
    }
}

pub struct StepsPart {
    base: PartBase,
    wizard: Rc<Wizard>,
    look: Cell<LookNow>,
}

pub fn make(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    let columns = container::columns_of(&spec);
    let wizard = Rc::new(Wizard { spec: spec.clone(), host: host.clone(), count: spec.pages.len(), step_here: Cell::new(crate::markup::design_shown(&spec)) });
    let mut base = PartBase::new(spec, host.clone());
    for page in &base.spec.pages.clone() {
        base.node.add(Box::new(Page::new(Box::new(container::grid_of(&page.children, page.grid.unwrap_or(columns).max(1), crate::grid::GAP, &host).owned(page.item.as_deref())))));
    }
    let (w, usable) = (wizard.clone(), wizard.clone());
    let back = ContainerButton::new(|| locale::translate("back", "Back"), move |_| w.go_back()).shows({
        let w = wizard.clone();
        move || w.shown() > 1
    });
    base.node.add(Box::new(back.usable(move || usable.enabled())));
    let (w, usable) = (wizard.clone(), wizard.clone());
    let next = ContainerButton::new(|| locale::translate("next", "Next"), move |_| w.go_next()).accent().shows({
        let w = wizard.clone();
        move || w.shown() < w.count as i64
    });
    base.node.add(Box::new(next.usable(move || usable.enabled())));
    let (w, usable) = (wizard.clone(), wizard.clone());
    let finish = ContainerButton::new(|| locale::translate("finish", "Finish"), move |_| {
        w.ask("finish");
    })
    .accent()
    .shows({
        let w = wizard.clone();
        move || w.shown() >= w.count as i64
    });
    base.node.add(Box::new(finish.usable(move || usable.enabled())));
    Box::new(StepsPart { base, wizard, look: Cell::new(LookNow::default()) })
}

impl StepsPart {
    /// How high the line of titles and the line of buttons are.
    pub const TITLES_HEIGHT: f64 = 28.0;
    pub const BUTTONS_HEIGHT: f64 = 32.0;

    /// The step it is on, from 1: the part's state's, or its own without a name.
    pub fn step(&self) -> i64 {
        self.wizard.step()
    }

    /// Sets the step it is on, as code does.
    pub fn set_step(&self, step: i64) {
        self.wizard.set_step(step);
    }

    /// The step shown: the step it is on, or the nearest there is.
    pub fn shown(&self) -> i64 {
        self.wizard.shown()
    }

    /// Its steps' pages, in order.
    pub fn pages(&self) -> Vec<&Page> {
        self.base.node.children()[..self.wizard.count].iter().filter_map(|c| c.as_any().downcast_ref::<Page>()).collect()
    }

    fn button(&self, at: usize) -> &ContainerButton {
        self.base.node.children()[self.wizard.count + at].as_any().downcast_ref::<ContainerButton>().expect("a wizard's button")
    }

    pub fn back(&self) -> &ContainerButton {
        self.button(0)
    }

    pub fn next(&self) -> &ContainerButton {
        self.button(1)
    }

    pub fn finish(&self) -> &ContainerButton {
        self.button(2)
    }
}

impl Element for StepsPart {
    part_element!();

    /// The room it needs beyond its rows: the line of titles, the line of buttons and their padding over its highest step.
    fn around(&self) -> f64 {
        let highest = self.pages().into_iter().map(|p| container::least_of(p.grid())).fold(0.0, f64::max);
        container::around_for(&self.base.spec, Self::TITLES_HEIGHT + Self::BUTTONS_HEIGHT + 4.0 * self.look.get().padding, highest)
    }

    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.base.node.bounds = area;
        self.look.set(LookNow::of(cx.look()));
        let pad = cx.look().padding();
        let shown = self.wizard.shown() as usize;
        let count = self.wizard.count;
        let top = area.y + Self::TITLES_HEIGHT + 2.0 * pad;
        let bottom = area.bottom() - Self::BUTTONS_HEIGHT - 2.0 * pad;
        let page_area = Rect::new(area.x, top, area.width, (bottom - top).max(0.0));
        let children = self.base.node.children_mut();
        for (i, child) in children[..count].iter_mut().enumerate() {
            let page = child.as_any_mut().downcast_mut::<Page>().expect("a wizard's page");
            page.shown = i + 1 == shown;
            if page.shown {
                crate::element::arrange_safely(page, page_area, cx);
            }
        }
        // The buttons on the right, the last one at the edge.
        let mut right = area.right();
        for at in [2, 1, 0] {
            let button = children[count + at].as_any_mut().downcast_mut::<ContainerButton>().expect("a wizard's button");
            if !button.is_shown() {
                crate::element::arrange_safely(button, Rect::new(right, area.bottom() - Self::BUTTONS_HEIGHT, 0.0, Self::BUTTONS_HEIGHT), cx);
                continue;
            }
            let width = button.width_for(cx).max(80.0);
            right -= width;
            crate::element::arrange_safely(button, Rect::new(right, area.bottom() - Self::BUTTONS_HEIGHT, width, Self::BUTTONS_HEIGHT), cx);
            right -= 2.0 * pad;
        }
    }

    fn draw(&self, p: &Painter) {
        let look = p.look;
        let bounds = self.base.node.bounds;
        let shown = self.wizard.shown() as usize;
        let size = look.size("text");
        let mut x = bounds.x;
        for (i, page) in self.base.spec.pages.iter().enumerate() {
            let title = self.base.host.text(page.title.as_deref().filter(|t| !t.is_empty()));
            let current = i + 1 == shown;
            let color = look.color(if current { "accent" } else if i + 1 < shown { "text" } else { "muted" });
            let width = p.fonts.measure(&title, "text", size, current);
            let room = (width + 1.0).min(bounds.right() - x).max(0.0);
            p.text(&title, Rect::new(x, bounds.y, room, Self::TITLES_HEIGHT), color, size, "text", current, TextAlign::Left);
            x += width + 6.0 * look.padding();
        }
        p.fill(Rect::new(bounds.x, bounds.y + Self::TITLES_HEIGHT, bounds.width, look.border()), look.color("strong_border"), 0.0);
        let _framing = container::framing();
        draw_children(&self.base.node, p);
    }
}

impl Part for StepsPart {
    fn base(&self) -> &PartBase {
        &self.base
    }

    /// A wizard grows when a part of one of its steps does.
    fn grows(&self) -> bool {
        self.pages().iter().any(|p| container::grid_grows(p.grid()))
    }
}
