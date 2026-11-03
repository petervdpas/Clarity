//! The part host as callbacks (`ui::parts::PartHost`): what a
//! window's parts read and change, answered by the caller's own window model
//! (C#'s ViewModelHost). Values cross as JSON; a callback left NULL takes
//! the trait's default.

use std::ffi::{c_char, c_void};

use clarity_runtime::ui::{Hear, Level, Message, PartState, Text};
use ui::markup::{HandlerCall, PartSpec};
use ui::parts::data::canvas::engine::{Drawn, Hears, ScreenInput};
use ui::parts::{Choice, FieldFacts, Formats, PartHost};
use serde_json::{json, Map, Value};

use crate::{wire, ClarityAnswer};

type CText = *const c_char;
type CAnswer = *mut ClarityAnswer;

/// The caller's part host: its pointer and its functions.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ClarityPartHostCallbacks {
    pub user: *mut c_void,
    pub read: Option<unsafe extern "C" fn(*mut c_void, CText, usize, CAnswer)>,
    pub rows: Option<unsafe extern "C" fn(*mut c_void, CText, usize, CAnswer)>,
    pub write: Option<unsafe extern "C" fn(*mut c_void, CText, usize, CText, usize, CText, usize)>,
    pub edited: Option<unsafe extern "C" fn(*mut c_void, CText, usize)>,
    pub state: Option<unsafe extern "C" fn(*mut c_void, CText, usize, CAnswer)>,
    pub set_state: Option<unsafe extern "C" fn(*mut c_void, CText, usize, CText, usize)>,
    pub run: Option<unsafe extern "C" fn(*mut c_void, CText, usize, CText, usize) -> i32>,
    pub can_run: Option<unsafe extern "C" fn(*mut c_void, CText, usize, CText, usize) -> i32>,
    pub text: Option<unsafe extern "C" fn(*mut c_void, CText, usize, CAnswer)>,
    pub type_of: Option<unsafe extern "C" fn(*mut c_void, CText, usize, CAnswer)>,
    pub choices: Option<unsafe extern "C" fn(*mut c_void, CText, usize, CAnswer)>,
    pub offered: Option<unsafe extern "C" fn(*mut c_void, CText, usize, CText, usize, CAnswer)>,
    pub field: Option<unsafe extern "C" fn(*mut c_void, CText, usize, CAnswer)>,
    pub formats: Option<unsafe extern "C" fn(*mut c_void, CAnswer)>,
    pub open_address: Option<unsafe extern "C" fn(*mut c_void, CText, usize)>,
    pub resource: Option<unsafe extern "C" fn(*mut c_void, CText, usize, CAnswer)>,
    pub icon_path: Option<unsafe extern "C" fn(*mut c_void, CText, usize, CAnswer)>,
    pub changes: Option<unsafe extern "C" fn(*mut c_void) -> u64>,
    pub fetch: Option<unsafe extern "C" fn(*mut c_void, CText, usize, CText, usize, CAnswer)>,
    pub say: Option<unsafe extern "C" fn(*mut c_void, CText, usize)>,
    pub pointed: Option<unsafe extern "C" fn(*mut c_void, CText, usize, CText, usize, CText, usize) -> i32>,
    pub acted: Option<unsafe extern "C" fn(*mut c_void, CText, usize, CText, usize) -> i32>,
    pub told: Option<unsafe extern "C" fn(*mut c_void, CText, usize, CText, usize, CText, usize)>,
    pub bound_of: Option<unsafe extern "C" fn(*mut c_void, CText, usize, CAnswer)>,
    pub keys_of: Option<unsafe extern "C" fn(*mut c_void, CText, usize, CText, usize, CAnswer)>,
    pub hears: Option<unsafe extern "C" fn(*mut c_void, CText, usize, CAnswer)>,
    pub selected: Option<unsafe extern "C" fn(*mut c_void, CText, usize) -> i64>,
    pub select: Option<unsafe extern "C" fn(*mut c_void, CText, usize, i64)>,
    pub window_name: Option<unsafe extern "C" fn(*mut c_void, CAnswer)>,
    pub engine_start: Option<unsafe extern "C" fn(*mut c_void, CText, usize, CAnswer)>,
    pub engine_hears: Option<unsafe extern "C" fn(*mut c_void, u64) -> i32>,
    pub engine_frame: Option<unsafe extern "C" fn(*mut c_void, u64, f64, CText, usize)>,
    pub engine_drawn: Option<unsafe extern "C" fn(*mut c_void, u64, CAnswer)>,
    pub engine_stop: Option<unsafe extern "C" fn(*mut c_void, u64)>,
    pub release: Option<unsafe extern "C" fn(*mut c_void)>,
}

