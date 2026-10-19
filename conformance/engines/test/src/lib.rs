//! The engine every host is tested with (conformance/engines.json). It
//! speaks Clarity's engine interface (DESIGN 5.11): the host writes the
//! arguments as a JSON array into the engine's memory, calls the function
//! with its place and length, and reads `{"value": ...}` or `{"error": ...}`
//! back from the place and length the function gives, packed in an i64.

use std::str::FromStr;

use rust_decimal::Decimal;
use serde_json::{Value, json};

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
