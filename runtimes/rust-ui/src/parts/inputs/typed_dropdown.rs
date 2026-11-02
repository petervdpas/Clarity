//! A box to type in with a list of what fits below it, the same as on every
//! stack (conformance/dropdown-options.json): the dropdown's `typed`
//! variant (any text, the values its field already has offered) and its
//! `search` variant (a record a reference points at, picked among many,
//! filtered as it is typed). The list opens when the box gets the focus or
//! text is typed; Up and Down move through it, Enter or a click picks,
//! Escape closes it and puts the value's text back. Typed text that is not
//! picked counts only for `typed`; for `search`, empty text is no record.
//! Only a value that differs is a change; what code sets is never one.
//!
//! What fits is decided by the runtime's rules (`clarity_runtime::dropdown_options`).

use std::cell::RefCell;
use std::rc::Rc;

use clarity_runtime::dropdown_options;
use clarity_runtime::ui::RefOption;
use serde_json::{Map, Value};

use crate::element::{Cx, Element, ElementId};
use crate::geometry::Rect;
use crate::input::{Key, KeyInput, Modifiers, PointerInput};
use crate::markup::PartSpec;
use crate::painter::Painter;
use crate::part_element;
use crate::parts::inputs::choice_list::{self, text_of, ListState, SharedList};
use crate::parts::inputs::choices;
use crate::parts::text_editor::TextEditor;
use crate::parts::{Choice, Part, PartBase, PartHost};

/// Which of the two variants a type-ahead dropdown is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Variant {
    /// Any text may be typed; it offers the values its field already has.
    Typed,
    /// A record of a reference, filtered as it is typed, each with the fields it shows.
    Search,
}

/// The variant a dropdown's spec asks for and fits: `typed`, or `search`
/// bound to a reference; none for a plain dropdown.
pub fn variant_of(spec: &PartSpec, host: &dyn PartHost) -> Option<Variant> {
    if spec.flag("typed") {
        return Some(Variant::Typed);
    }
    let path = spec.text("bind").filter(|p| !p.is_empty())?;
    let refers = host.field(path).is_some_and(|f| f.refers.is_some());
    if spec.flag("search") && (refers || host.offered(spec, path).iter().any(|c| c.cells.is_some())) {
        return Some(Variant::Search);
    }
    None
}

/// A record's pieces: its text and the fields it shows, each once, none empty.
fn pieces(row: &Choice) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for piece in std::iter::once(&row.label).chain(row.cells.iter().flatten()) {
        if !piece.is_empty() && !out.contains(piece) {
            out.push(piece.clone());
        }
    }
    out
}

/// A record's line in a search's list: its text and the fields it shows, each once.
pub fn line_of(row: &Choice) -> String {
    pieces(row).join(", ")
}

pub struct TypeAheadPart {
    base: PartBase,
    variant: Variant,
    editor: TextEditor,
    /// Its list while it is open: the overlay's id and the state it shares with it.
    list: Option<(ElementId, SharedList)>,
    /// The rows the open list shows.
    rows: Vec<Choice>,
    /// The value's text as it was last shown; none before the first.
    shown: Option<String>,
    /// The focus went to its own list for a press there, and comes back: no leaving.
    lent: bool,
}

impl TypeAheadPart {
    pub fn new(spec: PartSpec, host: Rc<dyn PartHost>, variant: Variant) -> TypeAheadPart {
        let base = PartBase::new(spec, host);
        let mut editor = TextEditor::new(false);
        editor.placeholder = base.spec.text("placeholder").map(|p| base.host.text(Some(p)));
        let mut part = TypeAheadPart { base, variant, editor, list: None, rows: Vec::new(), shown: None, lent: false };
        part.refresh();
        part.sync();
        part
    }

    pub fn variant(&self) -> Variant {
        self.variant
    }

    /// Where the person types.
    pub fn editor(&self) -> &TextEditor {
        &self.editor
    }

    pub fn editor_mut(&mut self) -> &mut TextEditor {
        &mut self.editor
    }

    /// Its list's overlay id, while it is open.
    pub fn list(&self) -> Option<ElementId> {
        self.list.as_ref().map(|(id, _)| *id)
    }

    /// The row the open list marks, which Enter picks.
    pub fn highlighted(&self) -> Option<usize> {
        self.list.as_ref().and_then(|(_, s)| s.borrow().highlighted)
    }

    fn usable(&self) -> bool {
        self.base.enabled() && !self.base.read_only() && (self.base.bound().is_some() || self.base.name().is_some())
    }

    /// What the host offers for its field.
    fn all_offered(&self) -> Vec<Choice> {
        match self.base.bound() {
            Some(path) => self.base.host.offered(&self.base.spec, path),
            None => Vec::new(),
        }
    }

