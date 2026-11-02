//! The radio group (DESIGN 7.9, conformance/forms.json): one round button
//! per choice of its bound field (an enum's values, or what the host
//! offers), side by side or one under the other. Only the person picking
//! another choice is a change; what code sets is not. The arrow keys pick
//! the next or the previous choice, as a system's radio group does. As the
//! shared screen draws it.

use std::rc::Rc;

use serde_json::Value;

use crate::element::{Cx, Element};
use crate::fonts::Fonts;
use crate::geometry::Rect;
use crate::input::{Key, KeyInput, PointerInput};
use crate::markup::PartSpec;
use crate::painter::{Painter, TextAlign};
use crate::part_element;
use crate::parts::inputs::choices::{chosen, offered_for, same};
use crate::parts::{Choice, Part, PartBase, PartHost};
use crate::theme::ThemeLook;

/// How big a button's circle is.
pub const CIRCLE: f64 = 16.0;

/// How far a label may draw past its measured width.
const LABEL_SLACK: f64 = 4.0;
/// How high a line of choices is.
pub const LINE: f64 = 24.0;
/// The room between choices side by side, and between lines.
pub const ACROSS: f64 = 16.0;
pub const DOWN: f64 = 4.0;

pub struct RadioPart {
    base: PartBase,
    /// The choice the keys are at while it has the focus.
    focus_at: Option<usize>,
    /// The choice a press began on.
    pressed_at: Option<usize>,
    /// Where each choice is, as it was last laid out.
    places: Vec<Rect>,
}

pub fn make(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    Box::new(RadioPart { base: PartBase::new(spec, host), focus_at: None, pressed_at: None, places: Vec::new() })
}

impl RadioPart {
    /// Whether its choices stand one under the other.
    pub fn vertical(&self) -> bool {
        self.base.spec.text("orientation") == Some("vertical")
    }

    pub fn choices(&self) -> Vec<Choice> {
        offered_for(self.base.host.as_ref(), self.base.bound())
    }

    pub fn value(&self) -> Value {
        self.base.bound().and_then(|b| self.base.host.read(b)).unwrap_or(Value::Null)
    }

    /// Where its value is among its choices; none when it is none of them.
    pub fn chosen(&self) -> Option<usize> {
        chosen(&self.choices(), &self.value())
    }

    /// Where each choice is (its circle and its label), as it was last laid out.
    pub fn places(&self) -> &[Rect] {
        &self.places
    }

    /// Where each choice is: flowing on in lines, or one under the other; the lines in the middle of its height when they fit, else from its top.
    pub fn lay_out(&self, fonts: &Fonts, look: &ThemeLook) -> Vec<Rect> {
        let b = self.base.node.bounds;
        let size = look.size("text");
        let widths: Vec<f64> = self.choices().iter().map(|c| CIRCLE + 6.0 + fonts.measure(&c.label, "text", size, false)).collect();
        let mut places = Vec::new();
        let (mut x, mut y) = (b.x, 0.0);
        for width in widths {
            if self.vertical() {
                places.push(Rect::new(b.x, y, width.min(b.width), LINE));
                y += LINE + DOWN;
                continue;
            }
            if x > b.x && x + width > b.right() {
                x = b.x;
                y += LINE + DOWN;
            }
            places.push(Rect::new(x, y, width, LINE));
            x += width + ACROSS;
        }
        let height = places.last().map(|p: &Rect| p.bottom()).unwrap_or(0.0);
        let top = b.y + ((b.height - height) / 2.0).max(0.0);
        places.into_iter().map(|p| p.offset(0.0, top)).collect()
    }

    fn choice_at(&self, x: f64, y: f64) -> Option<usize> {
        self.places.iter().position(|p| p.contains(x, y))
    }

    /// The person picked the choice at `at`: a change when it differs from its value.
    pub fn pick(&mut self, at: usize, cx: &mut Cx) {
        let choices = self.choices();
        let Some(path) = self.base.bound().map(str::to_string) else { return };
        if !self.base.enabled() || self.base.read_only() || at >= choices.len() {
            return;
        }
        self.focus_at = Some(at);
        cx.invalidate();
        if same(&choices[at].value, &self.value()) {
            return;
        }
        self.base.host.write(self.base.name().unwrap_or(""), &path, choices[at].value.clone());
        self.base.fire("change");
    }
}

impl Element for RadioPart {
    part_element!();

    fn focusable(&self) -> bool {
        self.base.enabled() && !self.choices().is_empty()
    }

    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.base.node.bounds = area;
        self.places = self.lay_out(cx.fonts(), cx.look());
    }

    fn on_pointer_pressed(&mut self, at: PointerInput, cx: &mut Cx) {
        self.places = self.lay_out(cx.fonts(), cx.look());
        self.pressed_at = self.choice_at(at.screen_x, at.screen_y);
    }

    fn on_click(&mut self, cx: &mut Cx) {
        if let Some(at) = self.pressed_at.take() {
            self.pick(at, cx);
        }
    }

    fn on_key(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        let count = self.choices().len();
        if count == 0 {
            return false;
        }
        let now = self.focus_at.or_else(|| self.chosen()).unwrap_or(0);
        match key.key {
            Key::Right | Key::Down => self.pick((now + 1) % count, cx),
            Key::Left | Key::Up => self.pick((now + count - 1) % count, cx),
            Key::Space => self.pick(now, cx),
            _ => return false,
        }
        true
    }

    fn on_focus_changed(&mut self, focused: bool, cx: &mut Cx) {
        self.focus_at = if focused { self.chosen() } else { None };
        cx.invalidate();
    }

    fn draw(&self, p: &Painter) {
        let look = p.look;
        let _dimmed = p.dimmed(self.base.enabled() && !self.base.read_only());
        let choices = self.choices();
        let places = self.lay_out(p.fonts, look);
        let chosen = self.chosen();
        let error = self.base.state().and_then(|s| s.error).is_some_and(|e| !e.show().is_empty());
        let focused = self.base.node.focused;
        let _clip = p.clip(self.base.node.bounds);
        for (i, (choice, place)) in choices.iter().zip(places).enumerate() {
            let circle = Rect::new(place.x + 1.0, place.y + (LINE - CIRCLE) / 2.0, CIRCLE, CIRCLE);
            let on = chosen == Some(i);
            p.fill(circle, look.color("input"), CIRCLE / 2.0);
            let line = if error {
                "error"
            } else if on {
                "accent"
            } else {
                "strong_border"
            };
            p.stroke(circle, look.color(line), if on { 1.5 } else { look.border() }, CIRCLE / 2.0);
            if on {
                p.fill(circle.inset(4.0), look.color("accent"), (CIRCLE - 8.0) / 2.0);
            }
            if focused && Some(i) == self.focus_at.or(chosen).or(Some(0)) {
                p.focus_ring(circle.inset(-3.0), CIRCLE / 2.0 + 3.0);
            }
            // The label's place reaches a little past its measured width: a
            // glyph may draw past its advance (k, f); the part's own clip stays.
            let text = Rect { x: circle.right() + 6.0, width: (place.right() - circle.right() - 6.0).max(0.0) + LABEL_SLACK, ..place };
            p.text(&choice.label, text, look.color("text"), look.size("text"), "text", false, TextAlign::Left);
        }
    }
}

impl Part for RadioPart {
    fn base(&self) -> &PartBase {
        &self.base
    }
}
