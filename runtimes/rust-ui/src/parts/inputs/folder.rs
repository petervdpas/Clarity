//! A folder part (X2), the same as on every stack: the path of a folder
//! setting, where a file store keeps its files. It is typed like any text,
//! taken when the input is left or Enter is pressed, and only a text that
//! differs from the value is a change; or it is chosen with the system's
//! folder picker through the button beside it, named in Clarity's own words,
//! and a chosen folder is taken at once. Where nothing picks a folder and on
//! a read-only one, there is no button. A bound one writes its field; an
//! unbound named one keeps its text in its state.
//!
//! The part takes the focus and holds its text editor, as a text box does;
//! the button is its child. The picker waits for the person, so it is asked
//! off the screen's thread and the folder comes back in `on_later`.

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
use crate::parts::containers::container::ContainerButton;
use crate::parts::inputs::file::Facts;
use crate::parts::text_editor::TextEditor;
use crate::parts::{Part, PartBase, PartHost};

const GAP: f64 = 4.0;

/// What the button does, in Clarity's own words: its name and the title of the picker.
pub fn choose_name() -> String {
    locale::translate("chooseFolder", "Choose a folder")
}

/// The folder the picker gave.
struct Picked(Option<String>);

pub struct FolderPart {
    base: PartBase,
    editor: TextEditor,
    own: String,
    shown: Option<String>,
}

pub fn make(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    Box::new(FolderPart::new(spec, host))
}

impl FolderPart {
    pub fn new(spec: PartSpec, host: Rc<dyn PartHost>) -> FolderPart {
        let mut base = PartBase::new(spec, host);
        let mut editor = TextEditor::new(false);
        editor.placeholder = base.spec.text("placeholder").map(|p| base.host.text(Some(p)));
        let facts = Rc::new(Facts { host: base.host.clone(), spec: base.spec.clone(), part: base.node.id() });
        let (choosing, showing, using) = (facts.clone(), facts.clone(), facts);
        let button = ContainerButton::new(|| "...".to_string(), move |cx| {
            if choosing.read_only() || !choosing.enabled() {
                return;
            }
            let picker = choosing.host.files();
            let title = choose_name();
            cx.later(choosing.part, move || Picked(picker.choose_folder(&title)));
        })
        .shows(move || !showing.read_only() && showing.host.files().picks_folders())
        .usable(move || using.enabled());
        base.node.add(Box::new(button));
        let mut part = FolderPart { base, editor, own: String::new(), shown: None };
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

    /// The button that opens the system's folder picker.
    pub fn choose_button(&self) -> &ContainerButton {
        self.base.node.children()[0].as_any().downcast_ref::<ContainerButton>().expect("the folder's button")
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
            self.base.host.change_state(&name, &mut |s| s.text = Some(typed.clone()));
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

    /// The input, and the button at its right when it shows, as tall as the input.
    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.base.node.bounds = area;
        self.ready();
        self.sync();
        let button = &mut self.base.node.children_mut()[0];
        if !button.is_shown() {
            button.arrange(Rect::new(area.right(), area.y, 0.0, area.height), cx);
            self.editor.place(area, cx);
            return;
        }
        let width = button.as_any().downcast_ref::<ContainerButton>().map(|b| b.width_for(cx)).unwrap_or(0.0).min(area.width / 2.0);
        button.arrange(Rect::new(area.right() - width, area.y, width, area.height), cx);
        self.editor.place(Rect::new(area.x, area.y, (area.width - width - GAP).max(0.0), area.height), cx);
    }

    fn draw(&self, p: &Painter) {
        self.editor.paint(p);
        crate::element::draw_children(&self.base.node, p);
    }

    fn on_pointer_pressed(&mut self, at: PointerInput, cx: &mut Cx) {
        self.ready();
        self.editor.pressed(at, cx);
    }

    fn on_pointer_moved(&mut self, at: PointerInput, cx: &mut Cx) {
        self.ready();
        self.editor.dragged(at, cx);
    }

    fn on_pointer_released(&mut self, _at: PointerInput, _cx: &mut Cx) {
        self.editor.released();
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
