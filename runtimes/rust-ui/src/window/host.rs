//! The host of one open window's parts: what they read is the runtime's
//! `WindowState`, what they change goes into it, and their handlers and edits
//! run through the program (`clarity_runtime::program::Program`), whose reply
//! is the window's new state. What a reply asks of the screen (messages,
//! windows to open, closing, the clipboard) waits here for the window stack.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::Arc;

use clarity_runtime::canvas::CanvasArgs;
use clarity_runtime::program::{Answer, CanvasEdit, Edit, Program};
use clarity_runtime::ui::{Ask, Hear, Level, Message, PartState, Reply, Text, WindowState};
use serde_json::{Map, Value};

use crate::app_files::AppFiles;
use crate::markup::{HandlerCall, PartSpec};
use crate::facts::ModelFacts;
use crate::parts::{Choice, FieldFacts, Formats, PartHost};
use crate::window::paths;
use crate::window::Running;

/// What a reply asks of the screen, for the window stack to do.
#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    /// Messages to show: an info note, a warning or an error in Clarity's dialog.
    Say(Vec<Message>),
    /// A window to open in place, with its values.
    Open(String, Map<String, Value>),
    /// A window a route marked `in new window` (or a handler's `open ... in
    /// new window`) opens, with its values: in a desktop window of its own
    /// where the host gives them, else in place.
    OpenNew(String, Map<String, Value>),
    /// The window closes.
    Close,
    /// Text to put on the clipboard.
    Copy(String),
    /// A question to ask in Clarity's own dialog, or a dialog window to open for its answer.
    Ask(Ask),
    /// Clarity's keys window opens (the built-in target `keys`).
    Keys,
    /// The program ends (the built-in target `quit`).
    Quit,
    /// A handler to run off the screen's thread, its program's long calls watched.
    Call(Call),
    /// Someone signed in on the sign-in window (`signin <strategy>`): it
    /// closes and the main window opens.
    SignedIn,
    /// The person signed out (`signout`): every window closes and the
    /// sign-in window shows again.
    SignedOut,
}

/// A handler that runs off the screen's thread, with the window's state as it was when it started.
#[derive(Debug, Clone, PartialEq)]
pub enum Call {
    /// A handler of the window, as Clarity writes its call.
    Dispatch(String),
    /// The handler an answer goes to, with its values.
    Answer(String, Vec<Value>),
}

impl Call {
    /// Runs it on `program` with `state`.
    pub fn run(self, program: &dyn Program, state: WindowState) -> Answer<Reply> {
        match self {
            Call::Dispatch(handler) => program.dispatch_event(state, &handler),
            Call::Answer(handler, values) => program.answer_event(state, &handler, values),
        }
    }

    /// Its request on the wire with `state`, run under the long call's
    /// `watch` (the browser sends it without waiting): the command and its
    /// arguments, `watch` among them.
    pub fn wire(&self, state: &WindowState, watch: u64) -> (&'static str, serde_json::Map<String, Value>) {
        let (command, mut args) = match self {
            Call::Dispatch(handler) => ("dispatch_event", crate::program_wire::args::dispatch(state, handler)),
            Call::Answer(handler, values) => ("answer_event", crate::program_wire::args::answer(state, handler, values)),
        };
        args.insert("watch".into(), Value::from(watch));
        (command, args)
    }
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
    /// Where the person went, for Back and Forward (window/history.rs).
    history: RefCell<super::history::History>,
    state: RefCell<WindowState>,
    specs: BTreeMap<String, PartSpec>,
    changes: Cell<u64>,
    effects: RefCell<Vec<Effect>>,
    pending: RefCell<Option<Pending>>,
    /// The undo scope of the part that had the focus last, which `-> undo` and `-> redo` act on.
    focused_scope: RefCell<Option<String>>,
    facts: Arc<ModelFacts>,
    hears: RefCell<Vec<Hear>>,
    /// What the windows of the program share: the person's keys, whether a long call runs.
    running: RefCell<Option<Rc<Running>>>,
    /// What the window closes with (`close with value`), for the handler that asked for it.
    result: RefCell<Option<Value>>,
    /// The texts the handlers put in text values, and what each showed:
    /// shown again in another language while the value still shows it.
    shown: RefCell<BTreeMap<String, (Text, String)>>,
    /// The engines beside the screen its canvases run, by number.
    engines: crate::engines::Engines,
    /// What the program knows of its fields and types, read from its JSON once.
    program_facts: std::cell::OnceCell<crate::program_facts::ProgramFacts>,
}