/// The names of the callbacks a host must give.
pub fn missing(callbacks: &ClarityPartHostCallbacks) -> Option<&'static str> {
    let given = [
        ("read", callbacks.read.is_some()),
        ("write", callbacks.write.is_some()),
        ("state", callbacks.state.is_some()),
        ("set_state", callbacks.set_state.is_some()),
        ("run", callbacks.run.is_some()),
        ("text", callbacks.text.is_some()),
        ("changes", callbacks.changes.is_some()),
    ];
    given.into_iter().find(|(_, is)| !is).map(|(name, _)| name)
}

/// The host behind a screen's parts.
pub struct CallbackHost {
    callbacks: ClarityPartHostCallbacks,
}

impl Drop for CallbackHost {
    fn drop(&mut self) {
        if let Some(release) = self.callbacks.release {
            crate::quietly(|| unsafe { release(self.callbacks.user) });
        }
    }
}

/// A text as (pointer, length) for a callback; none is NULL.
fn arg(text: Option<&str>) -> (CText, usize) {
    match text {
        Some(t) => (t.as_ptr() as CText, t.len()),
        None => (std::ptr::null(), 0),
    }
}

/// The JSON of a part's spec, as `offered` gives it.
pub fn spec_json(spec: &PartSpec) -> Value {
    json!({ "kind": spec.kind, "props": spec.props })
}

/// The JSON of a handler call.
pub fn call_json(call: &HandlerCall) -> Value {
    json!({ "event": call.event, "handler": call.handler, "args": call.args })
}

fn choices_of(value: Option<Value>) -> Vec<Choice> {
    let Some(Value::Array(items)) = value else { return Vec::new() };
    items
        .into_iter()
        .map(|item| Choice {
            label: item.get("label").and_then(Value::as_str).unwrap_or_default().to_string(),
            cells: item.get("cells").and_then(Value::as_array).map(|c| c.iter().map(|v| v.as_str().map(str::to_string).unwrap_or_else(|| v.to_string())).collect()),
            value: item.get("value").cloned().unwrap_or(Value::Null),
        })
        .collect()
}

fn field_of(value: &Value) -> FieldFacts {
    let text = |k: &str| value.get(k).and_then(Value::as_str).map(str::to_string);
    let flag = |k: &str| value.get(k).and_then(Value::as_bool).unwrap_or(false);
    FieldFacts {
        label: text("label").unwrap_or_default(),
        format: text("format"),
        required: flag("required"),
        min: text("min"),
        max: text("max"),
        refers: text("refers"),
        parent: text("parent"),
        color: flag("color"),
        help: text("help"),
    }
}

impl CallbackHost {
    pub fn new(callbacks: ClarityPartHostCallbacks) -> CallbackHost {
        CallbackHost { callbacks }
    }

    fn user(&self) -> *mut c_void {
        self.callbacks.user
    }

    /// Asks with one text: the answer, none when the callback is not given or set nothing.
    fn ask1(&self, f: Option<unsafe extern "C" fn(*mut c_void, CText, usize, CAnswer)>, a: Option<&str>) -> Option<ClarityAnswer> {
        let f = f?;
        let mut answer = ClarityAnswer::default();
        let (p, l) = arg(a);
        unsafe { f(self.user(), p, l, &mut answer) };
        answer.data.is_some().then_some(answer)
    }

    fn ask2(&self, f: Option<unsafe extern "C" fn(*mut c_void, CText, usize, CText, usize, CAnswer)>, a: Option<&str>, b: Option<&str>) -> Option<ClarityAnswer> {
        let f = f?;
        let mut answer = ClarityAnswer::default();
        let ((pa, la), (pb, lb)) = (arg(a), arg(b));
        unsafe { f(self.user(), pa, la, pb, lb, &mut answer) };
        answer.data.is_some().then_some(answer)
    }

    fn ask0(&self, f: Option<unsafe extern "C" fn(*mut c_void, CAnswer)>) -> Option<ClarityAnswer> {
        let f = f?;
        let mut answer = ClarityAnswer::default();
        unsafe { f(self.user(), &mut answer) };
        answer.data.is_some().then_some(answer)
    }
}

impl PartHost for CallbackHost {
    fn read(&self, path: &str) -> Option<Value> {
        self.ask1(self.callbacks.read, Some(path))?.json()
    }

