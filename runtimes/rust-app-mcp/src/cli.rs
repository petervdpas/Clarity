//! A command-line program serves its commands as they are typed: a call
//! becomes the command line the program reads with its own parser, so
//! defaults, limits and kinds are read as at the terminal; a command's
//! `ask` (its form, as conformance/command-line.json writes it) is asked of
//! the agent, and the answer given back as the terminal gives it.

use serde_json::{Value, json};

use crate::server::{Answer, Ask};
use crate::tools::Tool;

/// The command line of a call: the command's words, its options as
/// `--name=value` (a true flag as `--name`, a false or null one left out),
/// then `--` and its arguments in their order.
pub fn command_line(tool: &Tool, arguments: &Value) -> Vec<String> {
    let mut out = tool.line.clone();
    let given = arguments.as_object();
    // By name, so the line is the same whatever order the call gave.
    let mut options: Vec<(&String, &Value)> = given.into_iter().flatten().filter(|(name, _)| !tool.arguments.contains(name)).collect();
    options.sort_by(|a, b| a.0.cmp(b.0));
    for (name, value) in options {
        match value {
            Value::Null | Value::Bool(false) => {}
            Value::Bool(true) => out.push(format!("--{name}")),
            other => out.push(format!("--{name}={}", text(other))),
        }
    }
    out.push("--".into());
    for name in &tool.arguments {
        out.push(given.and_then(|g| g.get(name)).map(text).unwrap_or_default());
    }
    out
}

/// A value as typed: a text as it is, a number as JSON writes it.
fn text(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// The JSON schema of the answer to the question `form`.
pub fn schema(form: &Value) -> Value {
    let mut schema = match form["kind"].as_str() {
        Some("confirm") => return json!({ "type": "boolean" }),
        Some("choice") => return json!({ "type": "string", "enum": form["choices"] }),
        _ => match form["type"].as_str().unwrap_or("text") {
            "integer" => json!({ "type": "integer" }),
            "decimal" | "money" => json!({ "type": "number" }),
            "bool" => json!({ "type": "boolean" }),
            "date" => json!({ "type": "string", "format": "date" }),
            "datetime" => json!({ "type": "string", "format": "date-time" }),
            "duration" => json!({ "type": "string", "description": "3:45, 1:02:03 or seconds" }),
            "enum" => json!({ "type": "string", "enum": form["values"] }),
            _ => json!({ "type": "string" }),
        },
    };
    if let Some(default) = form["default"].as_str() {
        let value = match schema["type"].as_str() {
            Some("integer" | "number") => serde_json::from_str::<Value>(default).ok().filter(Value::is_number),
            _ => Some(Value::String(default.to_string())),
        };
        if let Some(value) = value {
            schema["default"] = value;
        }
    }
    schema
}

/// The answer the command gets for `answer`, as the terminal gives it
/// (`true`, `5`, a choice's number from 1), or none.
pub fn answered(form: &Value, answer: &Answer) -> Option<String> {
    let default = || form["default"].as_str().map(str::to_string);
    let given = match answer {
        Answer::Given(given) => given,
        _ if form["kind"] == "confirm" => return Some("false".into()),
        _ => return default(),
    };
    let schema = schema(form);
    match (form["kind"].as_str(), schema["type"].as_str(), given) {
        (Some("confirm"), _, Value::Bool(yes)) => Some(yes.to_string()),
        (Some("choice"), _, Value::String(text)) => form["choices"].as_array()?.iter().position(|c| c == text).map(|i| (i + 1).to_string()),
        (Some("choice" | "confirm"), _, _) => None,
        (_, Some("integer"), Value::Number(n)) if n.is_i64() || n.is_u64() => Some(n.to_string()),
        (_, Some("number"), Value::Number(n)) => Some(n.to_string()),
        (_, Some("boolean"), Value::Bool(yes)) => Some(yes.to_string()),
        (_, Some("string"), Value::String(text)) => match schema["enum"].as_array() {
            Some(values) if !values.contains(given) => None,
            _ => Some(text.clone()),
        },
        _ => None,
    }
}

/// Asks the agent `question` of the kind `form`; the answer as the terminal
/// gives it, or none.
pub fn ask(ask: &mut dyn Ask, question: &str, form: &Value) -> Option<String> {
    answered(form, &ask.ask(question, &schema(form)))
}