    /// The value it has; none for none.
    /// Unbound and named, it keeps what was typed in its state's text.
    fn value(&self) -> Value {
        match self.base.bound() {
            Some(path) => self.base.host.read(path).unwrap_or(Value::Null),
            None => match self.base.state().and_then(|s| s.text).filter(|t| !t.is_empty()) {
                Some(text) => Value::String(text),
                None => Value::Null,
            },
        }
    }

    /// The records a search can point at, without the empty choice for none.
    fn records(&self) -> Vec<Choice> {
        self.all_offered().into_iter().filter(|c| !c.value.is_null()).collect()
    }

    /// What fits the text in the box now.
    pub fn offered(&self) -> Vec<Choice> {
        self.offer(&self.editor.text())
    }

    /// The rows that fit `typed`.
    fn offer(&self, typed: &str) -> Vec<Choice> {
        match self.variant {
            Variant::Typed => {
                let values: Vec<String> = self.all_offered().into_iter().map(|c| if let Value::String(s) = c.value { s } else { c.label }).collect();
                dropdown_options::suggestions(&values, typed).into_iter().map(|v| Choice { value: Value::String(v.clone()), label: v, cells: None }).collect()
            }
            Variant::Search => {
                // Each record goes to the runtime's rule by where it is, its pieces as the fields it shows.
                let records = self.records();
                let mut show: Vec<String> = Vec::new();
                let options: Vec<RefOption> = records
                    .iter()
                    .enumerate()
                    .map(|(at, row)| {
                        let mut fields = Map::new();
                        for (i, cell) in row.cells.iter().flatten().enumerate() {
                            let key = i.to_string();
                            if !show.contains(&key) {
                                show.push(key.clone());
                            }
                            fields.insert(key, Value::String(cell.clone()));
                        }
                        RefOption { id: at as i64, text: row.label.clone(), fields }
                    })
                    .collect();
                dropdown_options::searched(options, &show, typed).into_iter().filter_map(|o| records.get(o.id as usize).cloned()).collect()
            }
        }
    }

    /// The text its value shows in the box.
    fn value_text(&self) -> String {
        let value = self.value();
        match self.variant {
            Variant::Typed => text_of(&value),
            Variant::Search if value.is_null() => String::new(),
            Variant::Search => self.records().into_iter().find(|c| choices::same(&c.value, &value)).map(|c| c.label).unwrap_or_default(),
        }
    }

    /// A row was picked: the value it gives, and whether that differs from the value.
    fn picked(&self, row: &Choice) -> (Value, bool) {
        match self.variant {
            Variant::Typed => self.typed(&text_of(&row.value)),
            Variant::Search => (row.value.clone(), !choices::same(&row.value, &self.value())),
        }
    }

    /// The box was left or Enter pressed with text no row was picked for: the value it gives, and whether that is a change.
    fn typed(&self, typed: &str) -> (Value, bool) {
        match self.variant {
            Variant::Typed => (Value::String(typed.to_string()), typed != self.value_text()),
            // Typed text picks nothing; emptied, it is no record.
            Variant::Search => (Value::Null, typed.trim().is_empty() && !self.value().is_null()),
        }
    }

    /// The editor follows what code set: enabled, read-only, marked with an error.
    fn refresh(&mut self) {
        let state = self.base.state();
        self.editor.enabled = self.base.enabled();
        self.editor.read_only = self.base.read_only();
        self.editor.marked = state.and_then(|s| s.error).is_some_and(|e| !e.show().is_empty());
    }

    fn ready(&mut self) {
        self.refresh();
        self.editor.mirror(&self.base.node);
    }

    /// A value set from elsewhere shows as it is.
    pub fn sync(&mut self) {
        let now = self.value_text();
        if self.shown.as_deref() == Some(now.as_str()) {
            return;
        }
        self.editor.set_text(&now);
        self.shown = Some(now);
    }

    /// Puts the value's text back in the box.
    fn show(&mut self) {
        self.shown = None;
        self.sync();
    }

    /// Fills the list with what fits the text; it opens when asked and there is something to offer.
    fn fill(&mut self, open: bool, cx: &mut Cx) {
        if !self.usable() {
            return;
        }
        let rows = self.offer(&self.editor.text());
        self.close(cx);
        self.rows = rows;
        if !open || self.rows.is_empty() {
            return;
        }
        let lines: Vec<String> = self.rows.iter().map(|r| if self.variant == Variant::Search { line_of(r) } else { r.label.clone() }).collect();
        let mut state = ListState::new(lines, None);
        if self.variant == Variant::Search && self.rows.iter().all(|r| r.cells.is_some()) {
            state.cells = Some(self.rows.iter().map(pieces).collect());
        }
        let bounds = self.base.node.bounds;
        let place = choice_list::below(cx.whole(), bounds, bounds.width, self.rows.len());
        let state: SharedList = Rc::new(RefCell::new(state));
        let id = choice_list::open(cx, state.clone(), place, self.base.node.id());
        self.list = Some((id, state));
    }

