//! The support module of engines written in Clarity (DESIGN 5.12, N5b).
//!
//! It speaks the engine interface (conformance/engines.json): `memory`,
//! `clarity_alloc`, `clarity_free`. Clarity's compiler adds the engine's own
//! functions to this module; they work on values through the `rt_*`
//! functions here, which the compiler finds by their exports and then
//! leaves out of the engine's exports. Built with the `interpreter`
//! feature, it runs an engine's IR instead (`interp`).
//!
//! In compiled code whole numbers are wasm's own i64 and yes or no its i32;
//! every other value is a handle, a number for a value in the call's arena,
//! which is emptied when the call ends. A list is a value: changing one
//! gives a new handle, so no other name sees the change. What the engine
//! keeps lives outside the arena; a call works on a copy, kept only when it
//! succeeds.

use std::cell::RefCell;

use serde_json::{Value as Json, json};

mod values;
use values::{Num, Value};

#[cfg(feature = "interpreter")]
mod interp;

#[derive(Default)]
struct Call {
    /// The values of this call; a handle is an index here, plus one.
    arena: Vec<Value>,
    args: Vec<Value>,
    /// The arguments as JSON, for those read by their type.
    json: Vec<Json>,
    answer: Option<Value>,
    /// Why the call failed or was refused, once it has.
    failed: Option<String>,
    /// What the engine keeps, as this call changes it.
    working: Vec<Value>,
    tells: Vec<Json>,
}

#[derive(Default)]
struct State {
    call: Call,
    /// What the engine keeps between calls; empty until the first call made it.
    kept: Option<Vec<Value>>,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
    STATE.with_borrow_mut(f)
}

fn put(state: &mut State, value: Value) -> i32 {
    state.call.arena.push(value);
    state.call.arena.len() as i32
}

fn get(state: &State, handle: i32) -> Value {
    state.call.arena.get((handle - 1) as usize).cloned().unwrap_or(Value::None)
}

fn fail(state: &mut State, why: String) {
    if state.call.failed.is_none() {
        state.call.failed = Some(why);
    }
}

/// A value, or the call failed with why and the value is none.
fn or_fail(state: &mut State, value: Result<Value, String>) -> Value {
    value.unwrap_or_else(|why| {
        fail(state, why);
        Value::None
    })
}

fn kept(s: &mut State, index: i32) -> Value {
    let index = index as usize;
    if s.call.working.len() <= index {
        s.call.working.resize(index + 1, Value::None);
    }
    s.call.working[index].clone()
}

fn keep(s: &mut State, index: i32, value: Value) {
    let index = index as usize;
    if s.call.working.len() <= index {
        s.call.working.resize(index + 1, Value::None);
    }
    s.call.working[index] = value;
}

fn text_at(at: i32, len: i32) -> String {
    let bytes = unsafe { std::slice::from_raw_parts(at as *const u8, len as usize) };
    String::from_utf8_lossy(bytes).into_owned()
}

// The engine interface.

#[unsafe(no_mangle)]
pub extern "C" fn clarity_alloc(len: i32) -> i32 {
    let mut room = Vec::<u8>::with_capacity(len.max(1) as usize);
    let at = room.as_mut_ptr() as i32;
    std::mem::forget(room);
    at
}

#[unsafe(no_mangle)]
pub extern "C" fn clarity_free(at: i32, len: i32) {
    unsafe { drop(Vec::from_raw_parts(at as *mut u8, 0, len.max(1) as usize)) };
}

/// Hands `text` to the host as an answer: its place in the high 32 bits, its length in the low.
fn hand_over(text: String) -> i64 {
    let bytes = text.into_bytes();
    let len = bytes.len() as i32;
    let at = clarity_alloc(len);
    unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), at as *mut u8, bytes.len()) };
    ((at as i64) << 32) | (len as i64 & 0xFFFF_FFFF)
}

// A call: begin, the engine's code, finish.

