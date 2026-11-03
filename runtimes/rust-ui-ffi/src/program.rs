//! The program as callbacks (`clarity_runtime::program::Program`): every
//! call becomes a command with its arguments as JSON, which the caller's
//! program answers `{"value": ...}` or `{"error": Text}` (see the header).
//! A command it does not answer takes the trait's default where there is
//! one, else says the program did not answer.

use std::ffi::{c_char, c_void};
use std::sync::Arc;

use base64::Engine as _;
use clarity_runtime::program::{Answer, CanvasEdit, Edit, ItemEdit, Program, RangeEdit, Reorder, StepPress, Started};
use clarity_runtime::screen::{Drawn, Hears, ScreenInput};
use clarity_runtime::ui::{RefOption, Reply, Text, WindowState};
use serde::de::DeserializeOwned;
use serde_json::{json, Map, Value};

use crate::{guard, put, wire, ClarityAnswer, ClarityStatus, Failure};

/// The caller's program: its pointer and the function it answers by.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ClarityProgramCallbacks {
    pub user: *mut c_void,
    pub call: Option<unsafe extern "C" fn(user: *mut c_void, command: *const c_char, command_len: usize, args: *const c_char, args_len: usize, answer: *mut ClarityAnswer)>,
    pub release: Option<unsafe extern "C" fn(user: *mut c_void)>,
}

/// The program behind the callbacks.
pub struct CallbackProgram {
    callbacks: ClarityProgramCallbacks,
}

// The header asks the caller's `call` to be safe from any thread.
unsafe impl Send for CallbackProgram {}
unsafe impl Sync for CallbackProgram {}

impl Drop for CallbackProgram {
    fn drop(&mut self) {
        if let Some(release) = self.callbacks.release {
            crate::quietly(|| unsafe { release(self.callbacks.user) });
        }
    }
}

/// The handle the caller holds: one hold of the program.
pub struct ClarityProgram {
    pub program: Arc<CallbackProgram>,
}

impl CallbackProgram {
    pub fn new(callbacks: ClarityProgramCallbacks) -> CallbackProgram {
        CallbackProgram { callbacks }
    }

    /// Asks the program: its whole answer as JSON; none when it set none, or it does not read.
    pub fn ask(&self, command: &str, args: Map<String, Value>) -> Option<Value> {
        let call = self.callbacks.call?;
        let args = Value::Object(args).to_string();
        let mut answer = ClarityAnswer::default();
        unsafe { call(self.callbacks.user, command.as_ptr() as *const c_char, command.len(), args.as_ptr() as *const c_char, args.len(), &mut answer) };
        answer.json()
    }

    /// Asks the program for a value: what it answers, or what went wrong as the screen shows it.
    pub fn answer<T: DeserializeOwned>(&self, command: &str, args: Map<String, Value>) -> Answer<T> {
        Self::read(command, self.ask(command, args))
    }

    /// Reads what the program answered to `command`.
    fn read<T: DeserializeOwned>(command: &str, answered: Option<Value>) -> Answer<T> {
        let Some(mut answered) = answered else {
            return Err(Text::plain(format!("The program did not answer {command}.")));
        };
        if let Some(error) = answered.get_mut("error").map(Value::take).filter(|e| !e.is_null()) {
            return Err(serde_json::from_value::<Text>(error.clone()).unwrap_or_else(|_| Text::plain(error.to_string())));
        }
        let value = answered.get_mut("value").map(Value::take).unwrap_or(Value::Null);
        serde_json::from_value::<T>(value).map_err(|e| Text::plain(format!("The program's answer to {command} does not read: {e}")))
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

impl Program for CallbackProgram {
    fn started(&self) -> Started {
        wire::started(&self.maybe("started", Map::new()).unwrap_or(Value::Null))
    }

    fn start_app(&self) -> Answer<Reply> {
        self.answer("start_app", Map::new())
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
        self.answer("dispatch_event", args(json!({ "state": state, "handler": handler })))
    }

    fn edit_event(&self, state: WindowState, edit: Edit) -> Answer<Reply> {
        self.answer("edit_event", wire::edit(&state, &edit))
    }

    fn undo_edit(&self, state: WindowState, scope: &str, back: bool) -> Answer<Reply> {
        self.answer("undo_edit", args(json!({ "state": state, "scope": scope, "back": back })))
    }

    fn reorder_event(&self, state: WindowState, reorder: Reorder) -> Answer<Reply> {
        self.answer("reorder_event", wire::reorder(&state, &reorder))
    }

    fn item_event(&self, state: WindowState, item: ItemEdit) -> Answer<Reply> {
        self.answer("item_event", wire::item(&state, &item))
    }

    fn range_event(&self, state: WindowState, range: RangeEdit) -> Answer<Reply> {
        self.answer("range_event", wire::range(&state, &range))
    }

    fn canvas_event(&self, state: WindowState, canvas: CanvasEdit) -> Answer<Reply> {
        self.answer("canvas_event", wire::canvas(&state, &canvas))
    }

    fn step_event(&self, state: WindowState, step: StepPress) -> Answer<Reply> {
        self.answer("step_event", wire::step(&state, &step))
    }

    fn view_event(&self, state: WindowState, part: &str, action: Map<String, Value>) -> Answer<Reply> {
        self.answer("view_event", args(json!({ "state": state, "part": part, "action": action })))
    }

    fn answer_event(&self, state: WindowState, handler: &str, values: Vec<Value>) -> Answer<Reply> {
        self.answer("answer_event", args(json!({ "state": state, "handler": handler, "values": values })))
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
            answered => Self::read::<Value>("keep_settings", answered).map(|_| ()),
        }
    }

    fn connect(&self) -> Answer<()> {
        match self.ask("connect", Map::new()) {
            None => Ok(()),
            answered => Self::read::<Value>("connect", answered).map(|_| ()),
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
        self.maybe("engine_hears", args(json!({ "id": id }))).map(|v| wire::hears(&v)).unwrap_or_default()
    }

    fn engine_frame(&self, id: u64, ms: f64, input: Vec<ScreenInput>) -> Result<Drawn, String> {
        let input: Vec<Value> = input.iter().map(wire::screen_input).collect();
        match self.answer::<Value>("engine_frame", args(json!({ "id": id, "ms": ms, "input": input }))) {
            Ok(drawn) => Ok(wire::drawn(&drawn)),
            Err(why) => Err(why.show()),
        }
    }

    fn engine_close(&self, id: u64) {
        self.tell("engine_close", args(json!({ "id": id })));
    }
}

/// A program over the caller's callbacks.
///
/// # Safety
/// `callbacks` points at a filled table; `out` at writable memory for a handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_program_new(callbacks: *const ClarityProgramCallbacks, out: *mut *mut ClarityProgram) -> ClarityStatus {
    guard(|| {
        let callbacks = *unsafe { crate::handle(callbacks) }?;
        if callbacks.call.is_none() {
            return Err(Failure::invalid("the program has no call"));
        }
        let program = ClarityProgram { program: Arc::new(CallbackProgram::new(callbacks)) };
        unsafe { put(out, Box::into_raw(Box::new(program))) }
    })
}

/// Lets go of the caller's hold of a program.
///
/// # Safety
/// `program` is NULL or a handle from `clarity_program_new`, not freed yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_program_free(program: *mut ClarityProgram) {
    if !program.is_null() {
        crate::quietly(|| drop(unsafe { Box::from_raw(program) }));
    }
}
