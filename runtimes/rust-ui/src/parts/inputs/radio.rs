//! The radio group (DESIGN 7.9, conformance/forms.json): one round button
//! per choice of its bound field (an enum's values, or what the host
//! offers), side by side or one under the other. Only the person picking
//! another choice is a change; what code sets is not. Each button is a Tab
//! stop of its own, which Space or Enter picks; the arrow keys pick the next
//! or the previous choice, as a system's radio group does. Side by side,
//! choices that do not fit on one line go on to the next. As the shared
//! screen draws it.

use std::cell::RefCell;
use std::rc::Rc;

use serde_json::Value;

use crate::element::{Cx, Element, ElementId, Node};
use crate::fonts::Fonts;
use crate::geometry::Rect;
use crate::input::{Key, KeyInput};
use crate::markup::PartSpec;
use crate::painter::{Painter, TextAlign};
use crate::part_element;
use crate::parts::inputs::choices::{chosen, offered_for, same};
use crate::parts::inputs::file::Facts;
use crate::parts::{Choice, Part, PartBase, PartHost};
use crate::theme::ThemeLook;

/// How big a button's circle is.
pub const CIRCLE: f64 = 16.0;

/// How far a label may draw past its measured width.
const LABEL_SLACK: f64 = 4.0;
/// How high a line of choices is.
pub const LINE: f64 = 24.0;
/// The room between choices side by side, and between lines.
pub const ACROSS: f64 = 12.0;
pub const DOWN: f64 = 4.0;

/// What the group and its buttons share: its spec and host, and its buttons by their ids.
struct Group {
    facts: Facts,
    buttons: RefCell<Vec<ElementId>>,
}

impl Group {
    fn choices(&self) -> Vec<Choice> {
        offered_for(self.facts.host.as_ref(), self.facts.bound())
    }

    fn value(&self) -> Value {
        self.facts.bound().and_then(|b| self.facts.host.read(b)).unwrap_or(Value::Null)
    }

    /// The person picked the choice at `at`: a change when it differs from its value.
    fn pick(&self, at: usize, cx: &mut Cx) {
        let choices = self.choices();
        if !self.facts.enabled() || self.facts.read_only() || self.facts.bound().is_none() || at >= choices.len() {
            return;
        }
        // The focus follows the pick, as the arrow keys move it.
        let buttons = self.buttons.borrow().clone();
        if cx.focused().is_some_and(|f| buttons.contains(&f)) {
            if let Some(id) = buttons.get(at) {
                cx.focus(Some(*id));
            }
        }
        cx.invalidate();
        if same(&choices[at].value, &self.value()) {
            return;
        }
        self.facts.write(choices[at].value.clone());
        self.facts.fire("change");
    }

    /// A key on the button of choice `at`: the arrows pick the next or the previous choice, Space or Enter this one.
    fn keyed(&self, at: usize, key: KeyInput, cx: &mut Cx) -> bool {
        let count = self.choices().len();
        if count == 0 || key.control() || key.alt() {
            return false;
        }
        match key.key {
            Key::Right | Key::Down => self.pick((at + 1) % count, cx),
            Key::Left | Key::Up => self.pick((at + count - 1) % count, cx),
            Key::Space | Key::Enter => self.pick(at, cx),
            _ => return false,
        }
        true
    }
}

/// One button of a radio group: a Tab stop of its own; a click or Space or Enter picks its choice. The group draws it.
pub struct RadioChoice {
    node: Node,
    index: usize,
    group: Rc<Group>,
}

impl RadioChoice {
    /// Which choice of the group it is.
    pub fn index(&self) -> usize {
        self.index
    }
}

impl Element for RadioChoice {
    fn node(&self) -> &Node {
        &self.node
    }

