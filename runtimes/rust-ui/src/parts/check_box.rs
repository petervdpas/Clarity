//! The checkbox and the switch (DESIGN 7.9, conformance/forms.json): a box
//! with a tick, or a track with a knob, and the caption beside it. Its value
//! is bound, else kept in its state's `checked`. Only the person ticking it
//! is a change; what code sets is not. A click, Space or Enter ticks it; a
//! switch's knob can also be dragged to the other side. A read-only one
//! cannot be ticked and is no Tab stop. As the C# version draws them.

use std::rc::Rc;

use serde_json::Value;
use skia_safe::{Color, Paint, PaintCap, PaintStyle};

use crate::element::{Cx, Element};
use crate::geometry::Rect;
use crate::input::{Key, KeyInput, Modifiers, PointerInput};
use crate::markup::PartSpec;
use crate::painter::{Painter, TextAlign};
use crate::part_element;
use crate::parts::{Part, PartBase, PartHost};

/// How big the box is.
pub const BOX_SIZE: f64 = 16.0;
/// How wide and high the switch's track is.
pub const TRACK_WIDTH: f64 = 32.0;
pub const TRACK_HEIGHT: f64 = 18.0;

pub struct CheckPart {
    base: PartBase,
    /// What an unbound part without a name keeps.
    kept: bool,
    /// Where a press on a switch's track started, while its knob may be dragged.
    drag_from: Option<f64>,
    /// How far the knob was dragged, and whether far enough to be a drag.
    drag_by: f64,
    dragged: bool,
}

pub fn make(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    Box::new(CheckPart { base: PartBase::new(spec, host), kept: false, drag_from: None, drag_by: 0.0, dragged: false })
}

impl CheckPart {
    pub fn is_switch(&self) -> bool {
        self.base.spec.kind == "switch"
    }

    /// Its caption as shown: what code set, else as the design wrote it, translated.
    pub fn caption(&self) -> String {
        self.base.caption().unwrap_or_default()
    }

    /// Whether it is on: its bound value, else its state's, else its own.
    pub fn is_checked(&self) -> bool {
        match self.base.bound() {
            Some(path) => self.base.host.read(path) == Some(Value::Bool(true)),
            None => self.base.state().and_then(|s| s.checked).unwrap_or(self.kept),
        }
    }

    /// The person ticked it: it flips and tells its change.
    pub fn tick(&mut self, cx: &mut Cx) {
        if !self.base.enabled() || self.base.read_only() {
            return;
        }
        let next = !self.is_checked();
        let name = self.base.name().map(str::to_string);
        if let Some(path) = self.base.bound().map(str::to_string) {
            self.base.host.write(name.as_deref().unwrap_or(""), &path, Value::Bool(next));
        } else if let Some(name) = name {
            crate::parts::change_state(&*self.base.host, &name, |s| s.checked = Some(next));
            self.base.host.edited(&name);
        } else {
            self.kept = next;
        }
        self.base.fire("change");
        cx.invalidate();
    }

    /// Where the knob's left edge is, on or off, before a drag moves it.
    fn knob_left(&self, on: bool) -> f64 {
        self.mark().x + if on { TRACK_WIDTH - TRACK_HEIGHT } else { 0.0 } + 2.0
    }

    /// Where the box or the track is.
    fn mark(&self) -> Rect {
        let b = self.base.node.bounds;
        if self.is_switch() {
            Rect::new(b.x, b.y + (b.height - TRACK_HEIGHT) / 2.0, TRACK_WIDTH, TRACK_HEIGHT)
        } else {
            Rect::new(b.x + 1.0, b.y + (b.height - BOX_SIZE) / 2.0, BOX_SIZE, BOX_SIZE)
        }
    }
}

/// The tick inside a box.
fn draw_tick(p: &Painter, place: Rect, color: Color) {
    let mut paint = Paint::default();
    paint.set_color(color);
    paint.set_anti_alias(true);
    paint.set_style(PaintStyle::Stroke);
    paint.set_stroke_width(2.0);
    paint.set_stroke_cap(PaintCap::Round);
    let (x, y, s) = (place.x as f32, place.y as f32, place.width as f32);
    p.canvas.draw_line((x + s * 0.25, y + s * 0.52), (x + s * 0.43, y + s * 0.7), &paint);
    p.canvas.draw_line((x + s * 0.43, y + s * 0.7), (x + s * 0.76, y + s * 0.32), &paint);
}

