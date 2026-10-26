//! The support module of engines written in Clarity (DESIGN 5.12, N5b).
//!
//! It speaks the engine interface (conformance/engines.json): `memory`,
//! `clarity_alloc`, `clarity_free`. Clarity's compiler adds the engine's own
//! functions to this module; they work on values through the `rt_*`
//! functions here, which the compiler finds by their exports and then
//! leaves out of the engine's exports.
//!
//! Whole numbers are wasm's own i64 and yes or no its i32; every other
//! value is a handle, a number for a value in the call's arena, which is
//! emptied when the call ends. A list is a value: changing one gives a new
//! handle, so no other name sees the change. What the engine keeps lives
//! outside the arena; a call works on a copy, kept only when it succeeds.

use std::cell::RefCell;

use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use serde_json::{Value as Json, json};

#[derive(Debug, Clone)]
enum Value {
    None,
    Int(i64),
    /// A decimal or money: exact, as rust_decimal has it.
    Dec(Decimal),
    Bool(bool),
    Text(String),
    List(Vec<Value>),
    /// A record (`record Dot`): its fields in order.
    Record(Vec<(String, Value)>),
}

impl Value {
    fn json(&self) -> Json {
        match self {
            Value::None => Json::Null,
            Value::Int(n) => json!(n),
            Value::Dec(d) => json!(d.to_string()),
            Value::Bool(b) => json!(b),
            Value::Text(t) => json!(t),
            Value::List(items) => Json::Array(items.iter().map(Value::json).collect()),
            Value::Record(fields) => Json::Object(fields.iter().map(|(name, value)| (name.clone(), value.json())).collect()),
        }
    }

    fn from_json(value: &Json) -> Value {
        match value {
            Json::Null => Value::None,
            Json::Bool(b) => Value::Bool(*b),
            Json::Number(n) => n.as_i64().map(Value::Int).unwrap_or(Value::None),
            Json::String(t) => Value::Text(t.clone()),
            Json::Array(items) => Value::List(items.iter().map(Value::from_json).collect()),
            Json::Object(fields) => Value::Record(fields.iter().map(|(name, value)| (name.clone(), Value::from_json(value))).collect()),
        }
    }

    /// The value as text, as `+` joins it to text.
    fn show(&self) -> String {
        match self {
            Value::None => String::new(),
            Value::Int(n) => n.to_string(),
            Value::Dec(d) => d.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Text(t) => t.clone(),
            Value::List(items) => items.iter().map(Value::show).collect::<Vec<_>>().join(", "),
            Value::Record(_) => self.json().to_string(),
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::None, Value::None) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Text(a), Value::Text(b)) => a == b,
            (Value::List(a), Value::List(b)) => a == b,
            (Value::Record(a), Value::Record(b)) => a == b,
            (a, b) => match (a.number(), b.number()) {
                (Some(x), Some(y)) => x == y,
                _ => false,
            },
        }
    }
}

impl Value {
    /// A whole number or a decimal as a decimal; a decimal inside a record
    /// or list that came as JSON is still its exact text.
    fn number(&self) -> Option<Decimal> {
        match self {
            Value::Int(n) => Some(Decimal::from(*n)),
            Value::Dec(d) => Some(*d),
            Value::Text(t) => t.parse().ok(),
            _ => None,
        }
    }

    /// A decimal read from JSON: exact text, or a number.
    fn decimal(json: &Json) -> Option<Value> {
        match json {
            Json::String(t) => t.parse::<Decimal>().ok().map(Value::Dec),
            Json::Number(n) => n.to_string().parse::<Decimal>().ok().map(Value::Dec),
            _ => None,
        }
    }
}

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

