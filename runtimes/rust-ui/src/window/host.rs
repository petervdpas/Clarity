//! The host of one open window's parts: what they read is the runtime's
//! `WindowState`, what they change goes into it, and their handlers and edits
//! run through the program (`clarity_runtime::program::Program`), whose reply
//! is the window's new state. What a reply asks of the screen (messages,
//! windows to open, closing, the clipboard) waits here for the window stack.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::sync::Arc;

use clarity_runtime::program::{Answer, Edit, Program};
use clarity_runtime::ui::{Level, Message, PartState, Reply, Text, WindowState};
use serde_json::{Map, Value};

use crate::app_files::AppFiles;
use crate::markup::{HandlerCall, PartSpec};
use crate::parts::PartHost;
use crate::window::paths;

/// What a reply asks of the screen, for the window stack to do.
#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    /// Messages to show: an info note, a warning or an error in Clarity's dialog.
    Say(Vec<Message>),
    /// A window to open in place, with its values.
    Open(String, Map<String, Value>),
    /// The window closes.
    Close,
    /// Text to put on the clipboard.
    Copy(String),
}

/// The edit the person made that is not sent yet: the part and its undo scope.
#[derive(Debug, Clone, PartialEq)]
struct Pending {
    part: String,
    scope: String,
}

pub struct ProgramHost {
    program: Arc<dyn Program>,
    files: AppFiles,
    state: RefCell<WindowState>,
    specs: BTreeMap<String, PartSpec>,
    changes: Cell<u64>,
    effects: RefCell<Vec<Effect>>,
    pending: RefCell<Option<Pending>>,
}

/// Every part of a window's tree by its name: in panels, pages and all.
fn named(parts: &[PartSpec], into: &mut BTreeMap<String, PartSpec>) {
    for part in parts {
        if let Some(name) = part.name() {
            into.insert(name.to_string(), part.clone());
        }
        named(&part.children, into);
        for page in &part.pages {
            named(&page.children, into);
        }
    }
}

impl ProgramHost {
    /// The host of the window whose state is `state`, with the parts of its markup.
    pub fn new(program: Arc<dyn Program>, files: AppFiles, state: WindowState, parts: &[PartSpec]) -> ProgramHost {
        let mut specs = BTreeMap::new();
        named(parts, &mut specs);
        ProgramHost { program, files, state: RefCell::new(state), specs, changes: Cell::new(0), effects: RefCell::new(Vec::new()), pending: RefCell::new(None) }
    }

    /// The window's state now.
    pub fn state_now(&self) -> WindowState {
        self.state.borrow().clone()
    }

    /// The window's name.
    pub fn window(&self) -> String {
        self.state.borrow().window.clone()
    }

    /// What the replies asked of the screen since the last time, in order.
    pub fn take_effects(&self) -> Vec<Effect> {
        std::mem::take(&mut *self.effects.borrow_mut())
    }

    fn changed(&self) {
        self.changes.set(self.changes.get() + 1);
    }

    /// Where the edits of `part` are undone: its `undoScope`, else its own name; none with `no undo`.
    pub fn scope(&self, part: &str) -> Option<String> {
        match self.specs.get(part) {
            None => Some(part.to_string()),
            Some(spec) if spec.flag("noUndo") => None,
            Some(spec) => Some(spec.text("undoScope").filter(|s| !s.is_empty()).unwrap_or(part).to_string()),
        }
    }

    fn read_only(&self, part: &str) -> bool {
        let designed = self.specs.get(part).is_some_and(|s| s.flag("readonly"));
        designed || self.state.borrow().parts.get(part).and_then(|s| s.readonly).unwrap_or(false)
    }

    /// Takes a reply: the window's new state, and what it asks of the screen.
    pub fn apply(&self, reply: Reply) {
        let mut state = reply.state;
        // The texts the handler put in text values show in the person's language.
        for (name, text) in &reply.shown {
            state.data.insert(name.clone(), Value::from(text.show()));
        }
        *self.state.borrow_mut() = state;
        let mut effects = self.effects.borrow_mut();
        if !reply.messages.is_empty() {
            effects.push(Effect::Say(reply.messages));
        }
        if let Some(copy) = reply.copy {
            effects.push(Effect::Copy(copy.show()));
        }
        for open in reply.open {
            effects.push(Effect::Open(open.window, open.with));
        }
        if reply.close {
            effects.push(Effect::Close);
        }
        drop(effects);
        self.changed();
    }

    /// Takes what a call of the program answered: its reply, or what went wrong as an error message. True when it answered a reply.
    pub fn answered(&self, answer: Answer<Reply>) -> bool {
        match answer {
            Ok(reply) => {
                self.apply(reply);
                true
            }
            Err(text) => {
                self.effects.borrow_mut().push(Effect::Say(vec![Message { level: Level::Error, text }]));
                self.changed();
                false
            }
        }
    }

