//! The slider (DESIGN 7.9, conformance/ranges.json): a number between its
//! lowest and highest value (its bound field's range when the part writes
//! none, else 0 and 100). Dragged, its thumb snaps to the steps and it
//! takes the nearest step when let go;
//! its keys move a step, ten, or to an end. Only a value that differs is a
//! change. Bound, it changes the field (a whole number, or exact text for
//! the rest); unbound, its state's text. As the shared screen draws it.

use std::rc::Rc;

use clarity_runtime::ranges::{self, Range, RangeKey};
use serde_json::Value;

use crate::element::{Cx, Element};
use crate::geometry::Rect;
use crate::input::{Key, KeyInput, PointerInput};
use crate::markup::PartSpec;
use crate::painter::Painter;
use crate::part_element;
use crate::parts::inputs::formats::{Exact, FormatKind};
use crate::parts::{Part, PartBase, PartHost};

/// How big the thumb is, and how high the track.
pub const THUMB: f64 = 16.0;
pub const TRACK: f64 = 4.0;

pub struct SliderPart {
    base: PartBase,
    /// Where it is dragged to, as a number; none while it is not dragged.
    dragging: Option<f64>,
    /// What an unbound part without a name keeps.
    kept: String,
}

pub fn make(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    Box::new(SliderPart { base: PartBase::new(spec, host), dragging: None, kept: String::new() })
}

fn number(text: &str) -> f64 {
    text.trim().parse().unwrap_or(0.0)
}

impl SliderPart {
    /// Its range: as the part writes it, else its bound field's own; 0 to 100 without ends, a step of 1 without one.
    pub fn range(&self) -> Range {
        let facts = self.base.bound().and_then(|b| self.base.host.field(b));
        let written = |name: &str| self.base.spec.text(name).filter(|t| !t.is_empty()).map(str::to_string);
        let min = written("min").or_else(|| facts.as_ref().and_then(|f| f.min.clone())).filter(|m| Exact::parse(m).is_some()).unwrap_or_else(|| "0".into());
        let max = written("max").or_else(|| facts.as_ref().and_then(|f| f.max.clone())).filter(|m| Exact::parse(m).is_some()).unwrap_or_else(|| "100".into());
        let max = if number(&max) < number(&min) { min.clone() } else { max };
        let step = written("step").filter(|s| number(s) > 0.0).unwrap_or_else(|| "1".into());
        Range { min: Some(min), max: Some(max), step }
    }

    /// Whether it holds whole numbers: its bound field's type says, else the range as written.
    pub fn whole(&self) -> bool {
        match self.base.bound().and_then(|b| self.base.host.type_of(b)).and_then(|t| FormatKind::of_type(&t)) {
            Some(FormatKind::Integer | FormatKind::Duration) => true,
            Some(_) => false,
            None => ["min", "max", "step"].iter().all(|n| !self.base.spec.text(n).unwrap_or("").contains('.')),
        }
    }

    /// Its value as exact text; none when it has none.
    pub fn value(&self) -> Option<String> {
        let held = match self.base.bound() {
            Some(path) => self.base.host.read(path),
            None => Some(Value::String(self.base.state().and_then(|s| s.text.clone()).unwrap_or_else(|| self.kept.clone()))),
        };
        held.as_ref().and_then(Exact::of).map(|e| e.text())
    }

    /// The value it shows: where it is dragged, else its value, else its lowest.
    pub fn shown(&self) -> f64 {
        self.dragging.or_else(|| self.value().map(|v| number(&v))).unwrap_or_else(|| number(self.range().min.as_deref().unwrap_or("0")))
    }

    fn track(&self) -> Rect {
        let b = self.base.node.bounds;
        Rect::new(b.x + THUMB / 2.0, b.y + (b.height - TRACK) / 2.0, (b.width - THUMB).max(0.0), TRACK)
    }

    /// The value at a point across it, within its range, not yet a step.
    fn at(&self, screen_x: f64) -> f64 {
        let range = self.range();
        let (low, high) = (number(range.min.as_deref().unwrap_or("0")), number(range.max.as_deref().unwrap_or("100")));
        let track = self.track();
        let part = if track.width <= 0.0 { 0.0 } else { ((screen_x - track.x) / track.width).clamp(0.0, 1.0) };
        low + part * (high - low)
    }

    /// The nearest step to the value at a point across it: where a dragged thumb shows.
    fn snapped(&self, screen_x: f64) -> f64 {
        let at = self.at(screen_x);
        ranges::dragged(&format!("{at}"), &self.range()).map(|s| number(&s)).unwrap_or(at)
    }

    fn usable(&self) -> bool {
        self.base.enabled() && !self.base.read_only()
    }

