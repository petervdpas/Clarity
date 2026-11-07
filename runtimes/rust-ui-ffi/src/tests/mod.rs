//! The C interface, called as C calls it: every entry through its
//! `extern "C"` function, with callbacks written in Rust as a C program
//! writes them. They are the library's own tests (one test binary, so Skia
//! is linked once); tests/c_header.rs builds a C program against the header.

mod audio;
mod basics;
mod custom;
mod program;
mod screen;

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::ffi::{c_char, c_void, CStr};

use crate::host::ClarityPartHostCallbacks;
use crate::ClarityAnswer;
use serde_json::{Map, Value};

/// Held by the tests that change what the library keeps for the process
/// (the person's choices, the texts' language), so they do not meet.
pub static GLOBAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Text as a C caller gives it.
pub fn t(text: &str) -> (*const c_char, usize) {
    (text.as_ptr() as *const c_char, text.len())
}

/// Text a callback was given.
pub unsafe fn given(ptr: *const c_char, len: usize) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    Some(String::from_utf8(unsafe { std::slice::from_raw_parts(ptr as *const u8, len) }.to_vec()).unwrap())
}

/// Answers a callback with text.
pub unsafe fn answer(to: *mut ClarityAnswer, text: &str) {
    assert_eq!(unsafe { crate::clarity_answer_set(to, text.as_ptr(), text.len()) }, crate::CLARITY_OK);
}

/// A string the library gave, read and freed.
pub unsafe fn taken(text: *mut c_char) -> Option<String> {
    if text.is_null() {
        return None;
    }
    let read = unsafe { CStr::from_ptr(text) }.to_str().unwrap().to_string();
    unsafe { crate::clarity_string_free(text) };
    Some(read)
}

/// The message of the last failing call.
pub fn last_error() -> String {
    unsafe { taken(crate::clarity_last_error()) }.unwrap_or_default()
}

/// A window model as a C# program would keep it: its values, its parts'
/// states, and what the library asked of it.
#[derive(Default)]
pub struct Model {
    pub values: RefCell<Map<String, Value>>,
    pub states: RefCell<BTreeMap<String, Value>>,
    pub log: RefCell<Vec<String>>,
    pub changes: RefCell<u64>,
    pub released: RefCell<bool>,
}

impl Model {
    pub fn with(values: Value) -> Box<Model> {
        let model = Box::<Model>::default();
        *model.values.borrow_mut() = values.as_object().cloned().unwrap_or_default();
        model
    }

    pub fn logged(&self) -> Vec<String> {
        std::mem::take(&mut *self.log.borrow_mut())
    }

    fn of<'a>(user: *mut c_void) -> &'a Model {
        unsafe { &*(user as *const Model) }
    }
}

unsafe extern "C" fn read(user: *mut c_void, path: *const c_char, len: usize, out: *mut ClarityAnswer) {
    let path = unsafe { given(path, len) }.unwrap();
    if let Some(value) = Model::of(user).values.borrow().get(&path) {
        unsafe { answer(out, &value.to_string()) };
    }
}

unsafe extern "C" fn write(user: *mut c_void, part: *const c_char, part_len: usize, path: *const c_char, path_len: usize, value: *const c_char, value_len: usize) {
    let model = Model::of(user);
    let (part, path, value) = unsafe { (given(part, part_len).unwrap(), given(path, path_len).unwrap(), given(value, value_len).unwrap()) };
    model.log.borrow_mut().push(format!("write {part} {path} {value}"));
    model.values.borrow_mut().insert(path, serde_json::from_str(&value).unwrap());
    *model.changes.borrow_mut() += 1;
}

unsafe extern "C" fn edited(user: *mut c_void, part: *const c_char, len: usize) {
    Model::of(user).log.borrow_mut().push(format!("edited {}", unsafe { given(part, len) }.unwrap()));
}

unsafe extern "C" fn state(user: *mut c_void, name: *const c_char, len: usize, out: *mut ClarityAnswer) {
    let name = unsafe { given(name, len) }.unwrap();
    if let Some(state) = Model::of(user).states.borrow().get(&name) {
        unsafe { answer(out, &state.to_string()) };
    }
}

unsafe extern "C" fn set_state(user: *mut c_void, name: *const c_char, len: usize, state: *const c_char, state_len: usize) {
    let (name, state) = unsafe { (given(name, len).unwrap(), given(state, state_len).unwrap()) };
    Model::of(user).states.borrow_mut().insert(name, serde_json::from_str(&state).unwrap());
}

unsafe extern "C" fn run(user: *mut c_void, part: *const c_char, part_len: usize, call: *const c_char, call_len: usize) -> i32 {
    let (part, call) = unsafe { (given(part, part_len), given(call, call_len).unwrap()) };
    let call: Value = serde_json::from_str(&call).unwrap();
    Model::of(user).log.borrow_mut().push(format!("run {} {} {}", part.unwrap_or_default(), call["event"].as_str().unwrap(), call["handler"].as_str().unwrap()));
    1
}

unsafe extern "C" fn text(_user: *mut c_void, written: *const c_char, len: usize, out: *mut ClarityAnswer) {
    // `@saveCaption` by its key, as the program's translations give it.
    let written = unsafe { given(written, len) }.unwrap_or_default();
    let shown = if written == "@saveCaption" { "Save it".to_string() } else { written };
    unsafe { answer(out, &shown) };
}

unsafe extern "C" fn changes(user: *mut c_void) -> u64 {
    *Model::of(user).changes.borrow()
}

unsafe extern "C" fn told(user: *mut c_void, part: *const c_char, part_len: usize, tell: *const c_char, tell_len: usize, values: *const c_char, values_len: usize) {
    let (part, tell, values) = unsafe { (given(part, part_len).unwrap(), given(tell, tell_len).unwrap(), given(values, values_len).unwrap()) };
    Model::of(user).log.borrow_mut().push(format!("told {part} {tell} {values}"));
}

unsafe extern "C" fn release(user: *mut c_void) {
    *Model::of(user).released.borrow_mut() = true;
}

/// The part host over a model, as a C# program gives it: what it does not
/// answer is left NULL.
pub fn host_of(model: &Model) -> ClarityPartHostCallbacks {
    ClarityPartHostCallbacks {
        user: model as *const Model as *mut c_void,
        read: Some(read),
        rows: None,
        write: Some(write),
        edited: Some(edited),
        state: Some(state),
        set_state: Some(set_state),
        run: Some(run),
        can_run: None,
        text: Some(text),
        type_of: None,
        choices: None,
        offered: None,
        field: None,
        formats: None,
        open_address: None,
        resource: None,
        icon_path: None,
        changes: Some(changes),
        fetch: None,
        say: None,
        pointed: None,
        acted: None,
        told: Some(told),
        bound_of: None,
        keys_of: None,
        hears: None,
        selected: None,
        select: None,
        window_name: None,
        engine_start: None,
        engine_hears: None,
        engine_frame: None,
        engine_drawn: None,
        engine_stop: None,
        release: Some(release),
    }
}

/// A window of parts in a grid, as its markup gives it.
pub fn window(parts: &str) -> String {
    format!("<Window name=\"Main\">{parts}</Window>")
}

pub fn ok(status: i32) {
    assert_eq!(status, crate::CLARITY_OK, "{}", last_error());
}
