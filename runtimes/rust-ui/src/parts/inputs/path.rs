//! A path on the disk, the same as on every stack: a folder part's (X2),
//! or a file part's bound to a `file` field (docs/ledger/files-pictures.md,
//! step 3). It is typed like any text, taken when the input is left or
//! Enter is pressed, and only a text that differs from the value is a
//! change; or it is chosen with the system's dialog through the "..." button
//! inside its right edge (a folder dialog, or a file dialog of the part's
//! `extensions`, the save dialog with `newFile`), which says what it does
//! in Clarity's own words when the pointer rests on it and takes no Tab
//! stop, and a chosen one is taken at once. The dialog opens where the path
//! it holds is, else in its `startFolder`. Where nothing picks one (a
//! browser) and on a read-only one, there is no button; where nothing picks
//! a folder, a folder part without a placeholder of its own says in
//! Clarity's words that the folder's path on the server is typed. A bound
//! one writes its field; an unbound named one keeps its text in its state.
//!
//! The part takes the focus and holds its text editor, as a text box does,
//! and its button, which it draws and hands the presses on it. The dialog
//! waits for the person, so it is asked off the screen's thread and the
//! path comes back in `on_later`.

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

/// What a path part chooses: a folder, or a file (of its extensions; one
/// not there yet with `newFile`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Chooses {
    Folder,
    File,
}

/// The path the dialog gave.
/// What the dialog gave: a path on the disk, or (in a browser) a file's
/// name and bytes with no path, kept in the part's store on the server.
enum Picked {
    Path(Option<String>),
    File(Option<crate::files::StorageFile>),
}

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

pub struct PathPart {
    base: PartBase,
    chooses: Chooses,
    editor: TextEditor,
    button: ChooseButton,
    /// The placeholder its markup gives.
    placeholder: Option<String>,
    own: String,
    shown: Option<String>,
    /// A press began on the button.
    pressing_button: bool,
    pointer: Option<(f64, f64)>,
    /// The program's file stores, asked once, which a folder part picks from where nothing picks a folder (a browser).
    stores: std::cell::OnceCell<Vec<(String, String)>>,
    /// The list of stores while it is open: its overlay and what it shows.
    store_list: Option<(crate::element::ElementId, crate::parts::inputs::choice_list::SharedList)>,
}