/// Starts a call with its arguments, a JSON array at `at`, which the engine takes over.
#[unsafe(no_mangle)]
pub extern "C" fn rt_begin(at: i32, len: i32) {
    let bytes = unsafe { Vec::from_raw_parts(at as *mut u8, len as usize, len.max(1) as usize) };
    with(|s| {
        s.call = Call::default();
        s.call.working = s.kept.clone().unwrap_or_default();
        match serde_json::from_slice::<Json>(&bytes) {
            Ok(Json::Array(items)) => {
                s.call.args = items.iter().map(Value::from_json).collect();
                s.call.json = items;
            }
            _ => fail(s, "its arguments are no JSON list".into()),
        }
    });
}

/// Whether the engine's kept values are still to be made, on its first call.
#[unsafe(no_mangle)]
pub extern "C" fn rt_unkept() -> i32 {
    with(|s| s.kept.is_none()) as i32
}

/// Whether the call has failed or been refused, so the code stops.
#[unsafe(no_mangle)]
pub extern "C" fn rt_failed() -> i32 {
    with(|s| s.call.failed.is_some()) as i32
}

/// Ends a call: its answer, or why it failed. What it kept stays only when it succeeded.
#[unsafe(no_mangle)]
pub extern "C" fn rt_finish() -> i64 {
    let answer = with(|s| {
        let call = std::mem::take(&mut s.call);
        let mut answer = match call.failed {
            Some(why) => json!({ "error": why }),
            None => {
                s.kept = Some(call.working);
                json!({ "value": call.answer.as_ref().map(Value::json).unwrap_or(Json::Null) })
            }
        };
        if !call.tells.is_empty() && answer.get("value").is_some() {
            answer["tells"] = Json::Array(call.tells);
        }
        answer.to_string()
    });
    hand_over(answer)
}

#[unsafe(no_mangle)]
pub extern "C" fn rt_refuse(text: i32) {
    with(|s| {
        let why = get(s, text).show();
        fail(s, why);
    });
}

// Arguments and the answer.

