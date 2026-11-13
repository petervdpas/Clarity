//! The JSON-RPC 2.0 messages of MCP, one a line.

use serde_json::{Value, json};

pub const PARSE_ERROR: i64 = -32700;
pub const INVALID_REQUEST: i64 = -32600;
pub const NO_METHOD: i64 = -32601;
pub const BAD_PARAMS: i64 = -32602;

pub fn result(id: &Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

pub fn error(id: &Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

pub fn request(id: &Value, method: &str, params: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params })
}

/// A tool's answer: its text, and whether it is an error.
pub fn tool_result(text: &str, is_error: bool) -> Value {
    json!({ "content": [{ "type": "text", "text": text }], "isError": is_error })
}

/// What a message is.
pub enum Message {
    Request { id: Value, method: String, params: Value },
    Notification,
    Response { id: Value, result: Option<Value> },
}

/// A line read; an error message to give back when it is no message.
pub fn parse(line: &str) -> Result<Message, Value> {
    let value: Value = serde_json::from_str(line).map_err(|_| error(&Value::Null, PARSE_ERROR, "the message is no JSON"))?;
    let Value::Object(object) = value else {
        return Err(error(&Value::Null, INVALID_REQUEST, "a message is one JSON object"));
    };
    let id = object.get("id").cloned();
    match (object.get("method").and_then(Value::as_str), id) {
        (Some(method), Some(id)) => Ok(Message::Request { id, method: method.to_string(), params: object.get("params").cloned().unwrap_or(Value::Null) }),
        (Some(_), None) => Ok(Message::Notification),
        (None, Some(id)) => Ok(Message::Response { id, result: object.get("result").cloned() }),
        (None, None) => Err(error(&Value::Null, INVALID_REQUEST, "a message is a request, a notification or a response")),
    }
}
