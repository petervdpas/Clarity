//! The data formats of code (docs/FORMATS-PLAN.md): `read Order from json
//! text` and `json of value`, through Clarity.Formats, the one
//! implementation every stack calls. The library reads the format and fits
//! it to what the program wants; the record's own decoder (serde) reads
//! what was fitted. Writing goes the other way: serde writes the value,
//! the library writes it in the format, in Clarity's one form.

use serde::{Serialize, de::DeserializeOwned};

use crate::{Error, Result};

/// A new record (or any value) as serde writes it, which a kind carries as its base.
pub fn base<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string())
}

/// A text as a JSON string, for an option's value.
pub fn quoted(text: &str) -> String {
    serde_json::to_string(text).unwrap_or_else(|_| "\"\"".to_string())
}

/// Reads `input` in `format` with its `options` (a JSON object, empty for
/// none) as `kind` describes it, and decodes what was fitted as `T`.
pub fn read<T: DeserializeOwned>(format: &str, input: &str, options: &str, kind: &str) -> Result<T> {
    let fitted = clarity_formats::read_json(format, input.as_bytes(), options, kind).map_err(|p| Error::Value(p.text))?;
    serde_json::from_str(&fitted).map_err(|e| Error::Value(format!("what was read does not fit: {e}")))
}

/// `value` written in `format` with its `options`, as `kind` describes it.
pub fn write<T: Serialize + ?Sized>(format: &str, value: &T, options: &str, kind: &str) -> String {
    let encoded = serde_json::to_string(value).unwrap_or_default();
    match clarity_formats::write_json(format, &encoded, options, kind) {
        Ok(bytes) => String::from_utf8(bytes).unwrap_or_default(),
        // A kind the generator wrote always describes the value; should it not, the value as serde wrote it.
        Err(_) => encoded,
    }
}
