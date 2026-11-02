//! A file part (U1c), the same as on every stack: the name of a record's
//! file (cut off with an ellipsis when it is too long), or that it has none,
//! with buttons to choose another, open it in the program the system uses
//! for its kind, save a copy of it and remove it, in that order, named in
//! Clarity's own words. A saved copy says where it went: the place the
//! person chose on the desktop, the Downloads folder in a browser. A stored
//! file is fetched only to open it or save a copy. Choosing and removing are changes, each running its
//! change handlers; opening and saving a copy are not. A read-only one
//! neither chooses nor removes.
//!
//! The system's dialogs wait for the person, so they are asked off the
//! screen's thread (`Cx::later`); what was chosen comes back to the part
//! (`Element::on_later`).

use std::cell::RefCell;
use std::rc::Rc;

use crate::wire::{clean_name, FieldContent, FieldFile};
use clarity_runtime::ui::{Level, Text};
use serde_json::Value;

use crate::element::{Cx, Element, ElementId};
use crate::files::ChosenFile;
use crate::geometry::Rect;
use crate::later::Done;
use crate::locale;
use crate::markup::PartSpec;
use crate::painter::Painter;
use crate::part_element;
use crate::parts::containers::container::ContainerButton;
use crate::parts::{Part, PartBase, PartHost};

const GAP: f64 = 4.0;

/// What a part knows of its spec and host, shared with its buttons.
pub(crate) struct Facts {
    pub host: Rc<dyn PartHost>,
    pub spec: PartSpec,
    /// The part's id: what its dialogs give comes back to it.
    pub part: ElementId,
}

impl Facts {
    pub fn bound(&self) -> Option<&str> {
        self.spec.text("bind").filter(|b| !b.is_empty())
    }

    pub fn enabled(&self) -> bool {
        self.host.state(self.spec.name()).and_then(|s| s.enabled).unwrap_or(true)
    }

    pub fn read_only(&self) -> bool {
        self.spec.flag("readonly") || self.host.state(self.spec.name()).and_then(|s| s.readonly).unwrap_or(false)
    }

    pub fn fire(&self, event: &str) {
        for call in self.spec.on(event) {
            self.host.run(self.spec.name(), call);
        }
    }

    /// Writes the bound field, by the part's name.
    pub fn write(&self, value: Value) {
        if let Some(path) = self.bound() {
            self.host.write(self.spec.name().unwrap_or(""), path, value);
        }
    }

    /// The record's file; none when it has none.
    fn file(&self) -> Option<FieldFile> {
        let value = self.host.read(self.bound()?)?;
        FieldFile::read(&value)
    }

    /// The bytes of a field's file: new ones as they are, a stored one or one of the store fetched through the window.
    pub fn bytes_of(&self, content: &FieldContent) -> Option<Vec<u8>> {
        let bind = self.bound()?;
        match content {
            FieldContent::New(bytes) => Some(bytes.clone()),
            FieldContent::Stored => self.host.fetch(bind, None),
            FieldContent::InStore(path) => self.host.fetch(bind, Some(path)),
        }
    }

    /// The file removed, a change when there was one.
    fn remove(&self) {
        if self.read_only() || self.file().is_none() || self.bound().is_none() {
            return;
        }
        self.write(Value::Null);
        self.fire("change");
    }

    /// Opens the file in the program the system uses for its kind.
    fn open_it(&self) {
        let Some(file) = self.file() else { return };
        if let Some(bytes) = self.bytes_of(&file.content) {
            crate::files::system().show(&file.name, &bytes);
        }
    }

    /// Asks where a copy goes, off the screen's thread.
    fn save_copy(&self, cx: &mut Cx) {
        let Some(file) = self.file() else { return };
        let Some(bytes) = self.bytes_of(&file.content) else { return };
        let picker = crate::files::system();
        let title = locale::translate("saveFileCopy", "Save a copy");
        cx.later(self.part, move || Asked::Saved(picker.save_file(&title, &file.name, &bytes)));
    }

    /// Asks the system's dialog for a file, off the screen's thread.
    fn choose(&self, cx: &mut Cx) {
        if self.read_only() || !self.enabled() {
            return;
        }
        let picker = crate::files::system();
        let title = locale::translate("chooseFile", "Choose a file");
        cx.later(self.part, move || Asked::Chosen(picker.open_file(&title, None)));
    }
}

/// What a dialog gave back.
enum Asked {
    Chosen(Option<ChosenFile>),
    Saved(Option<String>),
}

pub struct FilePart {
    base: PartBase,
    facts: Rc<Facts>,
    name_box: Rect,
    /// Where the last copy saved went.
    saved: Option<String>,
    /// The name as it was drawn last.
    drawn_name: RefCell<String>,
}

pub fn make(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    Box::new(FilePart::new(spec, host))
}

/// Its buttons, by the order they were added: the order they read, which is the Tab order too.
const CHOOSE: usize = 0;
const OPEN: usize = 1;
const SAVE: usize = 2;
const REMOVE: usize = 3;

/// What a saved copy says: the place chosen (a whole path, on the desktop),
/// else that it is in the Downloads folder (a browser's download).
pub fn saved_where(place: &str) -> String {
    let path = std::path::Path::new(place);
    if path.is_absolute() || path.has_root() {
        locale::fill(&locale::translate("fileCopySavedAt", "A copy is saved as {place}"), "place", place)
    } else {
        locale::translate("fileCopySaved", "A copy is in your Downloads folder")
    }
}

