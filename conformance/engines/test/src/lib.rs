//! The engine every host is tested with (conformance/engines.json). It
//! speaks Clarity's engine interface (DESIGN 5.11): the host writes the
//! arguments as a JSON array into the engine's memory, calls the function
//! with its place and length, and reads `{"value": ...}` or `{"error": ...}`
//! back from the place and length the function gives, packed in an i64.

use std::str::FromStr;

use rust_decimal::Decimal;
use serde_json::{Value, json};

// The services of Clarity an engine may use (`uses clock, random, log, language`).
#[link(wasm_import_module = "clarity")]
unsafe extern "C" {
    /// Milliseconds since 1970-01-01 UTC.
    fn clock() -> i64;
    /// The next number of the call's random sequence (splitmix64).
    fn random() -> i64;
    /// A line of UTF-8 text into the application's log.
    fn log(at: *const u8, len: i32);
    /// The language the screen picked: up to `room` bytes at `at`, and its whole length.
    fn language(at: *mut u8, room: i32) -> i32;
}

/// Room in the engine's memory for the host to write into.
#[unsafe(no_mangle)]
pub extern "C" fn clarity_alloc(len: i32) -> i32 {
    let mut room = Vec::<u8>::with_capacity(len.max(0) as usize);
    let at = room.as_mut_ptr();
    std::mem::forget(room);
    at as i32
}

/// Gives back room the engine handed out, once the host has read it.
#[unsafe(no_mangle)]
pub extern "C" fn clarity_free(at: i32, len: i32) {
    if at != 0 {
        unsafe { drop(Vec::from_raw_parts(at as *mut u8, 0, len.max(0) as usize)) };
    }
}

/// Runs `function` on the arguments at `at`, which it takes over, and hands
/// its answer to the host.
fn answer(at: i32, len: i32, function: fn(&[Value]) -> Result<Value, String>) -> i64 {
    let bytes = unsafe { Vec::from_raw_parts(at as *mut u8, len as usize, len as usize) };
    let reply = match serde_json::from_slice::<Vec<Value>>(&bytes) {
        Ok(args) => match function(&args) {
            Ok(value) => json!({ "value": value }),
            Err(error) => json!({ "error": error }),
        },
        Err(error) => json!({ "error": format!("the arguments are no JSON list: {error}") }),
    };
    let mut out = serde_json::to_vec(&reply).unwrap_or_default().into_boxed_slice();
    let (place, size) = (out.as_mut_ptr() as i64, out.len() as i64);
    std::mem::forget(out);
    (place << 32) | size
}

fn integer(v: &Value) -> Result<i64, String> {
    v.as_i64().ok_or_else(|| format!("{v} is no integer"))
}

fn decimal(v: &Value) -> Result<Decimal, String> {
    v.as_str().and_then(|t| Decimal::from_str(t).ok()).ok_or_else(|| format!("{v} is no decimal"))
}

fn text(v: &Value) -> Result<&str, String> {
    v.as_str().ok_or_else(|| format!("{v} is no text"))
}

macro_rules! function {
    ($name:ident, $body:expr) => {
        #[unsafe(no_mangle)]
        pub extern "C" fn $name(at: i32, len: i32) -> i64 {
            answer(at, len, $body)
        }
    };
}

// add(a: integer, b: integer) -> integer
function!(add, |args| Ok(json!(integer(&args[0])? + integer(&args[1])?)));

// greet(name: text) -> text
function!(greet, |args| Ok(json!(format!("Hello, {}!", text(&args[0])?))));

// total(lines: list<Line>) -> money: the sum of price * count.
function!(total, |args| {
    let lines = args[0].as_array().ok_or("lines is no list")?;
    let mut sum = Decimal::ZERO;
    for line in lines {
        sum += decimal(&line["price"])? * Decimal::from(integer(&line["count"])?);
    }
    Ok(json!(sum.to_string()))
});

// size_of(n: integer) -> Size
function!(size_of, |args| Ok(json!(if integer(&args[0])? < 10 { "Small" } else { "Large" })));

// discounted(order: Order) -> Order: its total less 10%, rounded to cents.
function!(discounted, |args| {
    let mut order = args[0].clone();
    let total = decimal(&order["total"])? * Decimal::new(9, 1);
    order["total"] = json!(total.round_dp_with_strategy(2, rust_decimal::RoundingStrategy::MidpointAwayFromZero).to_string());
    Ok(order)
});

// fail(why: text) -> text: refuses, with the reason given.
function!(fail, |args| Err(text(&args[0])?.to_string()));

// spin() -> integer: never finishes, so only a time limit stops it.
function!(spin, |_args| {
    let mut turns: u64 = 0;
    loop {
        turns = std::hint::black_box(turns.wrapping_add(1));
    }
});

// grow(mb: integer) -> integer: holds mb megabytes at once, so a memory
// limit below that stops it.
function!(grow, |args| {
    let mb = integer(&args[0])? as usize;
    let held = std::hint::black_box(vec![1u8; mb * 1024 * 1024]);
    Ok(json!(held.len() / (1024 * 1024)))
});

// now() -> integer: the clock of the host.
function!(now, |_args| Ok(json!(unsafe { clock() })));

// dice(n: integer, sides: integer) -> list<integer>: n throws of a die.
function!(dice, |args| {
    let (n, sides) = (integer(&args[0])?, integer(&args[1])? as u64);
    Ok(json!((0..n).map(|_| (unsafe { random() } as u64 % sides) + 1).collect::<Vec<_>>()))
});

// note(text: text) -> bool: writes the text into the log.
function!(note, |args| {
    let line = text(&args[0])?;
    unsafe { log(line.as_ptr(), line.len() as i32) };
    Ok(json!(true))
});

// speaks() -> text: the language the screen picked, "" for the texts as written.
function!(speaks, |_args| {
    let mut room = [0u8; 64];
    let len = unsafe { language(room.as_mut_ptr(), room.len() as i32) } as usize;
    Ok(json!(String::from_utf8_lossy(&room[..len.min(room.len())])))
});

/// How often `count` was called in this instance, from where `clarity_open` set it.
static COUNTED: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);

/// The hook `on open`: runs once when the host makes an instance, before its first call.
#[unsafe(no_mangle)]
pub extern "C" fn clarity_open(at: i32, len: i32) -> i64 {
    answer(at, len, |_args| {
        COUNTED.store(10, std::sync::atomic::Ordering::Relaxed);
        Ok(Value::Null)
    })
}

// count() -> integer: how often it was called, which an engine with state remembers.
function!(count, |_args| Ok(json!(COUNTED.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1)));

/// Bytes as they cross: base64 text.
fn bytes(v: &Value) -> Result<Vec<u8>, String> {
    use base64::Engine;
    let text = v.as_str().ok_or_else(|| format!("{v} is no bytes"))?;
    base64::engine::general_purpose::STANDARD.decode(text).map_err(|e| format!("{v} is no bytes: {e}"))
}

// reverse(b: bytes) -> bytes: the bytes backwards.
function!(reverse, |args| {
    use base64::Engine;
    let mut b = bytes(&args[0])?;
    b.reverse();
    Ok(json!(base64::engine::general_purpose::STANDARD.encode(b)))
});

// length(b: bytes) -> integer: how many bytes there are.
function!(length, |args| Ok(json!(bytes(&args[0])?.len())));
