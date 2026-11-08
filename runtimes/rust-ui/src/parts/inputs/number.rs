//! A number box and a spin box, the same as on every stack
//! (conformance/forms.json, formats.json): the value shows in the person's
//! format and typed text is read the same way, when the input is left or
//! Enter is pressed. Text that is no number stays as typed and is marked,
//! and the value stays; a cleared input is zero; only a number that differs
//! is a change, and what code sets is never one. Unbound, or bound to a
//! value without a format, it keeps the text as typed. A spin box has a
//! range (conformance/ranges.json): its arrows and keys move it a step, or
//! ten, and a typed number is kept within the range. Numbers read from
//! the right, so their digits line up; an arrow held keeps spinning, after
//! a pause, as a system's spin box does; the arrows take no focus.
//!
//! The part holds its text editor and hands it what the person does; the
//! part itself takes the focus.

use std::rc::Rc;
use std::time::Duration;

use clarity_runtime::ranges::{self, Range, RangeKey};
use serde_json::Value;

use crate::element::{Cx, Element};
use crate::geometry::Rect;
use crate::input::{Key, KeyInput, Modifiers, PointerInput};
use crate::markup::PartSpec;
use crate::painter::{Painter, TextAlign};
use crate::part_element;
use crate::parts::inputs::formats::{format_of, formats_of, Exact, FieldFormat, FormatKind};
use crate::parts::text_box::{InputIcon, InputSideButton};
use crate::parts::text_editor::TextEditor;
use crate::parts::{Part, PartBase, PartHost};

/// How wide a spin box's arrows are.
const ARROW_WIDTH: f64 = 22.0;

/// How long a held arrow waits before it repeats, and then between repeats.
pub const REPEAT_DELAY: Duration = Duration::from_millis(300);
pub const REPEAT_INTERVAL: Duration = Duration::from_millis(100);

/// Which of a spin box's arrows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Arrow {
    Up,
    Down,
}

pub struct NumberPart {
    base: PartBase,
    editor: TextEditor,
    /// What an unbound part without a name keeps.
    own: String,
    /// The value as it was last shown; none before the first.
    shown: Option<String>,
    /// What was typed while it is no number; none when the input shows its value.
    bad: Option<String>,
    up: Option<InputSideButton>,
    down: Option<InputSideButton>,
    pointer: Option<(f64, f64)>,
    pressing: Option<Arrow>,
}

pub fn make(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    Box::new(NumberPart::new(spec, host))
}

/// A value as text for a range: a number as written, text as it is; none for none.
fn range_text(value: &Option<Value>) -> Option<String> {
    match value {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) => Some(s.clone()),
        Some(other) => Some(other.to_string()),
    }
}

/// Whether two values are the same number, or the same otherwise.
fn same(a: &Value, b: &Option<Value>) -> bool {
    let b = b.clone().unwrap_or(Value::Null);
    match (Exact::of(a), Exact::of(&b)) {
        (Some(x), Some(y)) => {
            let scale = x.scale.max(y.scale);
            x.rounded(scale) == y.rounded(scale)
        }
        _ => *a == b,
    }
}

impl NumberPart {
    pub fn new(spec: PartSpec, host: Rc<dyn PartHost>) -> NumberPart {
        let base = PartBase::new(spec, host);
        let mut editor = TextEditor::new(false);
        editor.placeholder = base.spec.text("placeholder").map(|p| base.host.text(Some(p)));
        // Numbers read from the right, so their digits line up.
        editor.align = TextAlign::Right;
        editor.undoes = !base.spec.flag("noUndo");
        let spin = base.spec.kind == "spinbox";
        let (up, down) = if spin {
            editor.right_inset = ARROW_WIDTH + 2.0;
            (Some(InputSideButton::new(InputIcon::Up)), Some(InputSideButton::new(InputIcon::Down)))
        } else {
            (None, None)
        };
        let mut part = NumberPart { base, editor, own: String::new(), shown: None, bad: None, up, down, pointer: None, pressing: None };
        part.refresh();
        part.sync();
        part
    }

    /// Where the person types.
    pub fn editor(&self) -> &TextEditor {
        &self.editor
    }

