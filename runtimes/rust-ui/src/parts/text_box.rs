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
    /// The caret and selected text it last put in its values for code to read
    /// (Writer gap 21); a caret there that differs is one code set.
    published: Option<(usize, String)>,
    /// A memo's toolbox (`toolbox WritingTools`): its page's coded tools, over the window while it may be written in.
    toolbox: Rc<crate::parts::data::canvas::toolbox::Toolbox>,
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
        // A memo of Markdown keeps the text font, its Markdown colored, Ctrl+B and Ctrl+I marking it.
        if base.spec.flag("markdown") {
            editor.language = crate::parts::code_colors::language("markdown");
            editor.markdown = true;
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
        let mut part = TextBoxPart { base, editor, own: String::new(), shown: None, eye, copy, pointer: None, pressing: None, published: None, toolbox: Rc::default() };
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

    /// What code told it since it last heard: text put in at the caret.
    fn hear_all(&mut self, cx: &mut Cx) {
        let Some(name) = self.base.name().map(str::to_string) else { return };
        let mut put = false;
        for hear in self.base.host.hears(&name) {
            if hear.hear == "insert"
                && let Some(Value::String(text)) = hear.values.first()
            {
                self.editor.put(text, cx);
                put = true;
            }
        }
        if put {
            self.commit();
        }
    }

    /// What is typed goes to its field, as `take` does, but without `change`: code put it there.
    fn commit(&mut self) {
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
        } else {
            self.own = typed;
        }
    }

    /// A caret code set (`manuscriptBox.caret = 12`) moves there, selecting nothing.
    fn follow_code(&mut self, cx: &mut Cx) {
        let Some(name) = self.base.name() else { return };
        let Some((caret, _)) = &self.published else { return };
        let set = self.base.host.state(Some(name)).and_then(|s| s.values).and_then(|v| v.get("caret").and_then(Value::as_u64));
        if let Some(set) = set.map(|c| c as usize)
            && set != *caret
        {
            let place = set.min(self.editor.text().chars().count());
            self.editor.select(place, place);
            cx.invalidate();
        }
    }

    /// Its caret and selected text, in its values for code to read: set when they changed.
    fn publish(&mut self) {
        let Some(name) = self.base.name().map(str::to_string) else { return };
        let now = (self.editor.caret(), self.editor.selected_text());
        if self.published.as_ref() == Some(&now) {
            return;
        }
        let mut state = self.base.host.state(Some(&name)).unwrap_or_default();
        let values = state.values.get_or_insert_with(serde_json::Map::new);
        values.insert("caret".into(), Value::from(now.0 as u64));
        values.insert("selectedText".into(), Value::String(now.1.clone()));
        self.base.host.set_state(&name, state);
        self.published = Some(now);
    }

    /// Whether its toolbox comes at the caret (`toolbox WritingTools`, a memo's unless written to float).
    fn toolbox_at_caret(&self) -> bool {
        !self.base.spec.children.is_empty() && self.base.spec.text("toolbox") == Some("caret")
    }

    /// Its toolbox at the caret's place on the screen, or at a point: the page's
    /// parts with a row of Cut, Copy and Paste under them.
    pub fn open_toolbox_at(&mut self, at: (f64, f64), cx: &mut Cx) {
        let Some(name) = self.base.name().map(str::to_string) else { return };
        let mut parts = self.base.spec.children.clone();
        let below = parts.iter().map(|p| p.place(false)).map(|g| i64::from(g.row) + i64::from(g.height)).max().unwrap_or(1);
        let row = format!(r#"<Window name="Edit"><Tool does="cut" at="1,{below}" /><Tool does="copy" at="2,{below}" /><Tool does="paste" at="3,{below}" /></Window>"#);
        parts.extend(crate::markup::read(&row).map(|w| w.parts).unwrap_or_default());
        let columns = self.base.spec.number("toolboxColumns").map_or(4, |c| c as i64).max(3);
        self.toolbox.open_at(at, self.base.node.bounds, &parts, columns, (self.base.host.clone(), name), cx);
    }

    /// Whether its toolbox at the caret shows now.
    pub fn toolbox_open(&self) -> bool {
        self.toolbox.opened().is_some()
    }

    /// A memo with a toolbox page: a floating one shows while the memo may be
    /// written in; one at the caret goes once a tool did its action, and with the memo.
    fn follow_toolbox(&mut self, cx: &mut Cx) {
        use crate::parts::data::canvas::toolbox::{Ask, Following, Shows};
        if self.base.spec.children.is_empty() {
            return;
        }
        let Some(name) = self.base.name().map(str::to_string) else { return };
        let writes = self.base.enabled() && !self.base.read_only() && self.base.node.bounds.width > 0.0;
        if self.toolbox_at_caret() {
            if let Some(Ask::Text(ask)) = self.toolbox.take_ask() {
                self.editor.clipboard_action(ask, cx);
                self.commit();
            }
            if self.toolbox.take_acted() || !writes {
                self.toolbox.close(cx);
            }
            return;
        }
        let shows = match self.base.spec.text("toolbox") {
            Some("slideout") => Shows::Slideout,
            _ => Shows::Popup,
        };
        let following = Following {
            shows: writes.then_some(shows),
            paints: false,
            canvas: self.base.node.bounds,
            link: (self.base.host.clone(), name),
            parts: self.base.spec.children.clone(),
            columns: self.base.spec.number("toolboxColumns").map_or(4, |c| c as i64),
            written: (2.0, "text".to_string()),
        };
        self.toolbox.follow(following, cx);
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

    /// Leaving the screen, its toolbox goes with it.
    fn on_detached(&mut self, cx: &mut Cx) {
        self.toolbox.close(cx);
    }

    /// With a toolbox at the caret it takes the right button itself, so a right-click calls up the toolbox.
    fn right_drags(&self, _x: f64, _y: f64) -> bool {
        self.toolbox_at_caret() && self.base.enabled() && !self.base.read_only()
    }

    /// A right-click with a toolbox at the caret: the caret goes there and the toolbox comes, in place of the menu.
    fn on_right_released(&mut self, at: PointerInput, cx: &mut Cx) -> bool {
        if !self.toolbox_at_caret() || !self.base.enabled() || self.base.read_only() {
            return false;
        }
        let b = crate::element::Element::node(&self.editor).bounds;
        self.editor.caret_to(at.screen_x - b.x, at.screen_y - b.y, cx);
        self.publish();
        self.open_toolbox_at((at.screen_x, at.screen_y + 2.0), cx);
        true
    }

    /// A row dragged from a part it takes rows from (`on drop from fragmentList`):
    /// the caret goes where it was let go, then the handler runs and finds it there.
    fn take_drop(&mut self, from: &str, x: f64, y: f64, cx: &mut Cx) -> bool {
        let Some(handler) = crate::parts::data::rows::drop_handler(&self.base, from) else { return false };
        self.take();
        // The point comes on the screen; the editor places from its own top left.
        let b = crate::element::Element::node(&self.editor).bounds;
        self.editor.caret_to(x - b.x, y - b.y, cx);
        self.publish();
        self.base.host.run(self.base.name(), &crate::markup::HandlerCall::new("drop", &handler, Vec::new()));
        cx.invalidate();
        true
    }

    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.base.node.bounds = area;
        self.ready();
        self.sync();
        self.editor.place(area, cx);
        // Where the person types, for code (Writer gap 21): what code told and set, then where it is now.
        if !self.editor.password() {
            self.hear_all(cx);
            self.follow_code(cx);
            self.publish();
        }
        self.follow_toolbox(cx);
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
        // The right button calls up its toolbox when it is let go.
        if at.button == crate::input::PointerButton::Right {
            return;
        }
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
        // Ctrl+Space calls up its toolbox at the caret.
        if key.key == Key::Space && key.control() && self.toolbox_at_caret() && self.base.enabled() && !self.base.read_only() {
            let caret = self.editor.caret_area().unwrap_or(self.base.node.bounds);
            self.open_toolbox_at((caret.x, caret.bottom() + 2.0), cx);
            return true;
        }
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

    /// A memo is a writing area, as a markdown part is a document: it takes the height the window has left, as a table does.
    fn grows(&self) -> bool {
        self.base.spec.kind == "memo" || self.base.spec.text("code").is_some()
    }
}
