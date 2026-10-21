//! What an engine written in Rust stands on (DESIGN 5.11). An engine is a
//! plain WebAssembly module, built as a `cdylib` for
//! `wasm32-unknown-unknown`, that speaks Clarity's engine interface: the host
//! writes a call's arguments as a JSON array into the engine's memory, and
//! the engine answers `{"value": ...}` or `{"error": "why"}`.
//!
//! ```ignore
//! clarity_engine::interface!();
//!
//! clarity_engine::function!(add, |(a, b): (i64, i64)| Ok(a + b));
//! clarity_engine::function!(greet, |(name,): (String,)| Ok(format!("Hello, {name}!")));
//! ```
//!
//! Arguments arrive as a tuple, in the order the declaration gives them, and
//! a result is anything serde writes: integers as numbers, decimals and money
//! as text, enums by name, records as objects of their fields, `Bytes` as
//! base64. An `Err` is the engine's refusal, in its words.

use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub use serde;
pub use serde_json;

/// Defines the engine interface, `clarity_alloc` and `clarity_free`. An
/// engine writes it once.
#[macro_export]
macro_rules! interface {
    () => {
        /// Room in the engine's memory for the host to write into.
        #[unsafe(no_mangle)]
        pub extern "C" fn clarity_alloc(len: i32) -> i32 {
            $crate::alloc(len)
        }

        /// Gives back room the engine handed out, once the host has read it.
        #[unsafe(no_mangle)]
        pub extern "C" fn clarity_free(at: i32, len: i32) {
            $crate::free(at, len)
        }
    };
}

/// Defines a function of the engine: `function!(total, |(lines,): (Vec<Line>,)| Ok(...))`.
#[macro_export]
macro_rules! function {
    ($name:ident, $function:expr) => {
        #[unsafe(no_mangle)]
        pub extern "C" fn $name(at: i32, len: i32) -> i64 {
            $crate::answer(at, len, $function)
        }
    };
}

/// Defines `on open`, which the host runs whenever it makes an instance.
#[macro_export]
macro_rules! on_open {
    (|| $body:expr) => {
        #[unsafe(no_mangle)]
        pub extern "C" fn clarity_open(at: i32, len: i32) -> i64 {
            $crate::answer(at, len, |(): ()| $body)
        }
    };
}

/// Room for `len` bytes, which the engine hands to the host.
pub fn alloc(len: i32) -> i32 {
    let mut room = Vec::<u8>::with_capacity(len.max(0) as usize);
    let at = room.as_mut_ptr();
    std::mem::forget(room);
    at as usize as i32
}

/// Gives back room that `alloc` or an answer handed out.
pub fn free(at: i32, len: i32) {
    if at != 0 {
        // SAFETY: the host gives back exactly the place and length it was handed.
        unsafe { drop(Vec::from_raw_parts(at as usize as *mut u8, 0, len.max(0) as usize)) };
    }
}

/// Runs `function` on the arguments at `at` (a JSON array the host wrote
/// there, which this takes over) and hands its answer to the host: the place
/// in the high 32 bits, the length in the low.
pub fn answer<A: DeserializeOwned, R: Serialize>(at: i32, len: i32, function: impl FnOnce(A) -> Result<R, String>) -> i64 {
    // SAFETY: the host wrote `len` bytes into room from `alloc`, and hands it over.
    let input = unsafe { Vec::from_raw_parts(at as usize as *mut u8, len.max(0) as usize, len.max(0) as usize) };
    let reply = match arguments::<A>(&input) {
        Ok(args) => match function(args).and_then(|value| serde_json::to_value(value).map_err(|e| e.to_string())) {
            Ok(value) => serde_json::json!({ "value": value }),
            Err(error) => serde_json::json!({ "error": error }),
        },
        Err(error) => serde_json::json!({ "error": format!("the arguments do not fit: {error}") }),
    };
    let mut out = serde_json::to_vec(&reply).unwrap_or_default().into_boxed_slice();
    let (place, size) = (out.as_mut_ptr() as usize as i64, out.len() as i64);
    std::mem::forget(out);
    (place << 32) | size
}

