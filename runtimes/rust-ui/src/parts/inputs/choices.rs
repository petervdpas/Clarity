//! What a dropdown or a radio group offers and which of it is its value,
//! shared by the parts that pick one of a list.

use serde_json::Value;

use crate::parts::inputs::formats::Exact;
use crate::parts::{Choice, PartHost};

/// The choices of a part bound to `bound`: the host's (an enum's values,
/// a reference's records, the offered languages and themes of the
/// settings); none for a part that binds nothing.
pub fn offered_for(host: &dyn PartHost, bound: Option<&str>) -> Vec<Choice> {
    match bound {
        Some(path) => host.choices(path),
        None => Vec::new(),
    }
}

/// Whether a choice's value is the value a part has: texts alike, numbers
/// of the same amount (`2` and `2.0`), anything else equal. None is only
/// none.
pub fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, _) | (_, Value::Null) => a.is_null() && b.is_null(),
        (Value::String(x), Value::String(y)) => x == y,
        (Value::String(_), _) | (_, Value::String(_)) => false,
        _ => match (Exact::of(a), Exact::of(b)) {
            (Some(x), Some(y)) => {
                let scale = x.scale.max(y.scale);
                x.rounded(scale) == y.rounded(scale)
            }
            _ => a == b,
        },
    }
}

/// Where `value` is among `choices`; none when it is none of them.
pub fn chosen(choices: &[Choice], value: &Value) -> Option<usize> {
    choices.iter().position(|c| same(&c.value, value))
}