    fn node_mut(&mut self) -> &mut Node {
        &mut self.node
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn focusable(&self) -> bool {
        self.group.facts.enabled() && !self.group.facts.read_only()
    }

    fn on_click(&mut self, cx: &mut Cx) {
        self.group.pick(self.index, cx);
    }

    fn on_key(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        self.group.keyed(self.index, key, cx)
    }

    fn on_focus_changed(&mut self, _focused: bool, cx: &mut Cx) {
        cx.invalidate();
    }

    // The group draws its buttons, so a choice's focus ring and its circle stay one picture.
    fn draw(&self, _p: &Painter) {}
}

pub struct RadioPart {
    base: PartBase,
    group: Rc<Group>,
    /// Where each choice is, as it was last laid out.
    places: Vec<Rect>,
}

pub fn make(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    Box::new(RadioPart::new(spec, host))
}

impl RadioPart {
    pub fn new(spec: PartSpec, host: Rc<dyn PartHost>) -> RadioPart {
        let base = PartBase::new(spec, host);
        let group = Rc::new(Group { facts: Facts { host: base.host.clone(), spec: base.spec.clone(), part: base.node.id() }, buttons: RefCell::new(Vec::new()) });
        let mut part = RadioPart { base, group, places: Vec::new() };
        part.rebuild(None);
        part
    }

    /// Whether its choices stand one under the other.
    pub fn vertical(&self) -> bool {
        self.base.spec.text("orientation") == Some("vertical")
    }

    pub fn choices(&self) -> Vec<Choice> {
        self.group.choices()
    }

    pub fn value(&self) -> Value {
        self.group.value()
    }

    /// Where its value is among its choices; none when it is none of them.
    pub fn chosen(&self) -> Option<usize> {
        chosen(&self.choices(), &self.value())
    }

    /// One element per choice, each a Tab stop of its own: their ids.
    pub fn buttons(&self) -> Vec<ElementId> {
        self.group.buttons.borrow().clone()
    }

    /// Where each choice is (its circle and its label), as it was last laid out.
    pub fn places(&self) -> &[Rect] {
        &self.places
    }

    /// The choice whose button has the focus; none for none.
    fn focus_at(&self) -> Option<usize> {
        self.base.node.children().iter().position(|b| b.node().focused)
    }

    /// A button per choice, made again when the number of choices changed; the focus stays on a button.
    fn rebuild(&mut self, cx: Option<&mut Cx>) {
        let count = self.choices().len();
        if count == self.base.node.children().len() {
            return;
        }
        let focused = self.focus_at();
        let mut cx = cx;
        match cx.as_deref_mut() {
            Some(cx) => self.base.node.clear(cx),
            None => self.base.node.children_mut().clear(),
        }
        let mut ids = Vec::new();
        for index in 0..count {
            let button = RadioChoice { node: Node::new(), index, group: self.group.clone() };
            ids.push(button.node.id());
            self.base.node.add(Box::new(button));
        }
        *self.group.buttons.borrow_mut() = ids.clone();
        if let (Some(at), Some(cx)) = (focused, cx) {
            if count > 0 {
                cx.focus(Some(ids[at.min(count - 1)]));
            }
        }
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

    /// The person picked the choice at `at`: a change when it differs from its value.
    pub fn pick(&mut self, at: usize, cx: &mut Cx) {
        self.group.pick(at, cx);
    }
}

impl Element for RadioPart {
    part_element!();

    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.base.node.bounds = area;
        self.rebuild(Some(cx));
        self.places = self.lay_out(cx.fonts(), cx.look());
        let places = self.places.clone();
        for (button, place) in self.base.node.children_mut().iter_mut().zip(places) {
            button.arrange(place, cx);
        }
    }

    fn draw(&self, p: &Painter) {
        let look = p.look;
        let _dimmed = p.dimmed(self.base.enabled() && !self.base.read_only());
        let choices = self.choices();
        let places = self.lay_out(p.fonts, look);
        let chosen = self.chosen();
        let focus_at = self.focus_at();
        let error = self.base.state().and_then(|s| s.error.clone()).is_some_and(|e| !e.show().is_empty());
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
            if focus_at == Some(i) {
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