/// The arguments of a call, from the JSON array the host wrote; an empty
/// array is also no arguments at all, `()`.
fn arguments<A: DeserializeOwned>(input: &[u8]) -> Result<A, serde_json::Error> {
    let value: serde_json::Value = serde_json::from_slice(input)?;
    if value.as_array().is_some_and(|a| a.is_empty())
        && let Ok(none) = serde_json::from_value::<A>(serde_json::Value::Null)
    {
        return Ok(none);
    }
    serde_json::from_value(value)
}

/// Bytes, as they cross: base64 text.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Bytes(pub Vec<u8>);

impl Serialize for Bytes {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use base64::Engine;
        serializer.serialize_str(&base64::engine::general_purpose::STANDARD.encode(&self.0))
    }
}

impl<'de> Deserialize<'de> for Bytes {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use base64::Engine;
        let text = String::deserialize(deserializer)?;
        base64::engine::general_purpose::STANDARD.decode(text).map(Bytes).map_err(serde::de::Error::custom)
    }
}

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "clarity")]
unsafe extern "C" {
    #[link_name = "clock"]
    fn clarity_clock() -> i64;
    #[link_name = "random"]
    fn clarity_random() -> i64;
    #[link_name = "log"]
    fn clarity_log(at: *const u8, len: i32);
    #[link_name = "language"]
    fn clarity_language(at: *mut u8, room: i32) -> i32;
}

/// The time, in milliseconds since 1970-01-01 UTC (`uses clock`).
pub fn clock() -> i64 {
    #[cfg(target_arch = "wasm32")]
    {
        // SAFETY: a service of the host, which takes and gives plain numbers.
        unsafe { clarity_clock() }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as i64)
    }
}

/// The next number of the call's random sequence (`uses random`).
pub fn random() -> u64 {
    #[cfg(target_arch = "wasm32")]
    {
        // SAFETY: a service of the host, which gives a plain number.
        unsafe { clarity_random() as u64 }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        0
    }
}

/// A line in the application's log (`uses log`).
pub fn log(line: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        // SAFETY: the host reads `len` bytes of UTF-8 at the place it is given.
        unsafe { clarity_log(line.as_ptr(), line.len() as i32) }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        eprintln!("{line}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_list_of_arguments_is_none() {
        assert!(arguments::<()>(b"[]").is_ok());
        assert_eq!(arguments::<(i64, i64)>(b"[2, 40]").unwrap(), (2, 40));
        assert!(arguments::<(i64,)>(b"[]").is_err());
    }

    #[test]
    fn bytes_cross_as_base64() {
        let bytes = Bytes(vec![1, 2, 3]);
        assert_eq!(serde_json::to_value(&bytes).unwrap(), serde_json::json!("AQID"));
        assert_eq!(serde_json::from_value::<Bytes>(serde_json::json!("AQID")).unwrap(), bytes);
    }
}

/// The language the person picked, as its tag such as `nl` (`uses
/// language`); none for the texts as the project writes them.
pub fn language() -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    {
        let mut room = [0u8; 64];
        // SAFETY: the host writes at most `room.len()` bytes at the place it is given.
        let len = unsafe { clarity_language(room.as_mut_ptr(), room.len() as i32) }.max(0) as usize;
        if len == 0 {
            return None;
        }
        let mut tag = room[..len.min(room.len())].to_vec();
        if len > room.len() {
            // A longer tag than any language has: ask again with room for all of it.
            tag = vec![0u8; len];
            // SAFETY: as above, with room for the whole tag.
            unsafe { clarity_language(tag.as_mut_ptr(), len as i32) };
        }
        String::from_utf8(tag).ok()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        None
    }
}