    fn rows(&self, path: &str) -> Vec<Value> {
        let answered = match self.callbacks.rows {
            Some(_) => self.ask1(self.callbacks.rows, Some(path)).and_then(ClarityAnswer::json),
            None => self.read(path),
        };
        match answered {
            Some(Value::Array(rows)) => rows,
            _ => Vec::new(),
        }
    }

    fn write(&self, part: &str, path: &str, value: Value) {
        if let Some(f) = self.callbacks.write {
            let value = value.to_string();
            let ((a, al), (b, bl), (c, cl)) = (arg(Some(part)), arg(Some(path)), arg(Some(&value)));
            unsafe { f(self.user(), a, al, b, bl, c, cl) };
        }
    }

    fn edited(&self, part: &str) {
        if let Some(f) = self.callbacks.edited {
            let (a, al) = arg(Some(part));
            unsafe { f(self.user(), a, al) };
        }
    }

    fn state(&self, name: Option<&str>) -> Option<PartState> {
        let name = name?;
        serde_json::from_value(self.ask1(self.callbacks.state, Some(name))?.json()?).ok()
    }

    fn set_state(&self, name: &str, state: PartState) {
        if let Some(f) = self.callbacks.set_state {
            let state = serde_json::to_string(&state).unwrap_or_else(|_| "{}".to_string());
            let ((a, al), (b, bl)) = (arg(Some(name)), arg(Some(&state)));
            unsafe { f(self.user(), a, al, b, bl) };
        }
    }

    fn run(&self, part: Option<&str>, call: &HandlerCall) -> bool {
        let Some(f) = self.callbacks.run else { return false };
        let call = call_json(call).to_string();
        let ((a, al), (b, bl)) = (arg(part), arg(Some(&call)));
        unsafe { f(self.user(), a, al, b, bl) != 0 }
    }

    fn can_run(&self, part: Option<&str>, call: &HandlerCall) -> bool {
        let Some(f) = self.callbacks.can_run else { return true };
        let call = call_json(call).to_string();
        let ((a, al), (b, bl)) = (arg(part), arg(Some(&call)));
        unsafe { f(self.user(), a, al, b, bl) != 0 }
    }

    fn text(&self, written: Option<&str>) -> String {
        self.ask1(self.callbacks.text, written).and_then(ClarityAnswer::text).unwrap_or_else(|| written.unwrap_or_default().to_string())
    }

    fn type_of(&self, path: &str) -> Option<String> {
        self.ask1(self.callbacks.type_of, Some(path))?.text()
    }

    fn choices(&self, path: &str) -> Vec<Choice> {
        choices_of(self.ask1(self.callbacks.choices, Some(path)).and_then(ClarityAnswer::json))
    }

    fn offered(&self, part: &PartSpec, path: &str) -> Vec<Choice> {
        if self.callbacks.offered.is_none() {
            return self.choices(path);
        }
        let spec = spec_json(part).to_string();
        choices_of(self.ask2(self.callbacks.offered, Some(&spec), Some(path)).and_then(ClarityAnswer::json))
    }

    fn field(&self, path: &str) -> Option<FieldFacts> {
        let value = self.ask1(self.callbacks.field, Some(path))?.json()?;
        value.is_object().then(|| field_of(&value))
    }

    fn formats(&self) -> Formats {
        let Some(value) = self.ask0(self.callbacks.formats).and_then(ClarityAnswer::json) else { return Formats::default() };
        Formats {
            language: value.get("language").and_then(Value::as_str).map(str::to_string),
            currency: value.get("currency").and_then(Value::as_str).map(str::to_string),
        }
    }

    fn open_address(&self, address: &str) {
        if let Some(f) = self.callbacks.open_address {
            let (a, al) = arg(Some(address));
            unsafe { f(self.user(), a, al) };
        }
    }

    fn resource(&self, name: &str) -> Option<Vec<u8>> {
        self.ask1(self.callbacks.resource, Some(name))?.data
    }

    fn icon_path(&self, name: &str) -> Option<String> {
        self.ask1(self.callbacks.icon_path, Some(name))?.text()
    }

    fn changes(&self) -> u64 {
        match self.callbacks.changes {
            Some(f) => unsafe { f(self.user()) },
            None => 0,
        }
    }

    fn fetch(&self, bind: &str, store_path: Option<&str>) -> Option<Vec<u8>> {
        self.ask2(self.callbacks.fetch, Some(bind), store_path)?.data
    }

    fn say(&self, level: Level, text: Text) {
        if let Some(f) = self.callbacks.say {
            let message = serde_json::to_string(&Message { level, text }).unwrap_or_default();
            let (a, al) = arg(Some(&message));
            unsafe { f(self.user(), a, al) };
        }
    }