    /// Let go at `at` (a number as text): the nearest step, a change when it differs.
    pub fn drop_at(&mut self, at: &str) {
        if !self.usable() {
            return;
        }
        if let Some(next) = ranges::dragged(at, &self.range()) {
            self.set(next);
        }
    }

    /// A key: a step, ten, or an end from its value.
    pub fn press_key(&mut self, key: RangeKey) {
        if !self.usable() {
            return;
        }
        if let Some(next) = ranges::keyed(self.value().as_deref(), &self.range(), key) {
            self.set(next);
        }
    }

    fn set(&mut self, next: String) {
        let same = match (self.value().and_then(|v| Exact::parse(&v)), Exact::parse(&next)) {
            (Some(a), Some(b)) => a.text() == b.text(),
            _ => false,
        };
        if same {
            return;
        }
        let whole = self.whole();
        let name = self.base.name().map(str::to_string);
        if let Some(path) = self.base.bound().map(str::to_string) {
            let held = match Exact::parse(&next).filter(|_| whole).and_then(|e| i64::try_from(e.rounded(0).n).ok()) {
                Some(n) => Value::from(n),
                None => Value::String(next.clone()),
            };
            self.base.host.write(name.as_deref().unwrap_or(""), &path, held);
        } else {
            let text = if whole { Exact::parse(&next).map(|e| e.rounded(0).text()).unwrap_or(next) } else { next };
            match name {
                Some(name) => {
                    crate::parts::change_state(&*self.base.host, &name, |s| s.text = Some(text.clone()));
                    self.base.host.edited(&name);
                }
                None => self.kept = text,
            }
        }
        self.base.fire("change");
    }
}

impl Element for SliderPart {
    part_element!();

    fn focusable(&self) -> bool {
        self.base.enabled()
    }

    fn on_pointer_pressed(&mut self, at: PointerInput, cx: &mut Cx) {
        if !self.usable() {
            return;
        }
        self.dragging = Some(self.snapped(at.screen_x));
        cx.invalidate();
    }

    fn on_pointer_moved(&mut self, at: PointerInput, cx: &mut Cx) {
        if !self.base.node.pressed || self.dragging.is_none() {
            return;
        }
        // The thumb snaps to the steps while it is dragged, as it will when let go.
        self.dragging = Some(self.snapped(at.screen_x));
        cx.invalidate();
    }

    fn on_pointer_cancelled(&mut self, cx: &mut Cx) {
        self.dragging = None;
        cx.invalidate();
    }

    fn on_pointer_released(&mut self, at: PointerInput, cx: &mut Cx) {
        if self.dragging.take().is_none() {
            return;
        }
        let to = self.at(at.screen_x);
        self.drop_at(&format!("{to}"));
        cx.invalidate();
    }

    fn on_key(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        let moved = match key.key {
            Key::Up | Key::Right => RangeKey::Up,
            Key::Down | Key::Left => RangeKey::Down,
            Key::PageUp => RangeKey::PageUp,
            Key::PageDown => RangeKey::PageDown,
            Key::Home => RangeKey::Home,
            Key::End => RangeKey::End,
            _ => return false,
        };
        self.press_key(moved);
        cx.invalidate();
        true
    }

    fn on_focus_changed(&mut self, focused: bool, cx: &mut Cx) {
        self.base.fire(if focused { "enter" } else { "leave" });
        cx.invalidate();
    }

    fn draw(&self, p: &Painter) {
        let look = p.look;
        let node = &self.base.node;
        let _dimmed = p.dimmed(self.base.enabled());
        let range = self.range();
        let (low, high) = (number(range.min.as_deref().unwrap_or("0")), number(range.max.as_deref().unwrap_or("100")));
        let track = self.track();
        let span = high - low;
        let part = if span <= 0.0 { 0.0 } else { ((self.shown() - low) / span).clamp(0.0, 1.0) };
        let x = track.x + part * track.width;
        p.fill(track, look.color("strong_border"), TRACK / 2.0);
        p.fill(Rect { width: x - track.x, ..track }, look.color("accent"), TRACK / 2.0);
        let thumb = Rect::new(x - THUMB / 2.0, node.bounds.y + (node.bounds.height - THUMB) / 2.0, THUMB, THUMB);
        p.fill(thumb, look.color("accent"), THUMB / 2.0);
        p.fill(thumb.inset(if node.pressed || node.hovered { 5.0 } else { 4.0 }), look.color("on_accent"), THUMB / 2.0);
        if node.focused {
            p.focus_ring(thumb.inset(-3.0), THUMB / 2.0 + 3.0);
        }
    }
}

impl Part for SliderPart {
    fn base(&self) -> &PartBase {
        &self.base
    }
}
