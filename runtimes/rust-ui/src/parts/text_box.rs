//! A text box, a memo and a password, the same as on every stack
//! (conformance/forms.json). Typed text is taken when the input is left or
//! Enter is pressed (a memo: only when it is left, Enter is a new line); only
//! text that differs from the value is a change, and what code sets is never
//! one. A bound input writes its field; an unbound named one keeps its text
//! in its state. A password has an eye that shows what is typed or hides it
//! again, and a button that copies it, named in Clarity's own words.
//!
//! The part holds its text editor and hands it what the person does; the
//! part itself takes the focus.

use std::rc::Rc;

use serde_json::Value;
use skia_safe::{Paint, PaintStyle, PaintCap};

use crate::element::{Cx, Element};
use crate::geometry::Rect;
use crate::input::{Key, KeyInput, Modifiers, PointerInput};
use crate::locale;
use crate::markup::PartSpec;
use crate::painter::Painter;
use crate::part_element;
use crate::parts::text_editor::TextEditor;
use crate::parts::{Part, PartBase, PartHost};

/// How wide each of a password's buttons is.
const BUTTON_WIDTH: f64 = 28.0;

/// The icons of the small buttons inside an input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputIcon {
    Eye,
    EyeShut,
    Copy,
    Up,
    Down,
    /// Three dots: a folder part's button that opens the system's picker.
    More,
}

/// A small flat button inside an input's right edge: a password's eye and
/// copy, later a spin box's arrows. The part that has it draws it and says
/// what a press on it does.
#[derive(Debug, Clone, PartialEq)]
pub struct InputSideButton {
    pub icon: InputIcon,
    /// What it does, in the person's language; it says so when the pointer rests on it.
    pub name: String,
    /// Where it is, on the screen.
    pub bounds: Rect,
}

impl InputSideButton {
    pub fn new(icon: InputIcon) -> InputSideButton {
        InputSideButton { icon, name: String::new(), bounds: Rect::default() }
    }

    /// Draws it, with the look of the pointer over it or pressing it.
    pub fn draw(&self, p: &Painter, enabled: bool, hovered: bool, pressed: bool) {
        let look = p.look;
        let b = self.bounds;
        if enabled && (hovered || pressed) {
            p.fill(b, look.color(if pressed { "accent_soft" } else { "raised" }), look.radius("input"));
        }
        let mut pen = Paint::default();
        pen.set_color(look.color(if enabled { "text" } else { "muted" }));
        pen.set_anti_alias(true);
        pen.set_style(PaintStyle::Stroke);
        pen.set_stroke_width(1.6);
        pen.set_stroke_cap(PaintCap::Round);
        let canvas = p.canvas;
        let cx = (b.x + b.width / 2.0) as f32;
        let cy = (b.y + b.height / 2.0) as f32;
        match self.icon {
            InputIcon::Eye | InputIcon::EyeShut => {
                canvas.draw_oval(skia_safe::Rect::new(cx - 8.0, cy - 5.0, cx + 8.0, cy + 5.0), &pen);
                canvas.draw_circle((cx, cy), 2.5, &pen);
                if self.icon == InputIcon::EyeShut {
                    canvas.draw_line((cx - 7.0, cy - 7.0), (cx + 7.0, cy + 7.0), &pen);
                }
            }
            InputIcon::Copy => {
                canvas.draw_round_rect(skia_safe::Rect::new(cx - 3.0, cy - 3.0, cx + 6.0, cy + 6.0), 1.5, 1.5, &pen);
                canvas.draw_line((cx - 6.0, cy + 3.0), (cx - 6.0, cy - 6.0), &pen);
                canvas.draw_line((cx - 6.0, cy - 6.0), (cx + 3.0, cy - 6.0), &pen);
            }
            InputIcon::Up => {
                canvas.draw_line((cx - 4.0, cy + 2.0), (cx, cy - 2.0), &pen);
                canvas.draw_line((cx, cy - 2.0), (cx + 4.0, cy + 2.0), &pen);
            }
            InputIcon::Down => {
                canvas.draw_line((cx - 4.0, cy - 2.0), (cx, cy + 2.0), &pen);
                canvas.draw_line((cx, cy + 2.0), (cx + 4.0, cy - 2.0), &pen);
            }
            InputIcon::More => {
                let mut dot = pen.clone();
                dot.set_style(PaintStyle::Fill);
                for i in -1..=1 {
                    canvas.draw_circle((cx + i as f32 * 5.0, cy), 1.5, &dot);
                }
            }
        }
    }
}

/// Which of a password's buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    Eye,
    Copy,
}

pub struct TextBoxPart {
    base: PartBase,
    editor: TextEditor,
    /// What an unbound part without a name keeps.
    own: String,
    /// The value as it was last shown; none before the first.
    shown: Option<String>,
    eye: Option<InputSideButton>,
    copy: Option<InputSideButton>,
    /// Where the pointer is over it, on the screen; none when it is elsewhere.
    pointer: Option<(f64, f64)>,
    /// The button a press began on.
    pressing: Option<Side>,
}