    pub fn editor_mut(&mut self) -> &mut TextEditor {
        &mut self.editor
    }

    /// A spin box's arrows; none for a number box.
    pub fn up(&self) -> Option<&InputSideButton> {
        self.up.as_ref()
    }

    pub fn down(&self) -> Option<&InputSideButton> {
        self.down.as_ref()
    }

    /// What was typed while it is no number; none when the input shows its value.
    pub fn bad(&self) -> Option<&str> {
        self.bad.as_deref()
    }

    /// How its value shows and reads: the bound field's format; none keeps the text.
    pub fn format(&self) -> Option<FieldFormat> {
        self.base.bound().and_then(|path| format_of(self.base.host.as_ref(), path))
    }

    /// A spin box's range: as written, else the bound field's own; none for a number box.
    pub fn range(&self) -> Option<Range> {
        if self.base.spec.kind != "spinbox" {
            return None;
        }
        let facts = self.base.bound().and_then(|path| self.base.host.field(path));
        let written = |name: &str| self.base.spec.text(name).filter(|t| !t.is_empty()).map(str::to_string);
        // What code set (`levelBox.max = 3`) goes before what the line writes.
        let state = self.base.state();
        let set = |pick: fn(&clarity_runtime::ui::PartState) -> Option<i64>| state.as_deref().and_then(pick).map(|n| n.to_string());
        Some(Range {
            min: set(|s| s.min).or_else(|| written("min")).or_else(|| facts.as_ref().and_then(|f| f.min.clone())),
            max: set(|s| s.max).or_else(|| written("max")).or_else(|| facts.as_ref().and_then(|f| f.max.clone())),
            step: written("step").unwrap_or_else(|| "1".to_string()),
        })
    }

    /// Whether it holds whole numbers: its field is an integer or a duration; without one, its range has no decimals.
    pub fn whole(&self) -> bool {
        match self.format() {
            Some(format) => matches!(format.kind, FormatKind::Integer | FormatKind::Duration),
            None => !self.base.spec.text("step").unwrap_or("1").contains('.') && !self.base.spec.text("min").unwrap_or("").contains('.'),
        }
    }

    /// The value: its field's, or the text it keeps unbound.
    pub fn value(&self) -> Option<Value> {
        match self.base.bound() {
            Some(path) => self.base.host.read(path),
            None => Some(Value::String(self.base.state().and_then(|s| s.text.clone()).unwrap_or_else(|| self.own.clone()))),
        }
    }

    /// The value as it shows for editing.
    fn text_of(&self, value: &Option<Value>) -> String {
        match (self.format(), value) {
            (_, None) | (_, Some(Value::Null)) => String::new(),
            (Some(format), Some(v)) => formats_of(self.base.host.as_ref()).edit_text(&format, v),
            (None, Some(Value::String(s))) => s.clone(),
            (None, Some(other)) => other.to_string(),
        }
    }

    /// The editor follows what code set: enabled, read-only, marked.
    fn refresh(&mut self) {
        let state = self.base.state();
        self.editor.enabled = self.base.enabled();
        self.editor.read_only = self.base.read_only();
        self.editor.marked = self.bad.is_some() || state.and_then(|s| s.error.clone()).is_some_and(|e| !e.show().is_empty());
    }

    /// A value set from elsewhere (code, the record read again) shows as it is, and is no longer marked.
    pub fn sync(&mut self) {
        let now = self.text_of(&self.value());
        if self.shown.as_deref() == Some(now.as_str()) {
            return;
        }
        self.shown = Some(now.clone());
        self.bad = None;
        self.editor.set_text(&now);
    }

    /// Takes what is typed: a change when it is a number that differs from the value.
    pub fn take(&mut self) {
        self.sync();
        let typed = self.editor.text();
        if self.bad.is_none() && self.shown.as_deref() == Some(typed.as_str()) {
            return;
        }
        let Some(mut next) = self.read(&typed) else {
            self.bad = Some(typed);
            self.refresh();
            return;
        };
        self.bad = None;
        if let Some(range) = self.range() {
            if let Some(kept) = range_text(&Some(next.clone())).and_then(|t| ranges::typed_in(&t, &range)) {
                next = self.held(&kept);
            }
        }
        self.offer(next);
    }