fn argument(s: &mut State, index: i32) -> Value {
    match s.call.args.get(index as usize) {
        Some(value) => value.clone(),
        None => {
            fail(s, format!("it takes more than {index} values"));
            Value::None
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn rt_arg_int(index: i32) -> i64 {
    with(|s| match argument(s, index) {
        Value::Int(n) => n,
        Value::None if s.call.failed.is_some() => 0,
        _ => {
            fail(s, format!("value {} is no whole number", index + 1));
            0
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rt_arg_bool(index: i32) -> i32 {
    with(|s| match argument(s, index) {
        Value::Bool(b) => b as i32,
        Value::None if s.call.failed.is_some() => 0,
        _ => {
            fail(s, format!("value {} is no yes or no", index + 1));
            0
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rt_arg(index: i32) -> i32 {
    with(|s| {
        let value = argument(s, index);
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

#[unsafe(no_mangle)]
pub extern "C" fn rt_kept_int(index: i32) -> i64 {
    with(|s| if let Value::Int(n) = kept(s, index) { n } else { 0 })
}

#[unsafe(no_mangle)]
pub extern "C" fn rt_kept_bool(index: i32) -> i32 {
    with(|s| matches!(kept(s, index), Value::Bool(true)) as i32)
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

// Texts.

/// A text written in the engine's code: `len` bytes of UTF-8 at `at`, where the compiler put them.
#[unsafe(no_mangle)]
pub extern "C" fn rt_text(at: i32, len: i32) -> i32 {
    let bytes = unsafe { std::slice::from_raw_parts(at as *const u8, len as usize) };
    let text = String::from_utf8_lossy(bytes).into_owned();
    with(|s| put(s, Value::Text(text)))
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
        let text = get(s, a).show() + &get(s, b).show();
        put(s, Value::Text(text))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rt_eq(a: i32, b: i32) -> i32 {
    with(|s| get(s, a) == get(s, b)) as i32
}

#[unsafe(no_mangle)]
pub extern "C" fn rt_compare(a: i32, b: i32) -> i32 {
    with(|s| match (get(s, a), get(s, b)) {
        (Value::Text(x), Value::Text(y)) => x.cmp(&y) as i32,
        (x, y) => match (x.number(), y.number()) {
            (Some(x), Some(y)) => x.cmp(&y) as i32,
            _ => 0,
        },
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
        let mut items = if let Value::List(items) = get(s, list) { items } else { Vec::new() };
        items.push(get(s, item));
        put(s, Value::List(items))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rt_count(list: i32) -> i64 {
    with(|s| if let Value::List(items) = get(s, list) { items.len() as i64 } else { 0 })
}

fn item(s: &mut State, list: i32, at: i64) -> Value {
    let items = if let Value::List(items) = get(s, list) { items } else { Vec::new() };
    if at < 1 || at as usize > items.len() {
        fail(s, format!("there is no item {at} in a list of {}", items.len()));
        return Value::None;
    }
    items[at as usize - 1].clone()
}

#[unsafe(no_mangle)]
pub extern "C" fn rt_item_int(list: i32, at: i64) -> i64 {
    with(|s| if let Value::Int(n) = item(s, list, at) { n } else { 0 })
}

#[unsafe(no_mangle)]
pub extern "C" fn rt_item_bool(list: i32, at: i64) -> i32 {
    with(|s| matches!(item(s, list, at), Value::Bool(true)) as i32)
}

#[unsafe(no_mangle)]
pub extern "C" fn rt_item(list: i32, at: i64) -> i32 {
    with(|s| {
        let value = item(s, list, at);
        put(s, value)
    })
}

/// The list with item `at` set to `value`, as a new handle; the old one is as it was.
#[unsafe(no_mangle)]
pub extern "C" fn rt_set_item(list: i32, at: i64, value: i32) -> i32 {
    with(|s| {
        let mut items = if let Value::List(items) = get(s, list) { items } else { Vec::new() };
        if at < 1 || at as usize > items.len() {
            fail(s, format!("there is no item {at} in a list of {}", items.len()));
            return list;
        }
        items[at as usize - 1] = get(s, value);
        put(s, Value::List(items))
    })
}

/// Whether a list holds a value, or a text holds a part, ignoring case.
#[unsafe(no_mangle)]
pub extern "C" fn rt_contains(whole: i32, part: i32) -> i32 {
    with(|s| match (get(s, whole), get(s, part)) {
        (Value::List(items), wanted) => items.contains(&wanted),
        (Value::Text(text), Value::Text(part)) => text.to_lowercase().contains(&part.to_lowercase()),
        _ => false,
    }) as i32
}

#[unsafe(no_mangle)]
pub extern "C" fn rt_handle_int(handle: i32) -> i64 {
    with(|s| if let Value::Int(n) = get(s, handle) { n } else { 0 })
}

#[unsafe(no_mangle)]
pub extern "C" fn rt_handle_bool(handle: i32) -> i32 {
    with(|s| matches!(get(s, handle), Value::Bool(true)) as i32)
}

/// Two lists as one: `seen + [n]`.
#[unsafe(no_mangle)]
pub extern "C" fn rt_concat(a: i32, b: i32) -> i32 {
    with(|s| {
        let mut items = if let Value::List(items) = get(s, a) { items } else { Vec::new() };
        if let Value::List(more) = get(s, b) {
            items.extend(more);
        }
        put(s, Value::List(items))
    })
}

// Decimals and money.

#[unsafe(no_mangle)]
pub extern "C" fn rt_arg_dec(index: i32) -> i32 {
    with(|s| {
        let value = match s.call.json.get(index as usize).map(Value::decimal) {
            Some(Some(value)) => value,
            _ => {
                fail(s, format!("value {} is no decimal", index + 1));
                Value::None
            }
        };
        put(s, value)
    })
}

/// A list of decimals among the arguments.
#[unsafe(no_mangle)]
pub extern "C" fn rt_arg_decs(index: i32) -> i32 {
    with(|s| {
        let items = match s.call.json.get(index as usize) {
            Some(Json::Array(items)) => items.iter().map(Value::decimal).collect::<Option<Vec<_>>>(),
            _ => None,
        };
        let value = match items {
            Some(items) => Value::List(items),
            None => {
                fail(s, format!("value {} is no list of decimals", index + 1));
                Value::None
            }
        };
        put(s, value)
    })
}

/// A decimal written in the code: its text at `at`.
#[unsafe(no_mangle)]
pub extern "C" fn rt_dec(at: i32, len: i32) -> i32 {
    let bytes = unsafe { std::slice::from_raw_parts(at as *const u8, len as usize) };
    let value = std::str::from_utf8(bytes).ok().and_then(|t| t.parse::<Decimal>().ok()).map(Value::Dec).unwrap_or(Value::None);
    with(|s| put(s, value))
}

fn numbers(s: &State, a: i32, b: i32) -> (Decimal, Decimal) {
    (get(s, a).number().unwrap_or_default(), get(s, b).number().unwrap_or_default())
}

fn arithmetic(a: i32, b: i32, op: fn(Decimal, Decimal) -> Option<Decimal>) -> i32 {
    with(|s| {
        let (x, y) = numbers(s, a, b);
        let value = match op(x, y) {
            Some(d) => Value::Dec(d),
            None => {
                fail(s, "it went beyond what a decimal holds".into());
                Value::None
            }
        };
        put(s, value)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rt_num_add(a: i32, b: i32) -> i32 {
    arithmetic(a, b, |x, y| x.checked_add(y))
}

#[unsafe(no_mangle)]
pub extern "C" fn rt_num_sub(a: i32, b: i32) -> i32 {
    arithmetic(a, b, |x, y| x.checked_sub(y))
}

#[unsafe(no_mangle)]
pub extern "C" fn rt_num_mul(a: i32, b: i32) -> i32 {
    arithmetic(a, b, |x, y| x.checked_mul(y))
}

#[unsafe(no_mangle)]
pub extern "C" fn rt_num_div(a: i32, b: i32) -> i32 {
    let zero = with(|s| numbers(s, a, b).1.is_zero());
    if zero {
        with(|s| {
            fail(s, "it divided by zero".into());
            put(s, Value::None)
        })
    } else {
        arithmetic(a, b, |x, y| x.checked_div(y))
    }
}

/// `round(value, places)`, half away from zero (conformance/rounding.json).
#[unsafe(no_mangle)]
pub extern "C" fn rt_round(value: i32, places: i64) -> i32 {
    with(|s| {
        let d = get(s, value).number().unwrap_or_default();
        let places = places.clamp(0, 28) as u32;
        let rounded = if d.scale() <= places { d } else { d.round_dp_with_strategy(places, rust_decimal::RoundingStrategy::MidpointAwayFromZero) };
        put(s, Value::Dec(rounded))
    })
}

/// `sin(degrees)` and `cos(degrees)`, rounded to 10 decimals (conformance/drawing.json).
fn angle(degrees: i32, of: fn(f64) -> f64) -> i32 {
    with(|s| {
        let radians = get(s, degrees).number().unwrap_or_default().to_f64().unwrap_or(0.0).to_radians();
        let value = Decimal::from_f64_retain(of(radians)).unwrap_or_default();
        let rounded = if value.scale() <= 10 { value } else { value.round_dp_with_strategy(10, rust_decimal::RoundingStrategy::MidpointAwayFromZero) };
        put(s, Value::Dec(rounded.normalize()))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rt_sin(degrees: i32) -> i32 {
    angle(degrees, f64::sin)
}

#[unsafe(no_mangle)]
pub extern "C" fn rt_cos(degrees: i32) -> i32 {
    angle(degrees, f64::cos)
}

// Random.

/// `random(max)`: 1 to `max` from the host's next number.
#[unsafe(no_mangle)]
pub extern "C" fn rt_random(next: i64, max: i64) -> i64 {
    if max < 1 {
        with(|s| fail(s, format!("random takes a highest number of 1 or more, not {max}")));
        return 0;
    }
    1 + ((next as u64) % (max as u64)) as i64
}

// Records, their fields by name.

fn name_at(at: i32, len: i32) -> String {
    let bytes = unsafe { std::slice::from_raw_parts(at as *const u8, len as usize) };
    String::from_utf8_lossy(bytes).into_owned()
}

/// A new record without fields; `new` gives it each field at its default.
#[unsafe(no_mangle)]
pub extern "C" fn rt_record() -> i32 {
    with(|s| put(s, Value::Record(Vec::new())))
}

/// The record with field `name` (at `at`) set to `value`, as a new handle.
#[unsafe(no_mangle)]
pub extern "C" fn rt_with(record: i32, at: i32, len: i32, value: i32) -> i32 {
    let name = name_at(at, len);
    with(|s| {
        let mut fields = if let Value::Record(fields) = get(s, record) { fields } else { Vec::new() };
        let value = get(s, value);
        match fields.iter_mut().find(|(n, _)| *n == name) {
            Some(field) => field.1 = value,
            None => fields.push((name, value)),
        }
        put(s, Value::Record(fields))
    })
}

fn field(s: &State, record: i32, at: i32, len: i32) -> Value {
    let name = name_at(at, len);
    match get(s, record) {
        Value::Record(fields) => fields.into_iter().find(|(n, _)| *n == name).map(|(_, v)| v).unwrap_or(Value::None),
        _ => Value::None,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn rt_field(record: i32, at: i32, len: i32) -> i32 {
    with(|s| {
        let value = field(s, record, at, len);
        put(s, value)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rt_field_int(record: i32, at: i32, len: i32) -> i64 {
    with(|s| match field(s, record, at, len) {
        Value::Int(n) => n,
        other => other.number().and_then(|d| d.to_i64()).unwrap_or(0),
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rt_field_bool(record: i32, at: i32, len: i32) -> i32 {
    with(|s| matches!(field(s, record, at, len), Value::Bool(true)) as i32)
}

// Tells, for an engine beside the screen (DESIGN 5.12).

/// `tell name(values)`: a tell with its values, in the answer of this call.
#[unsafe(no_mangle)]
pub extern "C" fn rt_tell(at: i32, len: i32, values: i32) {
    let name = name_at(at, len);
    with(|s| {
        let values = match get(s, values) {
            Value::List(items) => Json::Array(items.iter().map(Value::json).collect()),
            _ => Json::Array(Vec::new()),
        };
        s.call.tells.push(json!({ "tell": name, "values": values }));
    });
}