/// The strategy a click target signs in with: `signin password`, or `signin(password)`.
fn signin_strategy(call: &HandlerCall) -> Option<String> {
    match (call.handler.as_str(), call.args.as_slice()) {
        ("signin", [strategy]) => Some(strategy.trim().to_string()),
        (handler, []) => handler.strip_prefix("signin ").map(|s| s.trim().to_string()).filter(|s| !s.is_empty() && !s.contains(' ')),
        _ => None,
    }
}

/// A translation's text with its braces written double shown single (`{{name}}` is `{name}`).
fn unbraced(text: &str) -> String {
    text.replace("{{", "{").replace("}}", "}")
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
        let facts = ModelFacts::of(files.model);
        ProgramHost { program, files, history: RefCell::new(super::history::History::default()), state: RefCell::new(state), specs, changes: Cell::new(0), effects: RefCell::new(Vec::new()), pending: RefCell::new(None), focused_scope: RefCell::new(None), facts, hears: RefCell::new(Vec::new()), running: RefCell::new(None), result: RefCell::new(None), shown: RefCell::new(BTreeMap::new()), engines: crate::engines::Engines::new(), program_facts: std::cell::OnceCell::new() }
    }

    /// The host shares what the program's windows share: the person's keys, and whether a long call runs.
    pub fn set_running(&self, running: Rc<Running>) {
        *self.running.borrow_mut() = Some(running);
    }

    fn running(&self) -> Option<Rc<Running>> {
        self.running.borrow().clone()
    }

    /// What the program knows of its fields and types (`Program::facts_json`), read once.
    fn program_facts(&self) -> &crate::program_facts::ProgramFacts {
        self.program_facts.get_or_init(|| crate::program_facts::ProgramFacts::read(&self.program.facts_json()))
    }

    /// The number of the open window, its own while it is open.
    pub fn instance(&self) -> u64 {
        self.state.borrow().instance
    }

    /// What the window closed with (`close with value`); taken once.
    pub fn take_result(&self) -> Option<Value> {
        self.result.borrow_mut().take()
    }

    /// Whether handlers run off the screen's thread, with the bar of a long call.
    fn long(&self) -> bool {
        self.running().is_some_and(|r| r.program.watches())
    }

    /// Whether a long call runs now: no other handler runs meanwhile.
    fn calling(&self) -> bool {
        self.running().is_some_and(|r| r.calling())
    }

    /// The texts the handlers put in text values show again, in the language
    /// shown now; a value changed since keeps what it is (C# SaidTextTests).
    pub fn reshow(&self) {
        let mut shown = self.shown.borrow_mut();
        let mut state = self.state.borrow_mut();
        shown.retain(|name, (text, was)| {
            if state.data.get(name.as_str()) != Some(&Value::from(was.as_str())) {
                return false;
            }
            *was = text.show();
            state.data.insert(name.clone(), Value::from(was.as_str()));
            true
        });
        drop(shown);
        drop(state);
        self.changed();
    }

    /// The answer to a question this window asked: the handler it goes to runs with `values`.
    pub fn answer(&self, handler: &str, values: Vec<Value>) {
        self.flush();
        if self.long() {
            self.effects.borrow_mut().push(Effect::Call(Call::Answer(handler.to_string(), values)));
            self.changed();
            return;
        }
        let state = self.state_now();
        self.answered(self.program.answer_event(state, handler, values));
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

    /// Where the edits of `part` are undone: the `undoScope` the project
    /// worked out for it (its form or panel, else itself); none for a part
    /// the markup gives none (an accordion, tabs, a list that only selects,
    /// one with `no undo`), so what the person does there is no edit. A part
    /// that is no part of the window's markup (code's own) is its own scope.
    pub fn scope(&self, part: &str) -> Option<String> {
        match self.specs.get(part) {
            None => Some(part.to_string()),
            Some(spec) if spec.flag("noUndo") => None,
            Some(spec) => spec.text("undoScope").filter(|s| !s.is_empty()).map(str::to_string),
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
            let showing = text.show();
            state.data.insert(name.clone(), Value::from(showing.as_str()));
            self.shown.borrow_mut().insert(name.clone(), (text.clone(), showing));
        }
        *self.state.borrow_mut() = state;
        self.hears.borrow_mut().extend(reply.hears);
        let mut effects = self.effects.borrow_mut();
        if !reply.messages.is_empty() {
            effects.push(Effect::Say(reply.messages));
        }
        if let Some(copy) = reply.copy {
            effects.push(Effect::Copy(copy.show()));
        }
        // A handler that closes its window and opens others closes first
        // (conformance/windows.json): what it opens goes on the window below.
        if reply.close {
            if reply.result.is_some() {
                *self.result.borrow_mut() = reply.result;
            }
            effects.push(Effect::Close);
        }
        for open in reply.open {
            effects.push(if open.new_window { Effect::OpenNew(open.window, open.with) } else { Effect::Open(open.window, open.with) });
        }
        for ask in reply.asks {
            effects.push(Effect::Ask(ask));
        }
        // The built-in targets run by code: the keys window opens, the program ends.
        if reply.keys {
            effects.push(Effect::Keys);
        }
        if reply.quit {
            effects.push(Effect::Quit);
        }
        drop(effects);
        self.changed();
    }

    /// Takes what a call of the program answered: its reply, or what went wrong as an error message. True when it answered a reply.
    pub fn answered(&self, answer: Answer<Reply>) -> bool {
        match answer {
            Ok(reply) => {
                let before = self.settings_record();
                self.apply(reply);
                // A handler that changed a setting keeps it, as a part's write does (every language alike).
                if before.is_some() && self.settings_record() != before {
                    self.keep_settings();
                }
                true
            }
            Err(text) => {
                self.effects.borrow_mut().push(Effect::Say(vec![Message { level: Level::Error, text }]));
                self.changed();
                false
            }
        }
    }

    /// Where the window is now: the chosen row of its lists, tables and trees, and the open section, tab or step of its containers.
    pub fn place_now(&self) -> super::history::Place {
        let state = self.state.borrow();
        let mut place = super::history::Place::default();
        for (name, spec) in &self.specs {
            match spec.kind.as_str() {
                "list" | "table" | "tree" | "treeTable" => {
                    if let Some(row) = state.selected.get(name) {
                        place.selected.insert(name.clone(), *row);
                    }
                }
                "accordion" | "tabs" | "steps" | "content" => {
                    if let Some(open) = state.parts.get(name).and_then(|p| p.open.or(p.tab)) {
                        place.opened.insert(name.clone(), open);
                    }
                }
                _ => {}
            }
        }
        place
    }

    /// An event of the person is done: the window's place is a step of its history.
    pub fn note_place(&self) {
        let place = self.place_now();
        self.history.borrow_mut().note(place);
    }

    /// Goes Back (or Forward) to the place before (or after): rows are chosen and containers opened as they were, and their handlers run as when the person did it.
    pub fn go(&self, back: bool) {
        let to = if back { self.history.borrow_mut().back() } else { self.history.borrow_mut().forward() };
        let Some(to) = to else { return };
        let now = self.place_now();
        let names: std::collections::BTreeSet<String> = to.selected.keys().chain(now.selected.keys()).cloned().collect();
        for name in names {
            let row = to.selected.get(&name).copied();
            if now.selected.get(&name).copied() != row {
                crate::parts::PartHost::select(self, &name, row);
                let calls: Vec<HandlerCall> = self.specs.get(&name).map(|s| s.on("select").cloned().collect()).unwrap_or_default();
                for call in calls {
                    self.run(Some(&name), &call);
                }
            }
        }
        for (name, open) in &to.opened {
            if now.opened.get(name) != Some(open) {
                let kind = self.specs.get(name).map(|s| s.kind.clone()).unwrap_or_default();
                crate::parts::change_state(self, name, |s| if kind == "tabs" { s.tab = Some(*open) } else { s.open = Some(*open) });
                let calls: Vec<HandlerCall> = self.specs.get(name).map(|s| s.on("change").cloned().collect()).unwrap_or_default();
                for call in calls {
                    self.run(Some(name), &call);
                }
            }
        }
        self.changed();
    }

    /// Whether Back (or Forward) has a place to go to.
    pub fn can_go(&self, back: bool) -> bool {
        let history = self.history.borrow();
        if back { history.can_go_back() } else { history.can_go_forward() }
    }

    /// The window's settings record without its own number; none when it has none.
    fn settings_record(&self) -> Option<Map<String, Value>> {
        let Some(Value::Object(mut now)) = self.state.borrow().data.get("settings").cloned() else { return None };
        now.remove("id");
        Some(now)
    }

    /// Keeps the settings the window holds over what the program kept (the
    /// person's keys among them), as the web screen keeps them; says why when it cannot.
    fn keep_settings(&self) {
        let Some(Value::Object(now)) = self.state.borrow().data.get("settings").cloned() else { return };
        let mut now = now;
        // The settings record's own number is no setting.
        now.remove("id");
        // Clarity's own settings show at once, however they changed (a handler, a bound dropdown): the language and the theme.
        if let Some(language) = now.get("language").and_then(Value::as_str).filter(|l| !l.is_empty() && *l != crate::choices::language()) {
            crate::choices::choose_language(language);
        }
        if let Some(theme) = now.get("theme").and_then(Value::as_str).filter(|t| !t.is_empty() && Some(*t) != crate::choices::theme().as_deref()) {
            crate::choices::choose_theme(theme);
        }
        let mut kept = self.program.read_settings();
        kept.extend(now);
        if let Err(text) = self.program.keep_settings(kept) {
            self.effects.borrow_mut().push(Effect::Say(vec![Message { level: Level::Error, text }]));
            self.changed();
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

    /// The part `part` has the focus: its undo scope is what `-> undo` and `-> redo` act on.
    pub fn set_focused(&self, part: &str) {
        if let Some(scope) = self.scope(part) {
            *self.focused_scope.borrow_mut() = Some(scope);
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

    /// Signs in with `strategy`, its inputs (as Clarity.Auth's catalog names
    /// them) the values of this window's parts of those names: a part bound
    /// to a field gives the field's value, an unbound one what was typed. A
    /// missing part gives no input, which the program refuses. A refusal
    /// shows as any refusal does.
    fn sign_in_by_names(&self, strategy: &str) -> bool {
        let names = clarity_auth::catalog().iter().find(|s| s.name == strategy).map(|s| s.inputs).unwrap_or_default();
        let mut inputs = Map::new();
        for name in names {
            if let Some(value) = self.input_value(name) {
                inputs.insert(name.to_string(), Value::String(value));
            }
        }
        match self.signed_in_with(strategy, inputs) {
            Ok(()) => true,
            Err(text) => self.answered(Err(text)),
        }
    }

    /// The one way a sign-in goes, by a `signin <strategy>` button or a
    /// sign-in part: the program checks `inputs`; on success the window
    /// closes and the main window opens (or the splash's rules hold), and a
    /// refusal is given back.
    fn signed_in_with(&self, strategy: &str, inputs: Map<String, Value>) -> Result<(), Text> {
        self.flush();
        self.program.signin(strategy, inputs)?;
        self.effects.borrow_mut().push(Effect::SignedIn);
        self.changed();
        Ok(())
    }

    /// The text of the part named `name`: its field's value when it is bound, else what was typed in it.
    fn input_value(&self, name: &str) -> Option<String> {
        let spec = self.specs.get(name)?;
        let state = self.state.borrow();
        match spec.text("bind").filter(|b| !b.is_empty()) {
            Some(path) => match paths::read(&state.data, path)? {
                Value::String(text) => Some(text),
                Value::Null => Some(String::new()),
                other => Some(other.to_string()),
            },
            None => Some(state.parts.get(name).and_then(|p| p.text.clone()).unwrap_or_default()),
        }
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
        if path.starts_with("settings.") {
            self.keep_settings();
        }
    }

    fn edited(&self, part: &str) {
        self.changed();
        self.edit_of(part);
    }

    fn state(&self, name: Option<&str>) -> Option<PartState> {
        let name = name?;
        Some(self.state.borrow().parts.get(name).cloned().unwrap_or_default())
    }

    fn set_state(&self, name: &str, state: PartState) {
        self.state.borrow_mut().parts.insert(name.to_string(), state);
        self.changed();
    }

    fn window_name(&self) -> Option<String> {
        Some(self.window())
    }

    fn signin(&self) -> Option<crate::facts::SigninFacts> {
        self.facts.signin().cloned()
    }

    fn sign_in(&self, strategy: &str, inputs: Map<String, Value>) -> Result<(), Text> {
        self.signed_in_with(strategy, inputs)
    }

    /// While a long call runs, no other handler can.
    fn can_run(&self, _part: Option<&str>, _call: &HandlerCall) -> bool {
        !self.calling()
    }

    fn run(&self, part: Option<&str>, call: &HandlerCall) -> bool {
        // While a long call runs, no other handler does.
        if self.calling() {
            return false;
        }
        // Clarity's own targets: the keys window, ending the program, closing the window.
        if call.args.is_empty() && matches!(call.event.as_str(), "click" | crate::parts::NEW_WINDOW_CLICK) {
            let own = match call.handler.as_str() {
                "keys" => Some(Effect::Keys),
                "quit" => Some(Effect::Quit),
                "close" => Some(Effect::Close),
                _ => None,
            };
            // Back and Forward go to the places the person was in this window.
            let went = match call.handler.as_str() {
                "back" => Some(true),
                "forward" => Some(false),
                _ => None,
            };
            if let Some(back) = went.filter(|_| self.files.window(&call.handler).is_none()) {
                self.flush();
                self.go(back);
                return true;
            }
            let undo = match call.handler.as_str() {
                "undo" => Some(true),
                "redo" => Some(false),
                _ => None,
            };
            if let Some(back) = undo.filter(|_| self.files.window(&call.handler).is_none()) {
                let scope = self.focused_scope.borrow().clone();
                match scope {
                    Some(scope) => self.undo(&scope, back),
                    None => self.flush(),
                }
                return true;
            }
            if let Some(own) = own.filter(|_| self.files.window(&call.handler).is_none()) {
                self.flush();
                self.effects.borrow_mut().push(own);
                self.changed();
                return true;
            }
        }
        // Signing in and out (docs/APP-CAPABILITIES-PLAN.md step 4), unless a window has that name.
        if matches!(call.event.as_str(), "click" | crate::parts::NEW_WINDOW_CLICK) && self.files.window(&call.handler).is_none() {
            if let Some(strategy) = signin_strategy(call) {
                return self.sign_in_by_names(&strategy);
            }
            if call.handler == "signout" && call.args.is_empty() {
                self.flush();
                return match self.program.signout() {
                    Ok(()) => {
                        self.effects.borrow_mut().push(Effect::SignedOut);
                        self.changed();
                        true
                    }
                    Err(text) => self.answered(Err(text)),
                };
            }
        }
        // A click that names a popover or slide-out of this window opens it (C#'s `Show_<popover>`).
        if call.args.is_empty() && self.specs.get(&call.handler).is_some_and(|spec| spec.kind == "popover" || spec.kind == "slideout") {
            crate::parts::change_state(self, &call.handler, |s| s.visible = Some(true));
            return true;
        }
        // A click that names a window opens it, filled with this window's data its route names (`with book`).
        if matches!(call.event.as_str(), "click" | crate::parts::NEW_WINDOW_CLICK) && self.files.window(&call.handler).is_some() {
            self.flush();
            let with: Map<String, Value> = {
                let state = self.state.borrow();
                call.args.iter().filter_map(|name| state.data.get(name).map(|v| (name.clone(), v.clone()))).collect()
            };
            let open = if call.event == crate::parts::NEW_WINDOW_CLICK { Effect::OpenNew(call.handler.clone(), with) } else { Effect::Open(call.handler.clone(), with) };
            self.effects.borrow_mut().push(open);
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
        // Another event of the part whose edit waits (a table chooses the row
        // of the cell it wrote before its change runs) goes on that edit: its
        // handler runs as part of it, and the edit ends with the change, so
        // Ctrl+Z takes back the value and what the handlers did as one.
        let waiting = self.pending.borrow().clone();
        if let Some(waiting) = waiting.filter(|w| Some(w.part.as_str()) == part) {
            let readonly = self.read_only(&waiting.part);
            let edit = Edit { handler: Some(call.written()), part: waiting.part, scope: waiting.scope, ends: false, readonly, path: None, editable: None };
            let state = self.state_now();
            return self.answered(self.program.edit_event(state, edit));
        }
        self.flush();
        // A program that watches its long calls runs a click's handler off
        // the screen's thread. Every other event runs in place: a part may
        // wait for its answer (a wizard's Next refused), and a timer's tick
        // run apart would bring back a window older than what was typed since.
        if self.long() && call.event == "click" {
            self.effects.borrow_mut().push(Effect::Call(Call::Dispatch(call.written())));
            self.changed();
            return true;
        }
        let state = self.state_now();
        self.answered(self.program.dispatch_event(state, &call.written()))
    }

    /// A written text as shown: a translation's key (`@hello`) in the
    /// person's language, its braces written double shown single; plain text as written.
    fn text(&self, written: Option<&str>) -> String {
        match written {
            None => String::new(),
            Some(w) => match w.strip_prefix('@') {
                Some(key) => unbraced(&clarity_runtime::i18n::value(key).unwrap_or_else(|| key.to_string())),
                None => w.to_string(),
            },
        }
    }

    /// An address a link opens goes to the host's opener (the system's program for it on the desktop).
    fn open_address(&self, address: &str) {
        super::address::open_with_host(address);
    }

    fn resource(&self, name: &str) -> Option<Vec<u8>> {
        self.files.resource(name).map(<[u8]>::to_vec)
    }

    fn icon_path(&self, name: &str) -> Option<String> {
        self.facts.icon(name).map(str::to_string)
    }

    fn changes(&self) -> u64 {
        self.changes.get()
    }

    fn say(&self, level: Level, text: Text) {
        self.effects.borrow_mut().push(Effect::Say(vec![Message { level, text }]));
        self.changed();
    }

    /// The type as the generated program's facts give it (`list<Order>`, `money`, an enum's name), else as its model says.
    fn type_of(&self, path: &str) -> Option<String> {
        let window = self.window();
        self.program_facts().type_of(&window, path).map(str::to_string).or_else(|| self.facts.type_of(&window, path))
    }

    /// The field's facts as the generated program carries them, else as its model says.
    fn field(&self, path: &str) -> Option<FieldFacts> {
        let window = self.window();
        let model = self.facts.field(&window, path);
        match self.program_facts().field(&window, path) {
            // The program's facts carry no help; the model's do.
            Some(mut facts) => {
                facts.help = model.and_then(|m| m.help);
                Some(facts)
            }
            None => model,
        }
    }

    fn formats(&self) -> Formats {
        let mut formats = self.facts.formats();
        formats.language = Some(crate::choices::language());
        formats
    }

    /// An enum's values, the program's languages or themes for the settings
    /// that pick them, or the records a reference points at.
    fn choices(&self, path: &str) -> Vec<Choice> {
        let window = self.window();
        // A value of the window's own of an enum offers the enum's values, as an enum field does.
        if let Some(name) = self.facts.value_enum(&window, path) {
            let labelled = self.facts.enum_choices(name);
            return if labelled.is_empty() { self.program_facts().enum_values(name).iter().map(|v| Choice { value: Value::from(v.as_str()), label: v.clone(), cells: None }).collect() } else { labelled };
        }
        let Some((_, field)) = self.facts.field_at(&window, path) else { return Vec::new() };
        let offers = |offers: Vec<crate::choices::Offer>| offers.into_iter().map(|o| Choice { value: Value::from(o.name), label: o.label, cells: None }).collect();
        // A field of an enum, or a list of an enum's values, offers the enum's values.
        let enum_name = match field.kind.as_str() {
            "enum" => field.names.as_deref(),
            "values" => field.item.as_deref().filter(|item| self.facts.is_enum(item)),
            _ => None,
        };
        match if enum_name.is_some() { "enum" } else { field.kind.as_str() } {
            "enum" => {
                let Some(name) = enum_name else { return Vec::new() };
                let labelled = self.facts.enum_choices(name);
                if !labelled.is_empty() {
                    return labelled;
                }
                self.program_facts().enum_values(name).iter().map(|v| Choice { value: Value::from(v.as_str()), label: v.clone(), cells: None }).collect()
            }
            "language" => offers(crate::choices::languages()),
            "theme" => offers(crate::choices::themes()),
            "ref" => {
                let Some(entity) = field.names.clone() else { return Vec::new() };
                match self.program.ref_options(&entity) {
                    Ok(options) => options.into_iter().map(|o| Choice { value: Value::from(o.id), label: o.text, cells: None }).collect(),
                    Err(text) => {
                        self.say(Level::Error, text);
                        Vec::new()
                    }
                }
            }
            _ => Vec::new(),
        }
    }

    /// A typed dropdown's field values, a search dropdown's records with the fields it shows; else its choices.
    fn offered(&self, part: &PartSpec, path: &str) -> Vec<Choice> {
        let window = self.window();
        let at = self.facts.field_at(&window, path).map(|(entity, field)| (entity, field.facts.refers.clone()));
        let field_name = crate::facts::without_places(path).rsplit('.').next().unwrap_or_default().to_string();
        if part.flag("typed") {
            if let Some((entity, _)) = &at {
                return match self.program.field_values(entity, &field_name) {
                    Ok(values) => values.into_iter().map(|v| Choice { value: Value::from(v.clone()), label: v, cells: None }).collect(),
                    Err(text) => {
                        self.say(Level::Error, text);
                        Vec::new()
                    }
                };
            }
        }
        let show = part.names("show");
        if part.flag("search") && !show.is_empty() {
            if let Some((_, Some(target))) = &at {
                return match self.program.find_options("search", target, None, &show, "") {
                    Ok(Value::Array(found)) => found
                        .into_iter()
                        .map(|o| {
                            let fields = o.get("fields").cloned().unwrap_or(Value::Null);
                            let cells = show.iter().map(|f| match fields.get(f) { Some(Value::String(s)) => s.clone(), Some(Value::Null) | None => String::new(), Some(v) => v.to_string() }).collect();
                            Choice { value: o.get("id").cloned().unwrap_or(Value::Null), label: o.get("text").and_then(Value::as_str).unwrap_or_default().to_string(), cells: Some(cells) }
                        })
                        .collect(),
                    Ok(_) => Vec::new(),
                    Err(text) => {
                        self.say(Level::Error, text);
                        Vec::new()
                    }
                };
            }
        }
        self.choices(path)
    }

    fn fetch(&self, bind: &str, store_path: Option<&str>) -> Option<Vec<u8>> {
        // A file of a file store by its path, else the stored field of the record the path is in.
        if let Some(path) = store_path.filter(|p| !p.is_empty()) {
            let window = self.window();
            let store = self.facts.field_at(&window, bind).and_then(|(entity, _)| {
                let field = crate::facts::without_places(bind).rsplit('.').next().unwrap_or_default().to_string();
                self.facts.store_of(&entity, &field)
            });
            if let Some(store) = store {
                return self.program.store_file(&store, path).ok();
            }
        }
        let window = self.window();
        let (entity, _) = self.facts.field_at(&window, bind)?;
        let (owner, field) = bind.rsplit_once('.')?;
        let id = self.read(owner)?.get("id")?.as_i64()?;
        self.program.field_bytes(&entity, id, field).ok().flatten()
    }

    fn selected(&self, part: &str) -> Option<usize> {
        self.state.borrow().selected.get(part).copied()
    }

    fn select(&self, part: &str, place: Option<usize>) {
        let mut state = self.state.borrow_mut();
        let was = state.selected.get(part).copied();
        if was == place {
            return;
        }
        match place {
            Some(place) => state.selected.insert(part.to_string(), place),
            None => state.selected.remove(part),
        };
        drop(state);
        self.changed();
    }

    fn hears(&self, part: &str) -> Vec<Hear> {
        let mut hears = self.hears.borrow_mut();
        let (theirs, others): (Vec<Hear>, Vec<Hear>) = hears.drain(..).partition(|h| h.part == part);
        *hears = others;
        theirs
    }

    /// The canvas's handler of `event` runs with where the pointer was, kept
    /// in the part's state; a drag, a drop, a bend, a new line or a resize is one edit
    /// of its undo scope, as a change is. None when the canvas has no handler for it.
    fn pointed(&self, part: &str, event: &str, at: &Map<String, Value>) -> Option<bool> {
        let call = self.specs.get(part)?.on(event).next()?.clone();
        self.state.borrow_mut().parts.entry(part.to_string()).or_default().pointer = Some(at.clone());
        self.changed();
        if self.calling() {
            return Some(false);
        }
        if matches!(event, "drag" | "drop" | "bend" | "connect" | "resize") {
            if let Some(scope) = self.scope(part) {
                let waiting = self.pending.borrow_mut().take();
                if let Some(other) = waiting.filter(|w| w.part != part) {
                    self.send_edit(other, None);
                }
                return Some(self.send_edit(Pending { part: part.to_string(), scope }, Some(call.written())));
            }
        }
        Some(self.run(Some(part), &call))
    }

    /// What the person did on a canvas with a mode (objects, paint, vector),
    /// applied by the program as one edit, its `on change` with it.
    fn acted(&self, part: &str, action: &Map<String, Value>) -> Option<bool> {
        let spec = self.specs.get(part)?;
        let mode = spec.json("mode").and_then(Value::as_str).filter(|m| !m.is_empty())?.to_string();
        if self.calling() {
            return Some(false);
        }
        let shown = self.state.borrow().parts.get(part).cloned().unwrap_or_default();
        let grid = shown.grid.or_else(|| spec.number("grid").map(|g| g as i64)).unwrap_or(0);
        let snap = shown.snap.unwrap_or_else(|| spec.flag("snap"));
        let readonly = self.read_only(part);
        let text_of = |v: &Value, field: &str| v.get(field).and_then(Value::as_str).unwrap_or_default().to_string();
        let mut canvas = CanvasArgs { part: part.to_string(), mode: mode.clone(), list: String::new(), at: None, path: String::new(), units: None, grid, snap, readonly };
        match (mode.as_str(), spec.json("objects")) {
            ("objects", Some(objects)) => {
                let objects = match objects {
                    Value::String(text) => serde_json::from_str(text).unwrap_or(Value::Null),
                    other => other.clone(),
                };
                canvas.list = text_of(&objects, "list");
                canvas.at = Some([text_of(&objects, "x"), text_of(&objects, "y")]);
            }
            _ => {
                canvas.path = spec.text("bind").unwrap_or_default().to_string();
                let units = match spec.json("units") {
                    Some(Value::String(text)) => serde_json::from_str(text).unwrap_or(Value::Null),
                    Some(other) => other.clone(),
                    None => Value::Null,
                };
                canvas.units = units.get("width").and_then(Value::as_i64).zip(units.get("height").and_then(Value::as_i64)).map(|(w, h)| [w, h]);
            }
        }
        let handler = spec.on("change").next().map(HandlerCall::written);
        self.flush();
        let edit = CanvasEdit { canvas, action: action.clone(), scope: self.scope(part), handler, readonly };
        let state = self.state_now();
        Some(self.answered(self.program.canvas_event(state, edit)))
    }

    /// A custom part or a canvas's engine told `tell`: the handler wired to it on the part's line runs with the values.
    fn told(&self, part: &str, tell: &str, values: &[Value]) {
        let Some(spec) = self.specs.get(part) else { return };
        let wired = match spec.json("tells") {
            Some(Value::Array(items)) => items.clone(),
            Some(Value::String(text)) => serde_json::from_str::<Vec<Value>>(text).unwrap_or_default(),
            _ => Vec::new(),
        };
        let Some(handler) = wired.iter().find(|w| w.get("tell").and_then(Value::as_str) == Some(tell)).and_then(|w| w.get("handler")).and_then(Value::as_str) else { return };
        if self.calling() {
            return;
        }
        self.flush();
        let state = self.state_now();
        self.answered(self.program.answer_event(state, handler, values.to_vec()));
    }

    /// A new instance of the program's engine `name` for each canvas that runs it, its frames on a thread of their own.
    fn engine_start(&self, name: &str) -> Option<Result<u64, String>> {
        Some(crate::engines::ProgramFrames::open(self.program.clone(), name).map(|frames| self.engines.start(Box::new(frames))))
    }

    fn engine_hears(&self, id: u64) -> crate::parts::data::canvas::engine::Hears {
        self.engines.hears(id)
    }

    fn engine_frame(&self, id: u64, ms: f64, input: Vec<crate::parts::data::canvas::engine::ScreenInput>) {
        self.engines.frame(id, ms, input)
    }

    fn engine_drawn(&self, id: u64) -> Option<Result<crate::parts::data::canvas::engine::Drawn, String>> {
        self.engines.drawn(id)
    }

    fn engine_stop(&self, id: u64) {
        self.engines.stop(id)
    }

    /// The person's keys where they set their own and the project lets them, else the design's.
    fn keys_of(&self, command: &str, designed: Option<&str>) -> Vec<String> {
        match self.running() {
            Some(running) if self.facts.keys_editable() => running.keys_of(command, designed),
            _ => designed.filter(|d| !d.is_empty()).map(|d| vec![d.to_string()]).unwrap_or_default(),
        }
    }

    fn bound_of(&self, part: &str) -> Option<String> {
        self.specs.get(part).and_then(|s| s.text("bind")).filter(|b| !b.is_empty()).map(str::to_string)
    }
}