pub fn make(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    Box::new(TextBoxPart::new(spec, host))
}

/// A value as an input shows it: nothing for none, a text as it is, anything else as written.
fn value_text(value: Option<Value>) -> String {
    match value {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(s)) => s,
        Some(other) => other.to_string(),
    }
}

impl TextBoxPart {
    pub fn new(spec: PartSpec, host: Rc<dyn PartHost>) -> TextBoxPart {
        let base = PartBase::new(spec, host);
        let mut editor = TextEditor::new(base.spec.kind == "memo");
        editor.placeholder = base.spec.text("placeholder").map(|p| base.host.text(Some(p)));
        // Ctrl+Z takes back its typing, unless the part has `no undo`.
        editor.undoes = !base.spec.flag("noUndo");
        let password = base.spec.kind == "password";
        editor.set_password(password);
        // A memo of code writes in the mono font, its words colored by the language.
        if let Some(language) = base.spec.text("code").and_then(crate::parts::code_colors::language) {
            editor.language = Some(language);
            editor.font_role = "mono";
        }
        let (eye, copy) = if password {
            editor.right_inset = 2.0 * BUTTON_WIDTH + 2.0;
            let mut copy = InputSideButton::new(InputIcon::Copy);
            copy.name = locale::translate("copySecret", "Copy");
            (Some(InputSideButton::new(InputIcon::Eye)), Some(copy))
        } else if base.spec.flag("copy") {
            // `copy`: a button in its corner puts its text on the clipboard, read only too.
            editor.right_inset = BUTTON_WIDTH + 2.0;
            let mut copy = InputSideButton::new(InputIcon::Copy);
            copy.name = locale::translate("copyText", "Copy");
            (None, Some(copy))
        } else {
            (None, None)
        };
        let mut part = TextBoxPart { base, editor, own: String::new(), shown: None, eye, copy, pointer: None, pressing: None };
        part.named();
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

    /// A password's eye, which shows what is typed or hides it again.
    pub fn eye(&self) -> Option<&InputSideButton> {
        self.eye.as_ref()
    }

    /// A password's copy button.
    pub fn copy(&self) -> Option<&InputSideButton> {
        self.copy.as_ref()
    }

    /// The value it shows: its field, or the text it keeps unbound.
    pub fn value(&self) -> String {
        match self.base.bound() {
            Some(path) => value_text(self.base.host.read(path)),
            None => self.base.state().and_then(|s| s.text.clone()).unwrap_or_else(|| self.own.clone()),
        }
    }

    /// The eye says what it does next.
    fn named(&mut self) {
        let hidden = self.editor.password();
        if let Some(eye) = self.eye.as_mut() {
            eye.icon = if hidden { InputIcon::Eye } else { InputIcon::EyeShut };
            eye.name = if hidden { locale::translate("showSecret", "Show") } else { locale::translate("hideSecret", "Hide") };
        }
    }

    /// The eye: the password shows as typed, or hides again.
    pub fn reveal(&mut self) {
        let hidden = self.editor.password();
        self.editor.set_password(!hidden);
        self.named();
    }

    /// The copy button: what is in the input goes on the clipboard, typed or not yet taken.
    pub fn copy_out(&self, cx: &mut Cx) {
        let text = self.editor.text();
        cx.clipboard().set_text(&text);
    }

    /// The editor follows what code set: enabled, read-only, marked with an error.
    fn refresh(&mut self) {
        let state = self.base.state();
        self.editor.enabled = self.base.enabled();
        self.editor.read_only = self.base.read_only();
        self.editor.marked = state.and_then(|s| s.error.clone()).is_some_and(|e| !e.show().is_empty());
    }

    /// A value set from elsewhere (code, the record read again) shows as it is.
    pub fn sync(&mut self) {
        let now = self.value();
        if self.shown.as_deref() == Some(now.as_str()) {
            return;
        }
        self.editor.set_text(&now);
        self.shown = Some(now);
    }

    /// Takes what is typed: a change when it differs from the value.
    pub fn take(&mut self) {
        self.sync();
        let typed = self.editor.text();
        if self.shown.as_deref() == Some(typed.as_str()) {
            return;
        }
        self.shown = Some(typed.clone());
        let name = self.base.name().map(str::to_string);
        if let Some(path) = self.base.bound().map(str::to_string) {
            self.base.host.write(name.as_deref().unwrap_or(""), &path, Value::String(typed));
        } else if let Some(name) = name {
            crate::parts::change_state(&*self.base.host, &name, |s| s.text = Some(typed.clone()));
            self.base.host.edited(&name);
        } else {
            self.own = typed;
        }
        self.base.fire("change");
    }

    /// The button of a password at a point on the screen.
    fn side_at(&self, x: f64, y: f64) -> Option<Side> {
        if self.eye.as_ref().is_some_and(|b| b.bounds.contains(x, y)) {
            return Some(Side::Eye);
        }
        if self.copy.as_ref().is_some_and(|b| b.bounds.contains(x, y)) {
            return Some(Side::Copy);
        }
        None
    }

    /// Makes the editor ready for what the person does: their state and the part's look.
    fn ready(&mut self) {
        self.refresh();
        self.editor.mirror(&self.base.node);
    }
}

impl Element for TextBoxPart {
    part_element!();