    fn pointed(&self, part: &str, event: &str, at: &Map<String, Value>) -> Option<bool> {
        let f = self.callbacks.pointed?;
        let at = Value::Object(at.clone()).to_string();
        let ((a, al), (b, bl), (c, cl)) = (arg(Some(part)), arg(Some(event)), arg(Some(&at)));
        let ran = unsafe { f(self.user(), a, al, b, bl, c, cl) };
        (ran >= 0).then_some(ran != 0)
    }

    fn acted(&self, part: &str, action: &Map<String, Value>) -> Option<bool> {
        let f = self.callbacks.acted?;
        let action = Value::Object(action.clone()).to_string();
        let ((a, al), (b, bl)) = (arg(Some(part)), arg(Some(&action)));
        let ran = unsafe { f(self.user(), a, al, b, bl) };
        (ran >= 0).then_some(ran != 0)
    }

    fn told(&self, part: &str, tell: &str, values: &[Value]) {
        if let Some(f) = self.callbacks.told {
            let values = Value::Array(values.to_vec()).to_string();
            let ((a, al), (b, bl), (c, cl)) = (arg(Some(part)), arg(Some(tell)), arg(Some(&values)));
            unsafe { f(self.user(), a, al, b, bl, c, cl) };
        }
    }

    fn bound_of(&self, part: &str) -> Option<String> {
        self.ask1(self.callbacks.bound_of, Some(part))?.text().filter(|p| !p.is_empty())
    }

    fn keys_of(&self, command: &str, designed: Option<&str>) -> Vec<String> {
        match self.ask2(self.callbacks.keys_of, Some(command), designed).and_then(ClarityAnswer::json) {
            Some(Value::Array(keys)) => keys.into_iter().filter_map(|k| k.as_str().map(str::to_string)).collect(),
            _ => designed.filter(|d| !d.is_empty()).map(|d| vec![d.to_string()]).unwrap_or_default(),
        }
    }

    fn hears(&self, part: &str) -> Vec<Hear> {
        self.ask1(self.callbacks.hears, Some(part)).and_then(ClarityAnswer::json).and_then(|v| serde_json::from_value(v).ok()).unwrap_or_default()
    }

    fn selected(&self, part: &str) -> Option<usize> {
        let f = self.callbacks.selected?;
        let (a, al) = arg(Some(part));
        usize::try_from(unsafe { f(self.user(), a, al) }).ok()
    }

    fn select(&self, part: &str, place: Option<usize>) {
        if let Some(f) = self.callbacks.select {
            let (a, al) = arg(Some(part));
            unsafe { f(self.user(), a, al, place.map(|p| p as i64).unwrap_or(-1)) };
        }
    }

    fn engine_start(&self, name: &str) -> Option<Result<u64, String>> {
        let started = self.ask1(self.callbacks.engine_start, Some(name))?.json()?;
        if let Some(why) = started.get("error") {
            return Some(Err(why.as_str().map(str::to_string).unwrap_or_else(|| why.to_string())));
        }
        started.get("id").and_then(Value::as_u64).map(Ok)
    }

    fn engine_hears(&self, id: u64) -> Hears {
        let Some(f) = self.callbacks.engine_hears else { return Hears::default() };
        let bits = unsafe { f(self.user(), id) };
        Hears { pointer: bits & 1 != 0, keys: bits & 2 != 0 }
    }

    fn engine_frame(&self, id: u64, ms: f64, input: Vec<ScreenInput>) {
        if let Some(f) = self.callbacks.engine_frame {
            let input = Value::Array(input.iter().map(wire::ui_screen_input).collect()).to_string();
            let (a, al) = arg(Some(&input));
            unsafe { f(self.user(), id, ms, a, al) };
        }
    }

    fn engine_drawn(&self, id: u64) -> Option<Result<Drawn, String>> {
        let f = self.callbacks.engine_drawn?;
        let mut answer = ClarityAnswer::default();
        unsafe { f(self.user(), id, &mut answer) };
        let drawn = answer.json()?;
        if let Some(why) = drawn.get("error") {
            return Some(Err(why.as_str().map(str::to_string).unwrap_or_else(|| why.to_string())));
        }
        Some(Ok(wire::ui_drawn(&drawn)))
    }

    fn engine_stop(&self, id: u64) {
        if let Some(f) = self.callbacks.engine_stop {
            unsafe { f(self.user(), id) };
        }
    }

    fn window_name(&self) -> Option<String> {
        self.ask0(self.callbacks.window_name)?.text()
    }
}
