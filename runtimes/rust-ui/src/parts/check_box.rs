//! The checkbox and the switch (DESIGN 7.9, conformance/forms.json): a box
//! with a tick, or a track with a knob, and the caption beside it. Its value
//! is bound, else kept in its state's `checked`. Only the person ticking it
//! is a change; what code sets is not. A read-only one cannot be ticked.
//! As the shared screen draws them.

use std::rc::Rc;

use serde_json::Value;
use skia_safe::{Color, Paint, PaintCap, PaintStyle};

use crate::element::{Cx, Element};
use crate::geometry::Rect;
use crate::input::{Key, KeyInput};
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
}

pub fn make(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    Box::new(CheckPart { base: PartBase::new(spec, host), kept: false })
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
            self.base.host.change_state(&name, &mut |s| s.checked = Some(next));
            self.base.host.edited(&name);
        } else {
            self.kept = next;
        }
        self.base.fire("change");
        cx.invalidate();
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
        self.base.enabled()
    }

    fn on_click(&mut self, cx: &mut Cx) {
        self.tick(cx);
    }

    fn on_key(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        if key.key != Key::Space {
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
            let shift = if on { TRACK_WIDTH - TRACK_HEIGHT } else { 0.0 };
            let knob = Rect::new(mark.x + shift + 2.0, mark.y + 2.0, TRACK_HEIGHT - 4.0, TRACK_HEIGHT - 4.0);
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