impl FilePart {
    pub fn new(spec: PartSpec, host: Rc<dyn PartHost>) -> FilePart {
        let mut base = PartBase::new(spec, host);
        let facts = Rc::new(Facts { host: base.host.clone(), spec: base.spec.clone(), part: base.node.id() });
        // Each button reads what it shows and does through the part's facts.
        let f = |use_: fn(&Facts) -> bool| {
            let facts = facts.clone();
            move || use_(&facts)
        };
        let (opening, saving, choosing, removing) = (facts.clone(), facts.clone(), facts.clone(), facts.clone());
        let open = ContainerButton::new(|| locale::translate("openFile", "Open"), move |_| opening.open_it()).shows(f(|f| f.file().is_some())).usable(f(Facts::enabled));
        let save = ContainerButton::new(|| locale::translate("saveFileCopy", "Save a copy"), move |cx| saving.save_copy(cx)).shows(f(|f| f.file().is_some())).usable(f(Facts::enabled));
        let choose = ContainerButton::new(|| locale::translate("chooseFile", "Choose a file"), move |cx| choosing.choose(cx)).shows(f(|f| !f.read_only())).usable(f(Facts::enabled));
        let remove = ContainerButton::new(|| locale::translate("removeFile", "Remove the file"), move |_| removing.remove()).shows(f(|f| !f.read_only())).usable(f(|f| f.enabled() && f.file().is_some()));
        for button in [choose, open, save, remove] {
            base.node.add(Box::new(button));
        }
        FilePart { base, facts, name_box: Rect::default(), saved: None, drawn_name: RefCell::new(String::new()) }
    }

    /// The record's file; none when it has none.
    pub fn file(&self) -> Option<FieldFile> {
        self.facts.file()
    }

    pub fn is_empty(&self) -> bool {
        self.file().is_none()
    }

    /// What shows of the file: its name, or that there is none.
    pub fn shown(&self) -> String {
        self.file().map(|f| f.name).unwrap_or_else(|| locale::translate("noFile", "No file"))
    }

    fn button(&self, index: usize) -> &ContainerButton {
        self.base.node.children()[index].as_any().downcast_ref::<ContainerButton>().expect("a file part's button")
    }

    /// Opens the file in the program the system uses for its kind.
    pub fn open_button(&self) -> &ContainerButton {
        self.button(OPEN)
    }

    /// Saves a copy of the file where the person chooses.
    pub fn save_button(&self) -> &ContainerButton {
        self.button(SAVE)
    }

    /// Chooses a file with the system's dialog.
    pub fn choose_button(&self) -> &ContainerButton {
        self.button(CHOOSE)
    }

    /// Removes the file.
    pub fn remove_button(&self) -> &ContainerButton {
        self.button(REMOVE)
    }

    /// Where the file's name shows.
    pub fn name_box(&self) -> Rect {
        self.name_box
    }

    /// The name as it was drawn last: cut off with an ellipsis where it is wider than its place.
    pub fn drawn_name(&self) -> String {
        self.drawn_name.borrow().clone()
    }

    /// Where the last copy saved went; none before one was.
    pub fn saved(&self) -> Option<&str> {
        self.saved.as_deref()
    }

    /// A file chosen: it replaces what the record had, a change.
    pub fn take(&self, chosen: &str, bytes: Vec<u8>) {
        if self.facts.read_only() || self.facts.bound().is_none() {
            return;
        }
        let file = FieldFile { name: clean_name(chosen), content: FieldContent::New(bytes) };
        self.facts.write(file.value());
        self.facts.fire("change");
    }

    /// The file removed, a change when there was one.
    pub fn remove(&self) {
        self.facts.remove();
    }

    /// The file's bytes: new ones as chosen, a stored one fetched; none for none, or when fetching failed.
    pub fn bytes(&self) -> Option<Vec<u8>> {
        self.file().and_then(|f| self.facts.bytes_of(&f.content))
    }
}

impl Element for FilePart {
    part_element!();

    /// The name on the left, the buttons that show on the right, each as wide as its words.
    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.base.node.bounds = area;
        let mut right = area.right();
        // From the right: Remove, Save a copy, Open, Choose; so they read Choose, Open, Save a copy, Remove.
        for index in [REMOVE, SAVE, OPEN, CHOOSE] {
            let button = &mut self.base.node.children_mut()[index];
            let shown = button.is_shown();
            let width = if shown { button.as_any().downcast_ref::<ContainerButton>().map(|b| b.width_for(cx)).unwrap_or(0.0).min((right - area.x).max(0.0)) } else { 0.0 };
            if !shown {
                button.arrange(Rect::new(right, area.y, 0.0, area.height), cx);
                continue;
            }
            right -= width;
            button.arrange(Rect::new(right, area.y, width, area.height), cx);
            right -= GAP;
        }
        self.name_box = Rect::new(area.x, area.y, (right - area.x).max(0.0), area.height);
    }

    fn draw(&self, p: &Painter) {
        let look = p.look;
        let _dimmed = p.dimmed(self.facts.enabled());
        let empty = self.is_empty();
        let place = self.name_box.inset(look.padding());
        let drawn = p.fit(&self.shown(), place.width, look.size("text"), "text", false);
        p.plain_text(&drawn, place, look.color(if empty { "muted" } else { "text" }), look.size("text"));
        *self.drawn_name.borrow_mut() = drawn;
        crate::element::draw_children(&self.base.node, p);
    }

    fn on_later(&mut self, done: Done, cx: &mut Cx) {
        let Ok(asked) = done.downcast::<Asked>() else { return };
        match *asked {
            Asked::Chosen(Some(chosen)) => self.take(&chosen.name, chosen.bytes),
            Asked::Saved(Some(place)) => {
                self.base.host.say(Level::Info, Text::plain(saved_where(&place)));
                self.saved = Some(place);
            }
            _ => {}
        }
        cx.invalidate();
    }
}

impl Part for FilePart {
    fn base(&self) -> &PartBase {
        &self.base
    }
}
