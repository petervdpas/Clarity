//! The Anthropic client: the request it sends and the streamed reply it
//! reads, from a recorded stream, so the test needs no network.

use clarity_agent::anthropic::{body, read_stream};
use clarity_agent::{Block, Message, Request, Role, StopReason, ToolDef};
use serde_json::json;

#[test]
fn the_request_carries_the_instructions_cached_the_tools_and_the_conversation() {
    let request = Request {
        system: "You work on Clarity projects.".into(),
        messages: vec![Message { role: Role::User, content: vec![Block::Text { text: "Make a todo app".into() }] }],
        tools: vec![ToolDef { name: "check".into(), description: "Checks the project.".into(), input_schema: json!({"type": "object"}) }],
    };
    let sent = body(&request, "claude-sonnet-5-5", 16384);
    assert_eq!(
        sent,
        json!({
            "model": "claude-sonnet-5-5",
            "max_tokens": 16384,
            "stream": true,
            // The instructions are the same every turn, so they are cached.
            "system": [{ "type": "text", "text": "You work on Clarity projects.", "cache_control": { "type": "ephemeral" } }],
            "tools": [{ "name": "check", "description": "Checks the project.", "input_schema": { "type": "object" } }],
            "messages": [{ "role": "user", "content": [{ "type": "text", "text": "Make a todo app" }] }]
        })
    );
}

const STREAM: &str = r#"event: message_start
data: {"type":"message_start","message":{"id":"msg_1","type":"message","role":"assistant","content":[],"stop_reason":null}}

event: content_block_start
data: {"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"I will "}}

event: ping
data: {"type":"ping"}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"check it."}}

event: content_block_stop
data: {"type":"content_block_stop","index":0}

event: content_block_start
data: {"type":"content_block_start","index":1,"content_block":{"type":"tool_use","id":"toolu_1","name":"read_file","input":{}}}

event: content_block_delta
data: {"type":"content_block_delta","index":1,"delta":{"type":"input_json_delta","partial_json":"{\"path\": \"to"}}

event: content_block_delta
data: {"type":"content_block_delta","index":1,"delta":{"type":"input_json_delta","partial_json":"do.clarity\"}"}}

event: content_block_stop
data: {"type":"content_block_stop","index":1}

event: message_delta
data: {"type":"message_delta","delta":{"stop_reason":"tool_use"}}

event: message_stop
data: {"type":"message_stop"}
"#;

#[test]
fn a_streamed_reply_is_read_with_its_text_as_it_comes_and_its_tool_calls() {
    let mut shown = String::new();
    let reply = read_stream(STREAM.as_bytes(), &mut |t| shown.push_str(t)).unwrap();
    assert_eq!(shown, "I will check it.");
    assert_eq!(reply.stop, StopReason::ToolUse);
    assert_eq!(
        reply.content,
        [
            Block::Text { text: "I will check it.".into() },
            Block::ToolUse { id: "toolu_1".into(), name: "read_file".into(), input: json!({"path": "todo.clarity"}) },
        ]
    );
}

#[test]
fn an_error_in_the_stream_is_said() {
    let stream = "event: error\ndata: {\"type\":\"error\",\"error\":{\"type\":\"overloaded_error\",\"message\":\"Overloaded\"}}\n\n";
    assert_eq!(read_stream(stream.as_bytes(), &mut |_| {}).unwrap_err(), "the model said: Overloaded (overloaded_error)");
}