    /// What is typed as a value: in the field's format (cleared is zero), or as text; none when it is no number.
    fn read(&self, typed: &str) -> Option<Value> {
        if let Some(format) = self.format() {
            if typed.trim().is_empty() {
                return Some(if self.whole() { Value::from(0) } else { Value::String("0".to_string()) });
            }
            return formats_of(self.base.host.as_ref()).read(&format, typed);
        }
        if self.range().is_none() {
            return Some(Value::String(typed.to_string()));
        }
        if typed.trim().is_empty() {
            return Some(Value::String("0".to_string()));
        }
        Exact::parse(typed.trim()).map(|_| Value::String(typed.trim().to_string()))
    }

    /// A number as the value holds it: a whole number or exact text for a field with a format, else text.
    fn held(&self, number: &str) -> Value {
        if self.format().is_some() && self.whole() {
            if let Some(n) = Exact::parse(number).and_then(|e| i64::try_from(e.rounded(0).n).ok()) {
                return Value::from(n);
            }
        }
        Value::String(number.to_string())
    }

    /// Makes `next` the value when it differs; else the input shows the value again.
    fn offer(&mut self, next: Value) {
        let now = self.value();
        if same(&next, &now) {
            self.shown = None;
            self.sync();
            return;
        }
        let name = self.base.name().map(str::to_string);
        let text = match &next {
            Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        if let Some(path) = self.base.bound().map(str::to_string) {
            self.base.host.write(name.as_deref().unwrap_or(""), &path, next);
        } else if let Some(name) = name {
            crate::parts::change_state(&*self.base.host, &name, |s| s.text = Some(text.clone()));
            self.base.host.edited(&name);
        } else {
            self.own = text;
        }
        self.sync();
        self.refresh();
        self.base.fire("change");
    }

    /// A spin box's arrow or key: a step, or ten, from the value.
    pub fn spin(&mut self, key: RangeKey) {
        let Some(range) = self.range() else { return };
        if self.base.read_only() || !self.base.enabled() {
            return;
        }
        self.bad = None;
        let now = range_text(&self.value()).filter(|t| !t.trim().is_empty());
        if let Some(next) = ranges::keyed(now.as_deref(), &range, key) {
            let held = self.held(&next);
            self.offer(held);
        }
    }

    /// The arrow at a point on the screen.
    fn arrow_at(&self, x: f64, y: f64) -> Option<Arrow> {
        if self.up.as_ref().is_some_and(|b| b.bounds.contains(x, y)) {
            return Some(Arrow::Up);
        }
        if self.down.as_ref().is_some_and(|b| b.bounds.contains(x, y)) {
            return Some(Arrow::Down);
        }
        None
    }

    /// Makes the editor ready for what the person does: their state, the value and the part's look.
    fn ready(&mut self) {
        self.sync();
        self.refresh();
        self.editor.mirror(&self.base.node);
    }
}

impl Element for NumberPart {
    part_element!();

    fn focusable(&self) -> bool {
        self.base.enabled()
    }

    fn typing(&self) -> bool {
        true
    }

    fn take_typed(&mut self, _cx: &mut Cx) -> bool {
        self.take();
        true
    }

    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.base.node.bounds = area;
        self.ready();
        self.editor.place(area, cx);
        let half = ((area.height - 4.0) / 2.0).max(0.0);
        if let Some(up) = self.up.as_mut() {
            up.bounds = Rect::new(area.right() - 2.0 - ARROW_WIDTH, area.y + 2.0, ARROW_WIDTH, half);
        }
        if let Some(down) = self.down.as_mut() {
            down.bounds = Rect::new(area.right() - 2.0 - ARROW_WIDTH, area.y + 2.0 + half, ARROW_WIDTH, half);
        }
    }

    fn draw(&self, p: &Painter) {
        self.editor.paint(p);
        let enabled = self.base.enabled() && !self.base.read_only();
        let node = &self.base.node;
        let over = if node.hovered { self.pointer.and_then(|(x, y)| self.arrow_at(x, y)) } else { None };
        for (arrow, button) in [(Arrow::Up, &self.up), (Arrow::Down, &self.down)] {
            if let Some(button) = button {
                button.draw(p, enabled, over == Some(arrow), node.pressed && self.pressing == Some(arrow) && over == Some(arrow));
            }
        }
    }

