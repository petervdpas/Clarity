//! A program answered over the web server's wire (DESIGN 7.3 and 7.9):
//! every call of `clarity_runtime::program::Program` becomes a command with
//! its arguments as JSON, which the program answers `{"value": ...}` or
//! `{"error": Text}`. The C interface asks through its callbacks, the
//! browser host through the page's server. A command left unanswered takes
//! the trait's default where there is one, else says the program did not
//! answer.

pub mod args;

use base64::Engine as _;
use clarity_runtime::program::{Answer, CanvasEdit, Edit, ItemEdit, Program, RangeEdit, Reorder, StepPress, Started};
use clarity_runtime::screen::{Drawn, Hears, ScreenInput};
use clarity_runtime::ui::{RefOption, Reply, Text, WindowState};
use serde::de::DeserializeOwned;
use serde_json::{json, Map, Value};

/// What asks the program a command: its whole answer as JSON, none when it answered nothing.
pub trait Asks: Send + Sync {
    fn ask(&self, command: &str, args: Map<String, Value>) -> Option<Value>;
}

/// A program behind the wire, asked through `A`.
pub struct WireProgram<A> {
    asks: A,
}

impl<A: Asks> WireProgram<A> {
    pub fn new(asks: A) -> WireProgram<A> {
        WireProgram { asks }
    }

    /// What it asks through.
    pub fn asks(&self) -> &A {
        &self.asks
    }

    /// Asks the program: its whole answer as JSON; none when it answered none.
    pub fn ask(&self, command: &str, args: Map<String, Value>) -> Option<Value> {
        self.asks.ask(command, args)
    }

    /// Asks the program for a value: what it answers, or what went wrong as the screen shows it.
    pub fn answer<T: DeserializeOwned>(&self, command: &str, args: Map<String, Value>) -> Answer<T> {
        read(command, self.ask(command, args))
    }

    /// Asks the program for a value it may leave unanswered: none then.
    fn maybe(&self, command: &str, args: Map<String, Value>) -> Option<Value> {
        let answered = self.ask(command, args)?;
        if answered.get("error").is_some_and(|e| !e.is_null()) {
            return None;
        }
        answered.get("value").cloned()
    }

    /// Tells the program something it answers nothing to.
    fn tell(&self, command: &str, args: Map<String, Value>) {
        let _ = self.ask(command, args);
    }
}

