//! Whether a call's arguments fit its tool's inputs, before the person is
//! asked: each required one given, none the tool does not have, each of its
//! kind and within its minimum and maximum. What a value means (a date, a
//! duration) is the command line's to read.

use serde_json::Value;

use crate::tools::Tool;

/// What does not fit in `arguments` (an object, or nothing), first found.
pub fn misfit(tool: &Tool, arguments: &Value) -> Option<String> {
    let empty = serde_json::Map::new();
    let given = match arguments {
        Value::Null => &empty,
        Value::Object(given) => given,
        _ => return Some(format!("the arguments of {} are an object", tool.name)),
    };
    let properties = tool.input_schema.get("properties").and_then(Value::as_object).unwrap_or(&empty);
    // A null is the empty value: as if left out.
    let given: serde_json::Map<String, Value> = given.iter().filter(|(_, v)| !v.is_null()).map(|(k, v)| (k.clone(), v.clone())).collect();
    for required in tool.input_schema.get("required").and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str) {
        if !given.contains_key(required) {
            return Some(format!("{} needs the argument {required}", tool.name));
        }
    }
    for (name, value) in &given {
        let Some(schema) = properties.get(name) else {
            return Some(format!("{} has no argument {name}", tool.name));
        };
        if let Some(wrong) = unfit(schema, value) {
            return Some(format!("the argument {name} of {} {wrong}", tool.name));
        }
    }
    None
}

/// What `value` should be, when `schema` does not take it.
fn unfit(schema: &Value, value: &Value) -> Option<String> {
    if let Some(values) = schema.get("enum").and_then(Value::as_array) {
        if !values.contains(value) {
            let words: Vec<String> = values.iter().map(|v| v.as_str().map(str::to_string).unwrap_or_else(|| v.to_string())).collect();
            return Some(format!("is one of {}", words.join(", ")));
        }
        return None;
    }
    let fits = match schema.get("type").and_then(Value::as_str) {
        Some("string") => value.is_string().then_some(()).ok_or("is a text"),
        Some("boolean") => value.is_boolean().then_some(()).ok_or("is true or false"),
        Some("number") => value.is_number().then_some(()).ok_or("is a number"),
        Some("integer") => whole(value).then_some(()).ok_or("is a whole number"),
        _ => Ok(()),
    };
    if let Err(wrong) = fits {
        return Some(wrong.to_string());
    }
    let number = value.as_f64()?;
    if let Some(min) = schema.get("minimum").filter(|m| m.as_f64().is_some_and(|m| number < m)) {
        return Some(format!("is at least {min}"));
    }
    if let Some(max) = schema.get("maximum").filter(|m| m.as_f64().is_some_and(|m| number > m)) {
        return Some(format!("is at most {max}"));
    }
    None
}

fn whole(value: &Value) -> bool {
    value.is_i64() || value.is_u64()
}