    /// An arrow pressed spins at once, and again after a pause while it is held.
    fn on_pointer_pressed(&mut self, at: PointerInput, cx: &mut Cx) {
        self.ready();
        self.pointer = Some((at.screen_x, at.screen_y));
        self.pressing = self.arrow_at(at.screen_x, at.screen_y);
        if let Some(arrow) = self.pressing {
            self.spin(if arrow == Arrow::Up { RangeKey::Up } else { RangeKey::Down });
            cx.wake(self.base.node.id(), cx.now() + REPEAT_DELAY);
            cx.invalidate();
            return;
        }
        self.editor.pressed(at, cx);
    }

    fn on_pointer_moved(&mut self, at: PointerInput, cx: &mut Cx) {
        let was = self.pointer.and_then(|(x, y)| self.arrow_at(x, y));
        self.pointer = Some((at.screen_x, at.screen_y));
        if was != self.arrow_at(at.screen_x, at.screen_y) {
            cx.invalidate();
        }
        if self.pressing.is_none() {
            self.ready();
            self.editor.dragged(at, cx);
        }
    }

    fn on_pointer_released(&mut self, _at: PointerInput, cx: &mut Cx) {
        self.editor.released();
        if self.pressing.take().is_some() {
            cx.invalidate();
        }
    }

    fn cursor_at(&self, x: f64, y: f64) -> Option<crate::input::Cursor> {
        if self.arrow_at(x, y).is_some() {
            return None;
        }
        self.editor.cursor_at(x, y)
    }

    fn context_menu_at(&self, _x: f64, _y: f64) -> Option<Vec<crate::frame::menus::MenuEntry>> {
        self.editor.menu(self.base.node.id())
    }

    fn on_pointer_cancelled(&mut self, cx: &mut Cx) {
        self.editor.released();
        self.pressing = None;
        cx.invalidate();
    }

    /// Held on an arrow: it spins again, and wakes for the next time; let go or moved off it stops.
    fn on_wake(&mut self, cx: &mut Cx) {
        self.ready();
        self.editor.run_menu_asked(cx);
        let Some(arrow) = self.pressing else { return };
        if !self.base.node.pressed || self.pointer.and_then(|(x, y)| self.arrow_at(x, y)) != Some(arrow) || !self.base.enabled() {
            return;
        }
        self.spin(if arrow == Arrow::Up { RangeKey::Up } else { RangeKey::Down });
        cx.wake(self.base.node.id(), cx.now() + REPEAT_INTERVAL);
        cx.invalidate();
    }

    fn on_wheel(&mut self, _dx: f64, dy: f64, _held: Modifiers, cx: &mut Cx) -> bool {
        self.editor.wheel(dy, cx)
    }

    fn on_focus_changed(&mut self, focused: bool, cx: &mut Cx) {
        self.editor.focus_moved(focused, cx);
        if focused {
            self.base.fire("enter");
        } else {
            self.take();
            self.base.fire("leave");
        }
    }

    fn on_key(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        self.ready();
        // Enter takes the number; the form it is in, or the window's default button, does the rest.
        if key.key == Key::Enter && key.modifiers.is_empty() {
            self.take();
            cx.invalidate();
            return false;
        }
        // Up and Down, Page Up and Page Down spin a spin box; Left and Right move the caret.
        if self.range().is_some() && key.modifiers.is_empty() {
            let spin = match key.key {
                Key::Up => Some(RangeKey::Up),
                Key::Down => Some(RangeKey::Down),
                Key::PageUp => Some(RangeKey::PageUp),
                Key::PageDown => Some(RangeKey::PageDown),
                _ => None,
            };
            if let Some(spin) = spin {
                self.spin(spin);
                cx.invalidate();
                return true;
            }
        }
        self.editor.key(key, cx)
    }

    fn on_text(&mut self, text: &str, cx: &mut Cx) {
        self.ready();
        self.editor.typed(text, cx);
    }
}

impl Part for NumberPart {
    fn base(&self) -> &PartBase {
        &self.base
    }
}