#[unsafe(no_mangle)]
pub extern "C" fn rt_arg_int(index: i32) -> i64 {
    with(|s| {
        let value = values::arg_int(&s.call.args, index as usize);
        or_fail(s, value).int()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rt_arg_bool(index: i32) -> i32 {
    with(|s| {
        let value = values::arg_bool(&s.call.args, index as usize);
        or_fail(s, value).truth() as i32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rt_arg(index: i32) -> i32 {
    with(|s| {
        let value = values::arg(&s.call.args, index as usize);
        let value = or_fail(s, value);
        put(s, value)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rt_arg_dec(index: i32) -> i32 {
    with(|s| {
        let value = values::arg_decimal(&s.call.json, index as usize);
        let value = or_fail(s, value);
        put(s, value)
    })
}

/// A list of decimals among the arguments.
#[unsafe(no_mangle)]
pub extern "C" fn rt_arg_decs(index: i32) -> i32 {
    with(|s| {
        let value = values::arg_decimals(&s.call.json, index as usize);
        let value = or_fail(s, value);
        put(s, value)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rt_answer_int(n: i64) {
    with(|s| s.call.answer = Some(Value::Int(n)));
}

#[unsafe(no_mangle)]
pub extern "C" fn rt_answer_bool(b: i32) {
    with(|s| s.call.answer = Some(Value::Bool(b != 0)));
}

#[unsafe(no_mangle)]
pub extern "C" fn rt_answer(handle: i32) {
    with(|s| s.call.answer = Some(get(s, handle)));
}

// What the engine keeps.

#[unsafe(no_mangle)]
pub extern "C" fn rt_kept_int(index: i32) -> i64 {
    with(|s| if let Value::Int(n) = kept(s, index) { n } else { 0 })
}

#[unsafe(no_mangle)]
pub extern "C" fn rt_kept_bool(index: i32) -> i32 {
    with(|s| kept(s, index).truth() as i32)
}

#[unsafe(no_mangle)]
pub extern "C" fn rt_kept(index: i32) -> i32 {
    with(|s| {
        let value = kept(s, index);
        put(s, value)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rt_keep_int(index: i32, n: i64) {
    with(|s| keep(s, index, Value::Int(n)));
}

#[unsafe(no_mangle)]
pub extern "C" fn rt_keep_bool(index: i32, b: i32) {
    with(|s| keep(s, index, Value::Bool(b != 0)));
}

#[unsafe(no_mangle)]
pub extern "C" fn rt_keep(index: i32, handle: i32) {
    with(|s| {
        let value = get(s, handle);
        keep(s, index, value)
    });
}

// Values written in the code, and texts.

/// A text written in the engine's code: `len` bytes of UTF-8 at `at`, where the compiler put them.
#[unsafe(no_mangle)]
pub extern "C" fn rt_text(at: i32, len: i32) -> i32 {
    let text = text_at(at, len);
    with(|s| put(s, Value::Text(text)))
}

/// A decimal written in the code: its text at `at`.
#[unsafe(no_mangle)]
pub extern "C" fn rt_dec(at: i32, len: i32) -> i32 {
    let value = values::decimal(&text_at(at, len));
    with(|s| put(s, value))
}

#[unsafe(no_mangle)]
pub extern "C" fn rt_int(n: i64) -> i32 {
    with(|s| put(s, Value::Int(n)))
}

#[unsafe(no_mangle)]
pub extern "C" fn rt_bool(b: i32) -> i32 {
    with(|s| put(s, Value::Bool(b != 0)))
}

/// Two values as one text: `"Move " + n`.
#[unsafe(no_mangle)]
pub extern "C" fn rt_join(a: i32, b: i32) -> i32 {
    with(|s| {
        let text = values::join(&get(s, a), &get(s, b));
        put(s, text)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rt_eq(a: i32, b: i32) -> i32 {
    with(|s| get(s, a) == get(s, b)) as i32
}

#[unsafe(no_mangle)]
pub extern "C" fn rt_compare(a: i32, b: i32) -> i32 {
    with(|s| values::compare(&get(s, a), &get(s, b)))
}

// Decimals and money.

fn arithmetic(op: Num, a: i32, b: i32) -> i32 {
    with(|s| {
        let value = values::num(op, &get(s, a), &get(s, b));
        let value = or_fail(s, value);
        put(s, value)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rt_num_add(a: i32, b: i32) -> i32 {
    arithmetic(Num::Add, a, b)
}

#[unsafe(no_mangle)]
pub extern "C" fn rt_num_sub(a: i32, b: i32) -> i32 {
    arithmetic(Num::Sub, a, b)
}

#[unsafe(no_mangle)]
pub extern "C" fn rt_num_mul(a: i32, b: i32) -> i32 {
    arithmetic(Num::Mul, a, b)
}

#[unsafe(no_mangle)]
pub extern "C" fn rt_num_div(a: i32, b: i32) -> i32 {
    arithmetic(Num::Div, a, b)
}

/// `round(value, places)`, half away from zero (conformance/rounding.json).
#[unsafe(no_mangle)]
pub extern "C" fn rt_round(value: i32, places: i64) -> i32 {
    with(|s| {
        let rounded = values::round(&get(s, value), places);
        put(s, rounded)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rt_sin(degrees: i32) -> i32 {
    with(|s| {
        let value = values::angle(&get(s, degrees), f64::sin);
        put(s, value)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rt_cos(degrees: i32) -> i32 {
    with(|s| {
        let value = values::angle(&get(s, degrees), f64::cos);
        put(s, value)
    })
}

/// `random(max)`: 1 to `max` from the host's next number.
#[unsafe(no_mangle)]
pub extern "C" fn rt_random(next: i64, max: i64) -> i64 {
    with(|s| {
        let value = values::random(next, max).map(Value::Int);
        or_fail(s, value).int()
    })
}

// Lists of values, items counted from 1.

/// A new empty list, which `rt_push` fills: `[1, 2, 3]`.
#[unsafe(no_mangle)]
pub extern "C" fn rt_list() -> i32 {
    with(|s| put(s, Value::List(Vec::new())))
}

/// The list with `item` added at the end, as a new handle.
#[unsafe(no_mangle)]
pub extern "C" fn rt_push(list: i32, item: i32) -> i32 {
    with(|s| {
        let list = values::push(&get(s, list), get(s, item));
        put(s, list)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rt_count(list: i32) -> i64 {
    with(|s| values::count(&get(s, list)))
}

fn item(list: i32, at: i64) -> Value {
    with(|s| {
        let value = values::item(&get(s, list), at);
        or_fail(s, value)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rt_item_int(list: i32, at: i64) -> i64 {
    if let Value::Int(n) = item(list, at) { n } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn rt_item_bool(list: i32, at: i64) -> i32 {
    item(list, at).truth() as i32
}

#[unsafe(no_mangle)]
pub extern "C" fn rt_item(list: i32, at: i64) -> i32 {
    let value = item(list, at);
    with(|s| put(s, value))
}

/// The list with item `at` set to `value`, as a new handle; the old one is as it was.
#[unsafe(no_mangle)]
pub extern "C" fn rt_set_item(list: i32, at: i64, value: i32) -> i32 {
    with(|s| match values::set_item(&get(s, list), at, get(s, value)) {
        Ok(changed) => put(s, changed),
        Err(why) => {
            fail(s, why);
            list
        }
    })
}

/// Whether a list holds a value, or a text holds a part, ignoring case.
#[unsafe(no_mangle)]
pub extern "C" fn rt_contains(whole: i32, part: i32) -> i32 {
    with(|s| values::contains(&get(s, whole), &get(s, part))) as i32
}

/// Two lists as one: `seen + [n]`.
#[unsafe(no_mangle)]
pub extern "C" fn rt_concat(a: i32, b: i32) -> i32 {
    with(|s| {
        let list = values::concat(&get(s, a), &get(s, b));
        put(s, list)
    })
}

// Records, their fields by name.

/// A new record without fields; `new` gives it each field at its default.
#[unsafe(no_mangle)]
pub extern "C" fn rt_record() -> i32 {
    with(|s| put(s, Value::Record(Vec::new())))
}

/// The record with field `name` (at `at`) set to `value`, as a new handle.
#[unsafe(no_mangle)]
pub extern "C" fn rt_with(record: i32, at: i32, len: i32, value: i32) -> i32 {
    let name = text_at(at, len);
    with(|s| {
        let changed = values::with_field(&get(s, record), &name, get(s, value));
        put(s, changed)
    })
}

fn field(record: i32, at: i32, len: i32) -> Value {
    let name = text_at(at, len);
    with(|s| values::field(&get(s, record), &name))
}

#[unsafe(no_mangle)]
pub extern "C" fn rt_field(record: i32, at: i32, len: i32) -> i32 {
    let value = field(record, at, len);
    with(|s| put(s, value))
}

#[unsafe(no_mangle)]
pub extern "C" fn rt_field_int(record: i32, at: i32, len: i32) -> i64 {
    field(record, at, len).int()
}

#[unsafe(no_mangle)]
pub extern "C" fn rt_field_bool(record: i32, at: i32, len: i32) -> i32 {
    field(record, at, len).truth() as i32
}

// Tells, for an engine beside the screen (DESIGN 5.12).

/// `tell name(values)`: a tell with its values, in the answer of this call.
#[unsafe(no_mangle)]
pub extern "C" fn rt_tell(at: i32, len: i32, values: i32) {
    let name = text_at(at, len);
    with(|s| {
        let told = values::tell(&name, &get(s, values));
        s.call.tells.push(told);
    });
}
