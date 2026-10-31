//! The dropdown's options of U1c, decided here, not in the screen
//! (DESIGN 1.7). A `typed` dropdown offers the values its field already
//! has: each once, without empty text, those containing the typed text
//! (ignoring case), sorted ignoring case and then as written. A `search`
//! dropdown offers the records whose text or shown fields hold every typed
//! word, ignoring case, in the order they came. Every runtime offers alike
//! (conformance/dropdown-options.json).

use crate::ui::RefOption;
use crate::{Error, Result};
use serde_json::Value;

/// The suggestions of a typed dropdown for what is typed.
pub fn suggestions(values: &[String], typed: &str) -> Vec<String> {
    let wanted = typed.trim().to_lowercase();
    let mut once: Vec<String> = vec![];
    for v in values {
        if !v.is_empty() && !once.contains(v) && v.to_lowercase().contains(&wanted) {
            once.push(v.clone());
        }
    }
    once.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()).then_with(|| a.cmp(b)));
    once
}

/// A shown field's value as text to search in.
fn text_of(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// The records a search dropdown offers for what is typed, each with the
/// fields of `show` only.
pub fn searched(options: Vec<RefOption>, show: &[String], typed: &str) -> Vec<RefOption> {
    let words: Vec<String> = typed.to_lowercase().split_whitespace().map(str::to_string).collect();
    options
        .into_iter()
        .map(|mut o| {
            o.fields.retain(|name, _| show.contains(name));
            o
        })
        .filter(|o| {
            let texts: Vec<String> = std::iter::once(o.text.to_lowercase()).chain(show.iter().map(|f| text_of(o.fields.get(f).unwrap_or(&Value::Null)).to_lowercase())).collect();
            words.iter().all(|w| texts.iter().any(|t| t.contains(w.as_str())))
        })
        .collect()
}

/// `find_options` (DESIGN 1.7): what a dropdown of `kind` offers for
/// `typed`: for `typed`, the field's `values`; for `search`, the entity's
/// `options` with the fields of `show`. Answers the list.
pub fn find(kind: &str, show: &[String], typed: &str, values: impl FnOnce() -> Result<Vec<String>>, options: impl FnOnce() -> Result<Vec<RefOption>>) -> Result<Value> {
    let answer = match kind {
        "typed" => serde_json::to_value(suggestions(&values()?, typed)),
        "search" => serde_json::to_value(searched(options()?, show, typed)),
        other => return Err(Error::Invalid(format!("a dropdown offers typed or search options, not {other}"))),
    };
    answer.map_err(|err| Error::Invalid(err.to_string()))
}