    fn close(&mut self, cx: &mut Cx) {
        if let Some((id, _)) = self.list.take() {
            cx.close(id);
        }
        cx.invalidate();
    }

    fn write(&mut self, value: Value) {
        match (self.base.bound().map(str::to_string), self.base.name().map(str::to_string)) {
            (Some(path), name) => self.base.host.write(name.as_deref().unwrap_or(""), &path, value),
            (None, Some(name)) => {
                let text = choice_list::text_of(&value);
                crate::parts::change_state(&*self.base.host, &name, |s| s.text = Some(text.clone()));
                self.base.host.edited(&name);
            }
            (None, None) => return,
        }
        self.base.fire("change");
    }

    /// Picks a row: a change when it gives another value.
    pub fn pick(&mut self, row: &Choice, cx: &mut Cx) {
        self.close(cx);
        if self.usable() {
            let (value, differs) = self.picked(row);
            if differs {
                self.write(value);
            }
        }
        self.show();
    }

    /// Takes what is typed, as when the box is left.
    pub fn take(&mut self, cx: &mut Cx) {
        self.close(cx);
        let typed = self.editor.text();
        if self.usable() && typed != self.value_text() {
            let (value, differs) = self.typed(&typed);
            if differs {
                self.write(value);
            }
        }
        self.show();
    }

    /// Text in the box changed: the list follows it.
    fn edited(&mut self, before: u64, cx: &mut Cx) {
        if self.editor.edits() != before && self.base.node.focused {
            self.fill(true, cx);
        }
    }
}

impl Element for TypeAheadPart {
    part_element!();

    fn focusable(&self) -> bool {
        self.base.enabled()
    }

    fn typing(&self) -> bool {
        true
    }

    fn take_typed(&mut self, cx: &mut Cx) -> bool {
        self.take(cx);
        true
    }

    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.base.node.bounds = area;
        self.ready();
        self.sync();
        self.editor.place(area, cx);
    }

    fn draw(&self, p: &Painter) {
        self.editor.paint(p);
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
            if std::mem::take(&mut self.lent) {
                return;
            }
            self.sync();
            if self.list.is_none() {
                self.fill(true, cx);
            }
            self.base.fire("enter");
        } else if self.list.is_some() && cx.focused().is_none() {
            // A press in its own list takes the focus for a moment and gives it back.
            self.lent = true;
        } else {
            self.take(cx);
            self.base.fire("leave");
        }
    }

    fn on_key(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        self.ready();
        match key.key {
            Key::Down | Key::Up if key.modifiers.is_empty() => {
                if self.list.is_none() {
                    self.fill(true, cx);
                }
                // As C#: the mark goes round, from the last row to the first and back.
                if let Some((_, state)) = self.list.as_ref() {
                    let mut st = state.borrow_mut();
                    let count = st.count();
                    if count > 0 {
                        let down = key.key == Key::Down;
                        st.highlighted = Some(match st.highlighted {
                            None if down => 0,
                            None => count - 1,
                            Some(at) if down => (at + 1) % count,
                            Some(at) => (at + count - 1) % count,
                        });
                        st.reveal();
                    }
                    cx.invalidate();
                }
                return true;
            }
            Key::Enter if key.modifiers.is_empty() => {
                if let Some(at) = self.highlighted() {
                    let rows = std::mem::take(&mut self.rows);
                    self.close(cx);
                    if let Some(row) = rows.get(at) {
                        self.pick(row, cx);
                    }
                    return true;
                }
                // Enter with no row marked takes the text, and stays here: the form does not submit.
                self.take(cx);
                return true;
            }
            Key::Escape if self.list.is_some() => {
                self.close(cx);
                self.show();
                return true;
            }
            _ => {}
        }
        let before = self.editor.edits();
        let took = self.editor.key(key, cx);
        self.edited(before, cx);
        took
    }

    fn on_text(&mut self, text: &str, cx: &mut Cx) {
        self.ready();
        let before = self.editor.edits();
        self.editor.typed(text, cx);
        self.edited(before, cx);
    }

    fn on_overlay_closed(&mut self, overlay: ElementId, cx: &mut Cx) {
        let Some((id, state)) = self.list.take() else { return };
        if id != overlay {
            self.list = Some((id, state));
            return;
        }
        let picked = state.borrow().picked;
        if let Some(row) = picked.and_then(|at| self.rows.get(at).cloned()) {
            self.pick(&row, cx);
        }
        cx.invalidate();
    }

    fn on_detached(&mut self, cx: &mut Cx) {
        self.close(cx);
    }

    fn caret_area(&self) -> Option<Rect> {
        self.editor.caret_area()
    }
}

impl Part for TypeAheadPart {
    fn base(&self) -> &PartBase {
        &self.base
    }
}
