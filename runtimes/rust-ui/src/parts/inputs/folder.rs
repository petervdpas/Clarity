//! A folder part (X2), the same as on every stack: the path of a folder
//! setting, where a file store keeps its files. It is typed like any text,
//! taken when the input is left or Enter is pressed, and only a text that
//! differs from the value is a change; or it is chosen with the system's
//! folder picker through the "..." button inside its right edge, which says
//! what it does in Clarity's own words when the pointer rests on it and
//! takes no Tab stop, and a chosen folder is taken at once. Where nothing
//! picks a folder (a browser) and on a read-only one, there is no button;
//! where nothing picks one, a part without a placeholder of its own says
//! in Clarity's words that the folder's path on the server is typed. A
//! bound one writes its field; an unbound named one keeps its text in its
//! state.
//!
//! The part takes the focus and holds its text editor, as a text box does,
//! and its button, which it draws and hands the presses on it. The picker
//! waits for the person, so it is asked off the screen's thread and the
//! folder comes back in `on_later`.

use std::rc::Rc;

use serde_json::Value;

use crate::element::{Cx, Element};
use crate::geometry::Rect;
use crate::input::{Key, KeyInput, Modifiers, PointerInput};
use crate::later::Done;
use crate::locale;
use crate::markup::PartSpec;
use crate::painter::Painter;
use crate::part_element;
use crate::parts::text_box::{InputIcon, InputSideButton};
use crate::parts::text_editor::TextEditor;
use crate::parts::{Part, PartBase, PartHost};

/// How wide the button is.
pub const BUTTON_WIDTH: f64 = 28.0;

/// What the button does, in Clarity's own words: its name and the title of the picker.
pub fn choose_name() -> String {
    locale::translate("chooseFolder", "Choose a folder")
}

/// What a folder part says where nothing picks a folder (a browser, whose program and folders are on the server).
pub fn type_name() -> String {
    locale::translate("typeServerFolder", "Type the path of a folder on the server")
}

/// The folder the picker gave.
struct Picked(Option<String>);

/// The small "..." button inside the box's right edge (an input's side
/// button with the More icon): no Tab stop; it says what it does when the
/// pointer rests on it. The part draws it.
#[derive(Debug, Clone, PartialEq)]
pub struct ChooseButton {
    side: InputSideButton,
    shown: bool,
}

impl std::ops::Deref for ChooseButton {
    type Target = InputSideButton;

    fn deref(&self) -> &InputSideButton {
        &self.side
    }
}

impl ChooseButton {
    pub fn is_shown(&self) -> bool {
        self.shown
    }

    /// It takes no focus: the part's box keeps it.
    pub fn focusable(&self) -> bool {
        false
    }

    /// What it says when the pointer rests at a point on it.
    pub fn tooltip_at(&self, x: f64, y: f64) -> Option<String> {
        (self.shown && self.side.bounds.contains(x, y) && !self.side.name.is_empty()).then(|| self.side.name.clone())
    }
}

pub struct FolderPart {
    base: PartBase,
    editor: TextEditor,
    button: ChooseButton,
    /// The placeholder its markup gives.
    placeholder: Option<String>,
    own: String,
    shown: Option<String>,
    /// A press began on the button.
    pressing_button: bool,
    pointer: Option<(f64, f64)>,
}

pub fn make(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    Box::new(FolderPart::new(spec, host))
}

impl FolderPart {
    pub fn new(spec: PartSpec, host: Rc<dyn PartHost>) -> FolderPart {
        let base = PartBase::new(spec, host);
        let mut editor = TextEditor::new(false);
        let placeholder = base.spec.text("placeholder").map(|p| base.host.text(Some(p)));
        editor.placeholder = placeholder.clone();
        editor.undoes = !base.spec.flag("noUndo");
        let mut side = InputSideButton::new(InputIcon::More);
        side.name = choose_name();
        let button = ChooseButton { side, shown: false };
        let mut part = FolderPart { base, editor, button, placeholder, own: String::new(), shown: None, pressing_button: false, pointer: None };
        part.sync();
        part
    }

    /// Where the person types the path.
    pub fn editor(&self) -> &TextEditor {
        &self.editor
    }

    pub fn editor_mut(&mut self) -> &mut TextEditor {
        &mut self.editor
    }

    /// The button that opens the system's folder picker, as it is now: shown or not by what the part and the system allow.
    pub fn choose_button(&self) -> ChooseButton {
        ChooseButton { side: self.button.side.clone(), shown: self.button_shows() }
    }

    /// Whether the button shows: not on a read-only one, nor where nothing picks a folder.
    fn button_shows(&self) -> bool {
        !self.base.read_only() && crate::files::system().picks_folders()
    }