impl PathPart {
    pub fn new(spec: PartSpec, host: Rc<dyn PartHost>, chooses: Chooses) -> PathPart {
        let base = PartBase::new(spec, host);
        let mut editor = TextEditor::new(false);
        let placeholder = base.spec.text("placeholder").map(|p| base.host.text(Some(p)));
        editor.placeholder = placeholder.clone();
        editor.undoes = !base.spec.flag("noUndo");
        let mut side = InputSideButton::new(InputIcon::More);
        side.name = name_of(chooses);
        let button = ChooseButton { side, shown: false };
        let mut part = PathPart { base, chooses, editor, button, placeholder, own: String::new(), shown: None, pressing_button: false, pointer: None, stores: std::cell::OnceCell::new(), store_list: None };
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

    /// What it chooses.
    pub fn chooses(&self) -> Chooses {
        self.chooses
    }

    /// Whether the button shows: not on a read-only one, nor where nothing picks a path.
    /// A file part picks a file where the system picks files and in a browser
    /// (its file input); a folder part picks a folder where the system picks
    /// folders, and in a browser one of the program's file stores, when it has any.
    fn button_shows(&self) -> bool {
        if self.base.read_only() {
            return false;
        }
        match self.chooses {
            Chooses::File => true,
            Chooses::Folder => crate::files::system().picks_folders() || !self.program_stores().is_empty(),
        }
    }

    /// The program's file stores, asked once.
    fn program_stores(&self) -> &[(String, String)] {
        self.stores.get_or_init(|| self.base.host.stores())
    }

    /// The names in the list of stores while it is open.
    pub fn stores_open(&self) -> Option<Vec<String>> {
        self.store_list.as_ref().map(|(_, list)| list.borrow().labels.clone())
    }

    /// Picks the store at `at` of the list: its folder is taken, a change when it differs.
    pub fn pick_store(&mut self, at: usize) {
        self.store_list = None;
        if let Some((_, folder)) = self.program_stores().get(at).cloned() {
            self.chosen(&folder);
        }
    }

    /// Opens the list of the program's file stores below the part (a browser's folder part).
    fn open_stores(&mut self, cx: &mut Cx) {
        let labels: Vec<String> = self.program_stores().iter().map(|(name, _)| name.clone()).collect();
        if labels.is_empty() {
            return;
        }
        use crate::parts::inputs::choice_list;
        let bounds = self.base.node.bounds;
        let place = choice_list::below(cx.whole(), bounds, bounds.width.max(160.0), labels.len());
        let state: choice_list::SharedList = Rc::new(std::cell::RefCell::new(choice_list::ListState::new(labels, None)));
        let id = choice_list::open(cx, state.clone(), place, self.base.node.id());
        self.store_list = Some((id, state));
        cx.invalidate();
    }

    /// Asks the system's dialog off the screen's thread: a folder, or a file
    /// of its extensions (where one is saved with `newFile`), opening where
    /// the path it holds is, else in its start folder.
    fn choose(&mut self, cx: &mut Cx) {
        if self.base.read_only() || !self.base.enabled() {
            return;
        }
        let picker = crate::files::system();
        if self.chooses == Chooses::Folder && !picker.picks_folders() {
            self.open_stores(cx);
            return;
        }
        let held = self.value();
        let held = held.trim();
        let spec = &self.base.spec;
        let written_start = || crate::files::start_folder(spec.text("startFolder"));
        let mut options = crate::files::PickerOptions::titled(&name_of(self.chooses));
        options.hidden = spec.flag("hidden");
        match self.chooses {
            Chooses::Folder => {
                options.start = if held.is_empty() { written_start() } else { Some(held.to_string()) };
                // Its extensions: the dialogs that show files while a folder is chosen show those.
                options.file_type = crate::files::filter_of(&super::picker::Takes::of(spec, &*self.base.host).patterns);
                cx.later(self.base.node.id(), move || Picked::Path(picker.open_folder(&options).and_then(|f| f.path)));
            }
            Chooses::File => {
                let at = std::path::Path::new(held);
                let beside = at.parent().map(|p| p.to_string_lossy().into_owned()).filter(|p| !held.is_empty() && !p.is_empty());
                options.start = beside.or_else(written_start);
                let takes = super::picker::Takes::of(spec, &*self.base.host);
                options.file_type = crate::files::filter_of(&takes.patterns);
                options.hidden = takes.hidden;
                if spec.flag("newFile") {
                    options.suggested_name = at.file_name().map(|n| n.to_string_lossy().into_owned());
                    cx.later(self.base.node.id(), move || Picked::Path(picker.save_file(&options).and_then(|f| f.path)));
                } else {
                    cx.later(self.base.node.id(), move || Picked::File(picker.open_file(&options)));
                }
            }
        }
    }

    /// The path it shows: its field, or the text it keeps unbound.
    pub fn value(&self) -> String {
        match self.base.bound() {
            Some(path) => match self.base.host.read(path) {
                None | Some(Value::Null) => String::new(),
                Some(Value::String(s)) => s,
                Some(other) => other.to_string(),
            },
            None => self.base.state().and_then(|s| s.text.clone()).unwrap_or_else(|| self.own.clone()),
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
        if !self.takes_path(&typed) {
            // Refused: it shows what it holds again.
            let held = self.shown.clone().unwrap_or_default();
            self.editor.set_text(&held);
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

    /// Whether it takes the path: a file of its extensions, a hidden one only
    /// with hidden; one it does not take is refused in Clarity's own words.
    /// Any folder is taken: a folder's name is no file's.
    fn takes_path(&self, path: &str) -> bool {
        if self.chooses == Chooses::Folder {
            return true;
        }
        super::picker::Takes::of(&self.base.spec, &*self.base.host).take(&*self.base.host, path)
    }

    /// Keeps a file the browser gave in the part's store on the server and
    /// takes the path it got there; one of another type is refused first, and
    /// one that could not be kept says why.
    fn keep_upload(&mut self, file: &crate::files::StorageFile) {
        if self.base.read_only() || !self.base.enabled() || !self.takes_path(&file.name) {
            return;
        }
        let host = self.base.host.clone();
        // The checker refuses a web app's file part without a store (clarity_spec::file_parts_without_store).
        let Some(store) = self.base.spec.text("store").map(str::to_string) else {
            eprintln!("{}: names no file store to keep a picked file in", self.base.name().unwrap_or("a file part"));
            return;
        };
        let Some(bytes) = file.read() else { return };
        match host.upload(&store, &file.name, &bytes) {
            Ok(path) => self.chosen(&path),
            Err(why) => host.say(clarity_runtime::ui::Level::Error, why),
        }
    }

    /// Takes a path the dialog gave: a change when it differs from the value.
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
        self.editor.marked = state.and_then(|s| s.error.clone()).is_some_and(|e| !e.show().is_empty());
        let typed_only = self.chooses == Chooses::Folder && !self.base.read_only() && !crate::files::system().picks_folders();
        self.editor.placeholder = self.placeholder.clone().or_else(|| typed_only.then(super::folder::type_name));
        self.editor.mirror(&self.base.node);
    }
}

impl Element for PathPart {
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
            match *picked {
                Picked::Path(Some(path)) | Picked::File(Some(crate::files::StorageFile { path: Some(path), .. })) if !path.is_empty() => self.chosen(&path),
                // A browser's file: refused here when it is of another type, else kept in the part's store on the server.
                Picked::File(Some(file)) => self.keep_upload(&file),
                _ => {}
            }
        }
        cx.invalidate();
    }

    fn on_overlay_closed(&mut self, overlay: crate::element::ElementId, cx: &mut Cx) {
        let Some((id, list)) = self.store_list.take() else { return };
        if id != overlay {
            self.store_list = Some((id, list));
            return;
        }
        let picked = list.borrow().picked;
        if let Some(at) = picked {
            self.pick_store(at);
        }
        cx.invalidate();
    }
}

impl Part for PathPart {
    fn base(&self) -> &PartBase {
        &self.base
    }
}

/// What its button says it does, and the dialog's title, in Clarity's own words.
fn name_of(chooses: Chooses) -> String {
    match chooses {
        Chooses::Folder => super::folder::choose_name(),
        Chooses::File => locale::translate("chooseFile", "Choose a file"),
    }
}