impl Element for CheckPart {
    part_element!();

    fn focusable(&self) -> bool {
        self.base.enabled() && !self.base.read_only()
    }

    fn on_pointer_pressed(&mut self, at: PointerInput, _cx: &mut Cx) {
        self.dragged = false;
        self.drag_by = 0.0;
        let movable = self.is_switch() && self.base.enabled() && !self.base.read_only();
        self.drag_from = (movable && self.mark().contains(at.screen_x, at.screen_y)).then_some(at.screen_x);
    }

    fn on_pointer_moved(&mut self, at: PointerInput, cx: &mut Cx) {
        let Some(from) = self.drag_from else { return };
        if !self.base.node.pressed {
            return;
        }
        self.drag_by = at.screen_x - from;
        if self.drag_by.abs() > 3.0 {
            self.dragged = true;
        }
        cx.invalidate();
    }

    fn on_pointer_released(&mut self, _at: PointerInput, cx: &mut Cx) {
        if self.drag_from.take().is_none() || !self.dragged {
            return;
        }
        let on = self.is_checked();
        let knob = self.knob_left(on) + self.drag_by + (TRACK_HEIGHT - 4.0) / 2.0;
        if (knob > self.mark().x + TRACK_WIDTH / 2.0) != on {
            self.tick(cx);
        }
        cx.invalidate();
    }

    fn on_pointer_cancelled(&mut self, cx: &mut Cx) {
        self.drag_from = None;
        self.dragged = false;
        cx.invalidate();
    }

    fn on_click(&mut self, cx: &mut Cx) {
        // A drag of the knob already decided.
        if self.dragged {
            self.dragged = false;
            return;
        }
        self.tick(cx);
    }

    fn on_key(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        if !matches!(key.key, Key::Space | Key::Enter) || key.control() || key.modifiers.has(Modifiers::ALT) {
            return false;
        }
        self.tick(cx);
        true
    }

    fn draw(&self, p: &Painter) {
        let look = p.look;
        let node = &self.base.node;
        let enabled = self.base.enabled();
        let _dimmed = p.dimmed(enabled && !self.base.read_only());
        let on = self.is_checked();
        let mark = self.mark();
        if self.is_switch() {
            p.fill(mark, look.color(if on { "accent" } else { "strong_border" }), TRACK_HEIGHT / 2.0);
            // While it is dragged the knob follows the pointer, within the track.
            let moved = if self.drag_from.is_some() && self.dragged { self.drag_by } else { 0.0 };
            let left = (self.knob_left(on) + moved).clamp(mark.x + 2.0, mark.right() - TRACK_HEIGHT + 2.0);
            let knob = Rect::new(left, mark.y + 2.0, TRACK_HEIGHT - 4.0, TRACK_HEIGHT - 4.0);
            p.fill(knob, look.color("panel"), knob.height / 2.0);
        } else {
            let radius = look.radius("input").min(4.0);
            let error = self.base.state().and_then(|s| s.error).is_some_and(|e| !e.show().is_empty());
            p.fill(mark, look.color(if on { "accent" } else { "input" }), radius);
            if !on || error {
                let line = if error {
                    "error"
                } else if node.hovered && enabled {
                    "accent"
                } else {
                    "strong_border"
                };
                p.stroke(mark, look.color(line), look.border(), radius);
            }
            if on {
                draw_tick(p, mark, look.color("on_accent"));
            }
        }
        if node.focused {
            p.focus_ring(mark.inset(-3.0), if self.is_switch() { TRACK_HEIGHT / 2.0 + 3.0 } else { 6.0 });
        }
        let b = node.bounds;
        let text = Rect { x: mark.right() + 8.0, width: (b.right() - mark.right() - 8.0).max(0.0), ..b };
        p.text(&self.caption(), text, look.color("text"), look.size("text"), "text", false, TextAlign::Left);
    }
}

impl Part for CheckPart {
    fn base(&self) -> &PartBase {
        &self.base
    }

    fn press(&mut self, cx: &mut Cx) {
        self.tick(cx);
    }
}
