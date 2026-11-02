//! The timer (DESIGN 5.26), as on the web: it shows nothing and has no place
//! in the window. While its window is on the screen and it is enabled, it
//! runs its tick handler every so many seconds (`every`, 1 unless given,
//! kept between a tenth of a second and a day). It starts as its window
//! joins the screen and stops as it leaves (closed, or under a window opened
//! in place); back on the screen, it starts again. Its ticks are the
//! screen's wake-ups, on the screen's thread; time a long handler took is
//! not made up: the next tick waits its whole time again.

use std::rc::Rc;
use std::time::{Duration, Instant};

use crate::element::{Cx, Element};
use crate::geometry::Rect;
use crate::markup::PartSpec;
use crate::painter::Painter;
use crate::parts::{Part, PartBase, PartHost};

pub struct TimerPart {
    base: PartBase,
    /// When it ticks next; none while it does not run.
    next: Option<Instant>,
    /// How many times it ticked.
    pub ticks: u64,
}

pub fn make(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    Box::new(TimerPart { base: PartBase::new(spec, host), next: None, ticks: 0 })
}

impl TimerPart {
    /// Seconds between its ticks: `every 0.5`; 1 when it says none, kept between a tenth of a second and a day.
    pub fn every(&self) -> f64 {
        let written = match self.base.spec.json("every") {
            Some(serde_json::Value::Number(n)) => n.as_f64(),
            Some(serde_json::Value::String(s)) => s.trim().parse::<f64>().ok(),
            _ => None,
        };
        written.filter(|s| s.is_finite()).map(|s| s.clamp(0.1, 86400.0)).unwrap_or(1.0)
    }

    /// Whether its clock runs.
    pub fn running(&self) -> bool {
        self.next.is_some()
    }

    /// When it ticks next; none while it does not run.
    pub fn next(&self) -> Option<Instant> {
        self.next
    }

    fn schedule(&mut self, from: Instant, cx: &mut Cx) {
        let at = from + Duration::from_secs_f64(self.every());
        self.next = Some(at);
        cx.wake(self.base.node.id(), at);
    }
}

impl Element for TimerPart {
    fn node(&self) -> &crate::element::Node {
        &self.base.node
    }
    fn node_mut(&mut self) -> &mut crate::element::Node {
        &mut self.base.node
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    fn as_part(&self) -> Option<&dyn Part> {
        Some(self)
    }
    fn as_part_mut(&mut self) -> Option<&mut dyn Part> {
        Some(self)
    }

    /// Never seen: it takes no pointer, no focus and draws nothing.
    fn is_shown(&self) -> bool {
        false
    }

    /// Laid out with its window: it takes no room.
    fn arrange(&mut self, area: Rect, _cx: &mut Cx) {
        self.base.node.bounds = Rect::new(area.x, area.y, 0.0, 0.0);
    }

    fn draw(&self, _p: &Painter) {}

    /// Its window joined the screen: its clock starts.
    fn on_attached(&mut self, cx: &mut Cx) {
        if self.next.is_none() {
            let now = cx.now();
            self.schedule(now, cx);
        }
    }

    /// Its window left the screen: its clock stops.
    fn on_detached(&mut self, _cx: &mut Cx) {
        self.next = None;
    }

    /// One tick: its handler runs when its window is on the screen and it is enabled; then it waits its time again.
    fn on_wake(&mut self, cx: &mut Cx) {
        if !self.base.node.attached() || self.next.is_none() {
            self.next = None;
            return;
        }
        // A tick whose handler cannot run now is left out, as C# TimerPart does.
        let can = self.base.spec.on("tick").all(|call| self.base.host.can_run(self.base.name(), call));
        if self.base.enabled() && can {
            self.ticks += 1;
            self.base.fire("tick");
            cx.invalidate();
        }
        // Measured from this tick: time a late wake-up lost is not made up.
        let now = cx.now();
        self.schedule(now, cx);
    }
}

impl Part for TimerPart {
    fn base(&self) -> &PartBase {
        &self.base
    }
}
