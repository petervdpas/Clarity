//! A trend (DESIGN 5.26), the same as on the web: the last values of one
//! number, taken every so many seconds while its window is open (hidden
//! too, so its history has no gap), drawn as Clarity's line chart with the
//! newest at the right. It takes a value when it is first laid out to be
//! drawn (hidden: when it joins the screen), and asks the screen to wake it
//! when the next is due (`Cx::wake`), so it draws once a second rather than
//! every frame.

use std::rc::Rc;
use std::time::{Duration, Instant};

use serde_json::Value;

use super::chart::{self, ChartMark, ChartShape, ChartShown};
use crate::element::{Cx, Element};
use crate::geometry::Rect;
use crate::input::PointerInput;
use crate::markup::PartSpec;
use crate::painter::Painter;
use crate::part_element;
use crate::parts::{Part, PartBase, PartHost};

/// How many values a trend keeps without `points`.
pub const KEPT_POINTS: usize = 60;

pub struct TrendPart {
    base: PartBase,
    history: Vec<f64>,
    last_taken: Option<Instant>,
    pointer: Option<(f64, f64)>,
}

pub fn make(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    Box::new(TrendPart { base: PartBase::new(spec, host), history: Vec::new(), last_taken: None, pointer: None })
}

impl TrendPart {
    /// How many values it keeps: `points 60`, 60 without it.
    pub fn points(&self) -> usize {
        self.base.spec.number("points").map(|n| n.max(1.0) as usize).unwrap_or(KEPT_POINTS)
    }

    /// Seconds between the values it takes: `every 0.5`, 1 without it, a tenth at least.
    pub fn every(&self) -> f64 {
        chart::read_number(self.base.spec.json("every")).unwrap_or(1.0).max(0.1)
    }

    /// The top of the axis, `max 100`; none lets it take in the largest value.
    pub fn max(&self) -> Option<f64> {
        chart::read_number(self.base.spec.json("max"))
    }

    /// The values it has taken, oldest first.
    pub fn history(&self) -> &[f64] {
        &self.history
    }

    /// Takes the value it follows now, keeping the last `points`; a value that is no number is not taken.
    pub fn take(&mut self) {
        let Some(bound) = self.base.bound() else { return };
        let Some(number) = chart::read_number(self.base.host.read(bound).as_ref()) else { return };
        self.history.push(number);
        let keep = self.points();
        if self.history.len() > keep {
            self.history.drain(..self.history.len() - keep);
        }
    }

    /// Takes the value when the next is due, and asks the screen to wake it for the one after.
    fn sample(&mut self, cx: &mut Cx) {
        let now = cx.now();
        let every = Duration::from_secs_f64(self.every());
        if self.last_taken.is_none_or(|last| now.saturating_duration_since(last) >= every) {
            self.take();
            self.last_taken = Some(now);
            cx.wake(self.base.node.id(), now + every);
        }
    }

    fn shown(&self) -> Vec<ChartShown> {
        let values: Vec<Value> = self.history.iter().map(|v| Value::from(*v)).collect();
        chart::numbers_shown(&values, 0)
    }

    /// Where each value is drawn.
    pub fn marks(&self) -> Vec<ChartMark> {
        chart::marks(self.base.node.bounds, ChartShape::Line, &self.shown(), self.max())
    }
}

impl Element for TrendPart {
    part_element!();

    /// Laid out to be drawn: a value is taken when the next is due, and the screen wakes it for the one after.
    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.base.node.bounds = area;
        if self.base.shown() {
            self.sample(cx);
        }
    }

    /// On its screen it starts taking values at once, shown or hidden, as Avalonia's trend did while its window was open.
    fn on_attached(&mut self, cx: &mut Cx) {
        if !self.base.shown() {
            self.sample(cx);
        }
    }

    /// The next value is due: shown, it draws again and laying it out takes
    /// it; hidden, it takes it now, so its history has no gap when it shows again.
    fn on_wake(&mut self, cx: &mut Cx) {
        if self.base.shown() {
            cx.invalidate();
        } else {
            self.sample(cx);
        }
    }

    fn on_pointer_moved(&mut self, at: PointerInput, cx: &mut Cx) {
        self.pointer = Some((at.screen_x, at.screen_y));
        cx.invalidate();
    }

    fn draw(&self, p: &Painter) {
        let pointer = self.pointer.filter(|_| self.base.node.hovered);
        chart::draw(p, self.base.node.bounds, ChartShape::Line, &self.shown(), self.max(), pointer);
    }
}

impl Part for TrendPart {
    fn base(&self) -> &PartBase {
        &self.base
    }

    fn grows(&self) -> bool {
        true
    }
}