thread_local! {
    /// The agent's asker while a program runs a command for it on this thread (`agent_call`).
    static AGENT_ASK: std::cell::Cell<Option<*mut (dyn FnMut(&str, &str) -> Option<String> + 'static)>> = const { std::cell::Cell::new(None) };
}

/// Runs `run` with `ask` as the agent's asker on this thread: a program
/// across the wire asks the agent with `ask_agent` (the C interface's
/// `clarity_agent_ask`) while `run` runs.
pub fn asking_agent<R>(ask: &mut dyn FnMut(&str, &str) -> Option<String>, run: impl FnOnce() -> R) -> R {
    // SAFETY: the pointer is only used while `run` runs, while `ask` lives.
    let erased: *mut (dyn FnMut(&str, &str) -> Option<String> + '_) = ask;
    let erased: *mut (dyn FnMut(&str, &str) -> Option<String> + 'static) = unsafe { std::mem::transmute(erased) };
    let before = AGENT_ASK.with(|a| a.replace(Some(erased)));
    struct Restore(Option<*mut (dyn FnMut(&str, &str) -> Option<String> + 'static)>);
    impl Drop for Restore {
        fn drop(&mut self) {
            AGENT_ASK.with(|a| a.set(self.0));
        }
    }
    let _restore = Restore(before);
    run()
}

/// Asks the agent whose command runs on this thread `question` of the kind
/// `form` (JSON): the answer as the terminal gives it; none when it gives
/// none, or no command of an agent runs here.
pub fn ask_agent(question: &str, form: &str) -> Option<String> {
    let ask = AGENT_ASK.with(|a| a.get())?;
    // SAFETY: set by `asking_agent` for as long as its `ask` lives on this thread.
    unsafe { (*ask)(question, form) }
}

/// Reads what the program answered to `command`.
pub fn read<T: DeserializeOwned>(command: &str, answered: Option<Value>) -> Answer<T> {
    let Some(mut answered) = answered else {
        return Err(Text::plain(format!("The program did not answer {command}.")));
    };
    if let Some(error) = answered.get_mut("error").map(Value::take).filter(|e| !e.is_null()) {
        return Err(serde_json::from_value::<Text>(error.clone()).unwrap_or_else(|_| Text::plain(error.as_str().map(str::to_string).unwrap_or_else(|| error.to_string()))));
    }
    let value = answered.get_mut("value").map(Value::take).unwrap_or(Value::Null);
    serde_json::from_value::<T>(value).map_err(|e| Text::plain(format!("The program's answer to {command} does not read: {e}")))
}

fn args(value: Value) -> Map<String, Value> {
    match value {
        Value::Object(map) => map,
        _ => Map::new(),
    }
}

fn with_state(state: &WindowState) -> Map<String, Value> {
    args(json!({ "state": state }))
}

fn base64_of(value: Option<String>) -> Answer<Option<Vec<u8>>> {
    match value {
        None => Ok(None),
        Some(text) => base64::engine::general_purpose::STANDARD.decode(text.trim()).map(Some).map_err(|e| Text::plain(format!("The program's bytes do not read: {e}"))),
    }
}

impl<A: Asks + 'static> Program for WireProgram<A> {
    fn started(&self) -> Started {
        args::started(&self.maybe("started", Map::new()).unwrap_or(Value::Null))
    }

    fn start_app(&self) -> Answer<Reply> {
        self.answer("start_app", Map::new())
    }

    fn agents(&self) -> Option<clarity_runtime::program::AgentsOffered> {
        serde_json::from_value(self.maybe("agents", Map::new())?).ok()
    }

    fn agent_call(&self, line: &[String], ask: &mut dyn FnMut(&str, &str) -> Option<String>) -> clarity_runtime::program::AgentRan {
        let answered = asking_agent(ask, || self.answer::<clarity_runtime::program::AgentRan>("agent_call", args(json!({ "line": line }))));
        answered.unwrap_or_else(|why| clarity_runtime::program::AgentRan { text: why.show(), refused: true, step: 0 })
    }

    fn agent_undo(&self, step: u64) -> Answer<()> {
        self.answer::<Value>("agent_undo", args(json!({ "step": step }))).map(|_| ())
    }

    fn open_window(&self, window: &str, with: Map<String, Value>) -> Answer<Reply> {
        self.answer("open_window", args(json!({ "window": window, "with": with })))
    }

    fn close_window(&self, state: WindowState) -> Answer<Reply> {
        self.answer("close_window", with_state(&state))
    }

    fn return_to(&self, state: WindowState) -> Answer<Reply> {
        self.answer("return_to", with_state(&state))
    }

    fn dispatch_event(&self, state: WindowState, handler: &str) -> Answer<Reply> {
        self.answer("dispatch_event", args::dispatch(&state, handler))
    }

    fn edit_event(&self, state: WindowState, edit: Edit) -> Answer<Reply> {
        self.answer("edit_event", args::edit(&state, &edit))
    }

    fn undo_edit(&self, state: WindowState, scope: &str, back: bool) -> Answer<Reply> {
        self.answer("undo_edit", args(json!({ "state": state, "scope": scope, "back": back })))
    }

    fn reorder_event(&self, state: WindowState, reorder: Reorder) -> Answer<Reply> {
        self.answer("reorder_event", args::reorder(&state, &reorder))
    }

    fn item_event(&self, state: WindowState, item: ItemEdit) -> Answer<Reply> {
        self.answer("item_event", args::item(&state, &item))
    }

    fn range_event(&self, state: WindowState, range: RangeEdit) -> Answer<Reply> {
        self.answer("range_event", args::range(&state, &range))
    }

    fn canvas_event(&self, state: WindowState, canvas: CanvasEdit) -> Answer<Reply> {
        self.answer("canvas_event", args::canvas(&state, &canvas))
    }

    fn step_event(&self, state: WindowState, step: StepPress) -> Answer<Reply> {
        self.answer("step_event", args::step(&state, &step))
    }

    fn view_event(&self, state: WindowState, part: &str, action: Map<String, Value>) -> Answer<Reply> {
        self.answer("view_event", args(json!({ "state": state, "part": part, "action": action })))
    }

    fn answer_event(&self, state: WindowState, handler: &str, values: Vec<Value>) -> Answer<Reply> {
        self.answer("answer_event", args::answer(&state, handler, &values))
    }

    fn ref_options(&self, entity: &str) -> Answer<Vec<RefOption>> {
        self.answer("ref_options", args(json!({ "entity": entity })))
    }

    fn field_values(&self, entity: &str, field: &str) -> Answer<Vec<String>> {
        self.answer("field_values", args(json!({ "entity": entity, "field": field })))
    }

    fn find_options(&self, kind: &str, entity: &str, field: Option<&str>, show: &[String], typed: &str) -> Answer<Value> {
        self.answer("find_options", args(json!({ "kind": kind, "entity": entity, "field": field, "show": show, "typed": typed })))
    }

    fn field_bytes(&self, entity: &str, id: i64, field: &str) -> Answer<Option<Vec<u8>>> {
        base64_of(self.answer::<Option<String>>("field_bytes", args(json!({ "entity": entity, "id": id, "field": field })))?)
    }

    fn store_file(&self, store: &str, path: &str) -> Answer<Vec<u8>> {
        base64_of(Some(self.answer::<String>("store_file", args(json!({ "store": store, "path": path })))?)).map(Option::unwrap_or_default)
    }

    fn read_settings(&self) -> Map<String, Value> {
        match self.maybe("read_settings", Map::new()) {
            Some(Value::Object(settings)) => settings,
            _ => Map::new(),
        }
    }

    fn keep_settings(&self, settings: Map<String, Value>) -> Answer<()> {
        // A program that keeps no settings answers nothing: they are kept for this run only.
        match self.ask("keep_settings", args(json!({ "settings": settings }))) {
            None => Ok(()),
            answered => read::<Value>("keep_settings", answered).map(|_| ()),
        }
    }

    fn connect(&self) -> Answer<()> {
        match self.ask("connect", Map::new()) {
            None => Ok(()),
            answered => read::<Value>("connect", answered).map(|_| ()),
        }
    }

    fn signin(&self, strategy: &str, inputs: Map<String, Value>) -> Answer<()> {
        read::<Value>("signin", self.ask("signin", args(json!({ "strategy": strategy, "inputs": inputs })))).map(|_| ())
    }

    fn signout(&self) -> Answer<()> {
        // A program whose people do not sign in answers nothing: nobody was signed in.
        match self.ask("signout", Map::new()) {
            None => Ok(()),
            answered => read::<Value>("signout", answered).map(|_| ()),
        }
    }

    fn set_language(&self, language: Option<&str>) {
        // The library's own texts follow the language as well as the program's.
        clarity_runtime::i18n::set_language(language);
        self.tell("set_language", args(json!({ "language": language })));
    }

    fn watches(&self) -> bool {
        self.maybe("watches", Map::new()).and_then(|v| v.as_bool()).unwrap_or(false)
    }

    fn start_watch(&self, id: u64) {
        self.tell("start_watch", args(json!({ "id": id })));
    }

    fn splash_status(&self) -> Option<Text> {
        serde_json::from_value(self.maybe("splash_status", Map::new())?).ok()
    }

    fn watch_progress(&self, id: u64) -> Option<(i64, i64)> {
        let progress = self.maybe("watch_progress", args(json!({ "id": id })))?;
        let pair = progress.as_array()?;
        Some((pair.first()?.as_i64()?, pair.get(1)?.as_i64()?))
    }

    fn stop_watch(&self, id: u64) {
        self.tell("stop_watch", args(json!({ "id": id })));
    }

    fn end_watch(&self, id: u64) {
        self.tell("end_watch", args(json!({ "id": id })));
    }

    fn facts_json(&self) -> Value {
        self.maybe("facts", Map::new()).unwrap_or(Value::Null)
    }

    fn engine_open(&self, name: &str) -> Option<Result<u64, String>> {
        let answered = self.ask("engine_open", args(json!({ "name": name })))?;
        if let Some(error) = answered.get("error").filter(|e| !e.is_null()) {
            let why = serde_json::from_value::<Text>(error.clone()).map(|t| t.show()).unwrap_or_else(|_| error.to_string());
            return Some(Err(why));
        }
        answered.get("value").and_then(Value::as_u64).map(Ok)
    }

    fn engine_hears(&self, id: u64) -> Hears {
        self.maybe("engine_hears", args(json!({ "id": id }))).map(|v| args::hears(&v)).unwrap_or_default()
    }

    fn engine_frame(&self, id: u64, ms: f64, input: Vec<ScreenInput>) -> Result<Drawn, String> {
        let input: Vec<Value> = input.iter().map(args::screen_input).collect();
        match self.answer::<Value>("engine_frame", args(json!({ "id": id, "ms": ms, "input": input }))) {
            Ok(drawn) => Ok(args::drawn(&drawn)),
            Err(why) => Err(why.show()),
        }
    }

    fn engine_close(&self, id: u64) {
        self.tell("engine_close", args(json!({ "id": id })));
    }
}
