//! The Anthropic client: the request it sends and the streamed reply it
//! reads, from a recorded stream, so the test needs no network.

use clarity_agent::anthropic::{Cache, body, read_stream};
use clarity_agent::Usage;
use clarity_agent::{Block, Message, Request, Role, StopReason, ToolDef};
use serde_json::json;

#[test]
fn the_request_carries_the_instructions_cached_the_tools_and_the_conversation() {
    let request = Request {
        system: "You work on Clarity projects.".into(),
        messages: vec![Message { role: Role::User, content: vec![Block::Text { text: "Make a todo app".into() }] }],
        tools: vec![ToolDef { name: "check".into(), description: "Checks the project.".into(), input_schema: json!({"type": "object"}) }],
    };
    let sent = body(&request, "claude-sonnet-5-5", 16384, Cache::Minutes);
    assert_eq!(
        sent,
        json!({
            "model": "claude-sonnet-5-5",
            "max_tokens": 16384,
            "stream": true,
            // The instructions are the same every turn: cached on their own.
            "system": [{ "type": "text", "text": "You work on Clarity projects.", "cache_control": { "type": "ephemeral" } }],
            "tools": [{ "name": "check", "description": "Checks the project.", "input_schema": { "type": "object" } }],
            "messages": [{ "role": "user", "content": [{ "type": "text", "text": "Make a todo app" }] }],
            // The conversation so far is cached too, up to its end, so each
            // step of a turn reads what came before at the cache's price.
            "cache_control": { "type": "ephemeral" }
        })
    );
    // An hour's cache, for turns further apart than five minutes, on both marks.
    let long = body(&request, "claude-sonnet-5-5", 16384, Cache::Hour);
    assert_eq!(long["cache_control"], json!({ "type": "ephemeral", "ttl": "1h" }));
    assert_eq!(long["system"][0]["cache_control"], json!({ "type": "ephemeral", "ttl": "1h" }));
    assert_eq!(Cache::from_field("1h"), Some(Cache::Hour));
    assert_eq!(Cache::from_field(" 5m "), Some(Cache::Minutes));
    assert_eq!(Cache::from_field(""), Some(Cache::Minutes));
    assert_eq!(Cache::from_field("forever"), None);
}

const STREAM: &str = r#"event: message_start
data: {"type":"message_start","message":{"id":"msg_1","type":"message","role":"assistant","content":[],"stop_reason":null,"usage":{"input_tokens":25,"cache_creation_input_tokens":1200,"cache_read_input_tokens":18000,"output_tokens":1}}}

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
data: {"type":"message_delta","delta":{"stop_reason":"tool_use"},"usage":{"output_tokens":42}}

event: message_stop
data: {"type":"message_stop"}
"#;

#[test]
fn a_streamed_reply_is_read_with_its_text_as_it_comes_and_its_tool_calls() {
    let mut shown = String::new();
    let reply = read_stream(STREAM.as_bytes(), &mut |t| shown.push_str(t)).unwrap();
    assert_eq!(shown, "I will check it.");
    assert_eq!(reply.stop, StopReason::ToolUse);
    // What the answer cost: the prompt's uncached part, what was written to
    // and read from the cache, and the answer (the count message_delta ends with).
    assert_eq!(reply.usage, Usage { input: 25, cache_write: 1200, cache_read: 18000, output: 42 });
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

/// A Messages API that answers one request with a short stream and gives back its headers.
fn messages_server() -> (String, std::thread::JoinHandle<Vec<String>>) {
    use std::io::{BufRead, BufReader, Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://127.0.0.1:{}/anthropic", listener.local_addr().unwrap().port());
    let served = std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut reader = BufReader::new(stream);
        let mut headers = Vec::new();
        let mut length = 0;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            if line.trim().is_empty() {
                break;
            }
            if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                length = value.trim().parse().unwrap();
            }
            headers.push(line.trim().to_string());
        }
        let mut body = vec![0; length];
        reader.read_exact(&mut body).unwrap();
        let stream = concat!(
            "data: {\"type\":\"message_start\",\"message\":{\"usage\":{\"input_tokens\":1,\"output_tokens\":1}}}\n\n",
            "data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"output_tokens\":1}}\n\n",
            "data: {\"type\":\"message_stop\"}\n\n"
        );
        write!(reader.get_mut(), "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{stream}", stream.len()).unwrap();
        headers
    });
    (base, served)
}

fn ask(model: &mut clarity_agent::anthropic::Anthropic) {
    use clarity_agent::Model;
    let request = Request { system: "s".into(), messages: vec![Message { role: Role::User, content: vec![Block::Text { text: "hi".into() }] }], tools: vec![] };
    model.reply(&request, &mut |_| {}).unwrap();
}

#[test]
fn a_key_goes_in_x_api_key() {
    use clarity_agent::anthropic::{Anthropic, Auth};
    let (base, served) = messages_server();
    ask(&mut Anthropic::new(&base, Auth::Key("sk-1".into()), "claude-sonnet-5-5", 100, Cache::Minutes).unwrap());
    let headers = served.join().unwrap();
    assert!(headers.iter().any(|h| h.starts_with("POST /anthropic/v1/messages ")), "{headers:?}");
    assert!(headers.iter().any(|h| h == "x-api-key: sk-1"), "{headers:?}");
    assert!(!headers.iter().any(|h| h.to_ascii_lowercase().starts_with("authorization:")), "{headers:?}");
}

#[test]
fn a_microsoft_sign_in_sends_its_token_as_bearer() {
    use clarity_agent::anthropic::{Anthropic, Auth};
    let (base, served) = messages_server();
    let mut asked = 0;
    let source = move || {
        asked += 1;
        Ok(format!("token-{asked}"))
    };
    ask(&mut Anthropic::new(&base, Auth::Token(Box::new(source)), "my-deployment", 100, Cache::Minutes).unwrap());
    let headers = served.join().unwrap();
    assert!(headers.iter().any(|h| h == "authorization: Bearer token-1"), "{headers:?}");
    assert!(!headers.iter().any(|h| h.starts_with("x-api-key")), "{headers:?}");
}