    /// Sends the person's edit that waits, with the handler of its change when it has one; true when nothing refused.
    fn send_edit(&self, pending: Pending, handler: Option<String>) -> bool {
        let readonly = self.read_only(&pending.part);
        let edit = Edit { handler, part: pending.part, scope: pending.scope, ends: true, readonly, path: None, editable: None };
        let state = self.state_now();
        self.answered(self.program.edit_event(state, edit))
    }

    /// The person's edit that waits is sent: what was typed in an input without a change handler is one edit.
    pub fn flush(&self) {
        let pending = self.pending.borrow_mut().take();
        if let Some(pending) = pending {
            self.send_edit(pending, None);
        }
    }

    /// The person's edit of `part`: sent once the event is over, or with the change handler that follows it.
    fn edit_of(&self, part: &str) {
        let Some(scope) = self.scope(part) else { return };
        let earlier = self.pending.borrow_mut().replace(Pending { part: part.to_string(), scope });
        if let Some(earlier) = earlier.filter(|e| e.part != part) {
            self.send_edit(earlier, None);
        }
    }

    /// Ctrl+Z (`back`) or Ctrl+Y on a scope: its last edit taken back or done again.
    pub fn undo(&self, scope: &str, back: bool) {
        self.flush();
        let state = self.state_now();
        self.answered(self.program.undo_edit(state, scope, back));
    }

    /// The window is on top again: its `on return` runs.
    pub fn returned(&self) {
        let state = self.state_now();
        self.answered(self.program.return_to(state));
    }

    /// The window closes: its `on close` runs, and what its reply says shows in the window below.
    pub fn closing(&self) -> Vec<Effect> {
        self.flush();
        let state = self.state_now();
        self.answered(self.program.close_window(state));
        self.take_effects().into_iter().filter(|e| !matches!(e, Effect::Close)).collect()
    }
}

impl PartHost for ProgramHost {
    fn read(&self, path: &str) -> Option<Value> {
        paths::read(&self.state.borrow().data, path)
    }

    fn write(&self, part: &str, path: &str, value: Value) {
        if self.read(path).as_ref() == Some(&value) {
            return;
        }
        paths::write(&mut self.state.borrow_mut().data, path, value);
        self.changed();
        self.edit_of(part);
    }

    fn edited(&self, part: &str) {
        self.changed();
        self.edit_of(part);
    }

    fn state(&self, name: Option<&str>) -> Option<PartState> {
        let name = name?;
        Some(self.state.borrow().parts.get(name).cloned().unwrap_or_default())
    }

    fn change_state(&self, name: &str, change: &mut dyn FnMut(&mut PartState)) {
        change(self.state.borrow_mut().parts.entry(name.to_string()).or_default());
        self.changed();
    }

    fn run(&self, part: Option<&str>, call: &HandlerCall) -> bool {
        // A click that names a window opens it.
        if call.args.is_empty() && self.files.window(&call.handler).is_some() {
            self.flush();
            self.effects.borrow_mut().push(Effect::Open(call.handler.clone(), Map::new()));
            self.changed();
            return true;
        }
        // A change the person made is an edit: what its handler changes is taken back with it.
        if matches!(call.event.as_str(), "change" | "reorder") {
            if let Some(part) = part {
                if let Some(scope) = self.scope(part) {
                    let waiting = self.pending.borrow_mut().take();
                    if let Some(other) = waiting.as_ref().filter(|w| w.part != part) {
                        self.send_edit(other.clone(), None);
                    }
                    return self.send_edit(Pending { part: part.to_string(), scope }, Some(call.written()));
                }
            }
        }
        self.flush();
        let state = self.state_now();
        self.answered(self.program.dispatch_event(state, &call.written()))
    }

    fn text(&self, written: Option<&str>) -> String {
        match written {
            None => String::new(),
            Some(w) => match w.strip_prefix('@') {
                Some(key) => clarity_runtime::i18n::value(key).unwrap_or_else(|| key.to_string()),
                None => w.to_string(),
            },
        }
    }

    fn changes(&self) -> u64 {
        self.changes.get()
    }

    fn say(&self, level: Level, text: Text) {
        self.effects.borrow_mut().push(Effect::Say(vec![Message { level, text }]));
        self.changed();
    }

    fn bound_of(&self, part: &str) -> Option<String> {
        self.specs.get(part).and_then(|s| s.text("bind")).filter(|b| !b.is_empty()).map(str::to_string)
    }
}