    /// Asks the system's folder picker, off the screen's thread.
    fn choose(&mut self, cx: &mut Cx) {
        if self.base.read_only() || !self.base.enabled() {
            return;
        }
        let picker = crate::files::system();
        let title = choose_name();
        cx.later(self.base.node.id(), move || Picked(picker.choose_folder(&title)));
    }

    /// The path it shows: its field, or the text it keeps unbound.
    pub fn value(&self) -> String {
        match self.base.bound() {
            Some(path) => match self.base.host.read(path) {
                None | Some(Value::Null) => String::new(),
                Some(Value::String(s)) => s,
                Some(other) => other.to_string(),
            },
            None => self.base.state().and_then(|s| s.text).unwrap_or_else(|| self.own.clone()),
        }
    }

    /// A value set from elsewhere (code, the settings read again) shows as it is.
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

    /// Takes a folder the picker gave: a change when it differs from the value.
    pub fn chosen(&mut self, path: &str) {
        if self.base.read_only() || !self.base.enabled() {
            return;
        }
        self.editor.set_text(path);
        self.take();
    }

    fn ready(&mut self) {
        let state = self.base.state();
        self.editor.enabled = self.base.enabled();
        self.editor.read_only = self.base.read_only();
        self.editor.marked = state.and_then(|s| s.error).is_some_and(|e| !e.show().is_empty());
        let typed_only = !self.base.read_only() && !crate::files::system().picks_folders();
        self.editor.placeholder = self.placeholder.clone().or_else(|| typed_only.then(type_name));
        self.editor.mirror(&self.base.node);
    }
}

impl Element for FolderPart {
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

    /// The input over its whole place, with the button inside its right edge when it shows; the typed path stops before it.
    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.base.node.bounds = area;
        self.ready();
        self.sync();
        let shows = self.button_shows();
        self.button.shown = shows;
        self.editor.right_inset = if shows { BUTTON_WIDTH + 2.0 } else { 0.0 };
        self.editor.place(area, cx);
        self.button.side.bounds = if shows { Rect::new(area.right() - 2.0 - BUTTON_WIDTH, area.y + 2.0, BUTTON_WIDTH, (area.height - 4.0).max(0.0)) } else { Rect::new(area.right(), area.y, 0.0, area.height) };
    }

    fn draw(&self, p: &Painter) {
        self.editor.paint(p);
        if self.button.shown {
            let node = &self.base.node;
            let over = node.hovered && self.pointer.is_some_and(|(x, y)| self.button.bounds.contains(x, y));
            self.button.side.draw(p, self.base.enabled(), over, node.pressed && self.pressing_button && over);
        }
    }

    fn tooltip_at(&self, x: f64, y: f64) -> Option<String> {
        self.button.tooltip_at(x, y)
    }

    fn cursor_at(&self, x: f64, y: f64) -> Option<crate::input::Cursor> {
        if self.button.shown && self.button.bounds.contains(x, y) {
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

    fn on_pointer_pressed(&mut self, at: PointerInput, cx: &mut Cx) {
        self.ready();
        self.pointer = Some((at.screen_x, at.screen_y));
        self.pressing_button = self.button.shown && self.button.bounds.contains(at.screen_x, at.screen_y);
        if self.pressing_button {
            cx.invalidate();
            return;
        }
        self.editor.pressed(at, cx);
    }

    fn on_pointer_moved(&mut self, at: PointerInput, cx: &mut Cx) {
        let was = self.pointer.is_some_and(|(x, y)| self.button.bounds.contains(x, y));
        self.pointer = Some((at.screen_x, at.screen_y));
        if was != self.button.bounds.contains(at.screen_x, at.screen_y) {
            cx.invalidate();
        }
        if !self.pressing_button {
            self.ready();
            self.editor.dragged(at, cx);
        }
    }

    fn on_pointer_released(&mut self, at: PointerInput, cx: &mut Cx) {
        self.editor.released();
        if std::mem::take(&mut self.pressing_button) && self.button.bounds.contains(at.screen_x, at.screen_y) {
            self.choose(cx);
            cx.invalidate();
        }
    }

    fn on_pointer_cancelled(&mut self, cx: &mut Cx) {
        self.editor.released();
        self.pressing_button = false;
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
        if key.key == Key::Enter && key.modifiers.is_empty() {
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

    fn on_later(&mut self, done: Done, cx: &mut Cx) {
        if let Ok(picked) = done.downcast::<Picked>() {
            if let Some(folder) = picked.0.filter(|f| !f.is_empty()) {
                self.chosen(&folder);
            }
        }
        cx.invalidate();
    }
}

impl Part for FolderPart {
    fn base(&self) -> &PartBase {
        &self.base
    }
}