    fn focusable(&self) -> bool {
        self.base.enabled()
    }

    fn typing(&self) -> bool {
        true
    }

    fn multiline(&self) -> bool {
        self.editor.is_multiline()
    }

    fn take_typed(&mut self, _cx: &mut Cx) -> bool {
        self.take();
        true
    }

    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.base.node.bounds = area;
        self.ready();
        self.sync();
        self.editor.place(area, cx);
        let height = (area.height - 4.0).max(0.0);
        if let Some(eye) = self.eye.as_mut() {
            eye.bounds = Rect::new(area.right() - 2.0 - 2.0 * BUTTON_WIDTH, area.y + 2.0, BUTTON_WIDTH, height);
        }
        // A memo's stands in its top right corner, one line high.
        let multiline = self.editor.is_multiline();
        if let Some(copy) = self.copy.as_mut() {
            let high = if multiline { height.min(crate::grid::ROW_HEIGHT - 4.0) } else { height };
            copy.bounds = Rect::new(area.right() - 2.0 - BUTTON_WIDTH, area.y + 2.0, BUTTON_WIDTH, high);
        }
    }

    fn draw(&self, p: &Painter) {
        self.editor.paint(p);
        let enabled = self.base.enabled();
        let node = &self.base.node;
        let over = if node.hovered { self.pointer.and_then(|(x, y)| self.side_at(x, y)) } else { None };
        for (side, button) in [(Side::Eye, &self.eye), (Side::Copy, &self.copy)] {
            if let Some(button) = button {
                button.draw(p, enabled, over == Some(side), node.pressed && self.pressing == Some(side) && over == Some(side));
            }
        }
    }

    fn tooltip_at(&self, x: f64, y: f64) -> Option<String> {
        match self.side_at(x, y)? {
            Side::Eye => self.eye.as_ref().map(|b| b.name.clone()),
            Side::Copy => self.copy.as_ref().map(|b| b.name.clone()),
        }
    }

    fn on_pointer_pressed(&mut self, at: PointerInput, cx: &mut Cx) {
        self.ready();
        self.pressing = self.side_at(at.screen_x, at.screen_y);
        if self.pressing.is_some() {
            cx.invalidate();
            return;
        }
        self.editor.pressed(at, cx);
    }

    fn on_pointer_moved(&mut self, at: PointerInput, cx: &mut Cx) {
        let was = self.pointer.and_then(|(x, y)| self.side_at(x, y));
        self.pointer = Some((at.screen_x, at.screen_y));
        if was != self.side_at(at.screen_x, at.screen_y) {
            cx.invalidate();
        }
        if self.pressing.is_none() {
            self.ready();
            self.editor.dragged(at, cx);
        }
    }

    fn on_pointer_released(&mut self, at: PointerInput, cx: &mut Cx) {
        self.editor.released();
        let Some(side) = self.pressing.take() else { return };
        if !self.base.enabled() || self.side_at(at.screen_x, at.screen_y) != Some(side) {
            return;
        }
        match side {
            Side::Eye => self.reveal(),
            Side::Copy => self.copy_out(cx),
        }
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
        // Enter in one line takes the text; the form it is in, or the window's default button, does the rest.
        if key.key == Key::Enter && !self.editor.is_multiline() && key.modifiers.is_empty() {
            self.take();
            cx.invalidate();
            return false;
        }
        self.editor.key(key, cx)
    }

    fn on_text(&mut self, text: &str, cx: &mut Cx) {
        self.ready();
        self.editor.typed(text, cx);
    }

    fn on_compose(&mut self, composing: Option<crate::element::Composition>, cx: &mut Cx) {
        self.editor.compose(composing, cx);
    }

    fn caret_area(&self) -> Option<crate::geometry::Rect> {
        self.editor.caret_area()
    }

    fn cursor_at(&self, x: f64, y: f64) -> Option<crate::input::Cursor> {
        if self.side_at(x, y).is_some() {
            return None;
        }
        self.editor.cursor_at(x, y)
    }

    fn context_menu_at(&self, _x: f64, _y: f64) -> Option<Vec<crate::frame::menus::MenuEntry>> {
        self.editor.menu(self.base.node.id())
    }

    fn on_wake(&mut self, cx: &mut Cx) {
        self.ready();
        self.editor.run_menu_asked(cx);
    }
}

impl Part for TextBoxPart {
    fn base(&self) -> &PartBase {
        &self.base
    }
}
