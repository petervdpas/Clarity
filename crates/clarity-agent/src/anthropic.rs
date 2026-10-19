//! Anthropic's Messages API as a [`Model`]: the request as JSON, sent with
//! the answer streamed, its text passed on as it comes and its tool calls
//! put together from their pieces.

use std::io::{BufRead, BufReader, Read};

use serde_json::{Value, json};

use crate::{Block, Model, ModelReply, Request, StopReason, Usage};

/// The version of the API this client speaks.
const API_VERSION: &str = "2023-06-01";

/// A model behind Anthropic's API, as a connection describes it.
pub struct Anthropic {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub max_tokens: u32,
    client: reqwest::blocking::Client,
}

impl Anthropic {
    pub fn new(base_url: &str, api_key: &str, model: &str, max_tokens: u32) -> Result<Anthropic, String> {
        let client = reqwest::blocking::Client::builder()
            // A long answer streams for a while; only the connection itself has a limit.
            .connect_timeout(std::time::Duration::from_secs(20))
            .timeout(None)
            .build()
            .map_err(|err| format!("cannot make the connection: {err}"))?;
        Ok(Anthropic {
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.to_string(),
            model: model.to_string(),
            max_tokens,
            client,
        })
    }
}

impl Model for Anthropic {
    fn reply(&mut self, request: &Request, on_text: &mut dyn FnMut(&str)) -> Result<ModelReply, String> {
        let response = self
            .client
            .post(format!("{}/v1/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", API_VERSION)
            .json(&body(request, &self.model, self.max_tokens))
            .send()
            .map_err(|err| format!("cannot reach {}: {err}", self.base_url))?;
        let status = response.status();
        if !status.is_success() {
            let text = response.text().unwrap_or_default();
            let said = serde_json::from_str::<Value>(&text)
                .ok()
                .and_then(|v| v["error"]["message"].as_str().map(String::from))
                .unwrap_or(text);
            return Err(format!("the model refused the request ({status}): {said}"));
        }
        read_stream(response, on_text)
    }
}

/// The request as the API takes it, streamed. The instructions are the
/// same every turn, so they are marked for caching.
pub fn body(request: &Request, model: &str, max_tokens: u32) -> Value {
    json!({
        "model": model,
        "max_tokens": max_tokens,
        "stream": true,
        "system": [{ "type": "text", "text": request.system, "cache_control": { "type": "ephemeral" } }],
        "tools": request.tools,
        "messages": request.messages,
    })
}

/// A block being received.
enum Building {
    Text(String),
    Tool { id: String, name: String, json: String },
}

/// Reads a streamed answer (server-sent events), passing its text on as it comes.
pub fn read_stream(stream: impl Read, on_text: &mut dyn FnMut(&str)) -> Result<ModelReply, String> {
    let mut blocks: Vec<Building> = Vec::new();
    let mut stop = StopReason::Other("the stream ended early".into());
    let mut usage = Usage::default();
    for line in BufReader::new(stream).lines() {
        let line = line.map_err(|err| format!("the answer broke off: {err}"))?;
        let Some(data) = line.strip_prefix("data:") else { continue };
        let event: Value = serde_json::from_str(data.trim()).map_err(|err| format!("the answer is no valid JSON: {err}"))?;
        match event["type"].as_str().unwrap_or_default() {
            // The prompt's tokens come first; message_delta may repeat them.
            "message_start" => read_usage(&event["message"]["usage"], &mut usage),
            "content_block_start" => {
                let block = &event["content_block"];
                blocks.push(match block["type"].as_str() {
                    Some("tool_use") => Building::Tool {
                        id: block["id"].as_str().unwrap_or_default().into(),
                        name: block["name"].as_str().unwrap_or_default().into(),
                        json: String::new(),
                    },
                    _ => Building::Text(block["text"].as_str().unwrap_or_default().into()),
                });
            }
            "content_block_delta" => {
                let delta = &event["delta"];
                match (blocks.last_mut(), delta["type"].as_str()) {
                    (Some(Building::Text(text)), Some("text_delta")) => {
                        let piece = delta["text"].as_str().unwrap_or_default();
                        on_text(piece);
                        text.push_str(piece);
                    }
                    (Some(Building::Tool { json, .. }), Some("input_json_delta")) => json.push_str(delta["partial_json"].as_str().unwrap_or_default()),
                    _ => {}
                }
            }
            "message_delta" => {
                read_usage(&event["usage"], &mut usage);
                stop = match event["delta"]["stop_reason"].as_str() {
                    Some("end_turn") => StopReason::EndTurn,
                    Some("tool_use") => StopReason::ToolUse,
                    Some("max_tokens") => StopReason::MaxTokens,
                    Some(other) => StopReason::Other(other.into()),
                    None => stop,
                };
            }
            "error" => {
                let error = &event["error"];
                return Err(format!(
                    "the model said: {} ({})",
                    error["message"].as_str().unwrap_or("an error"),
                    error["type"].as_str().unwrap_or("error")
                ));
            }
            _ => {}
        }
    }
    let content = blocks
        .into_iter()
        .filter_map(|b| match b {
            Building::Text(text) if text.is_empty() => None,
            Building::Text(text) => Some(Ok(Block::Text { text })),
            Building::Tool { id, name, json } => {
                let input = if json.trim().is_empty() {
                    Ok(json!({}))
                } else {
                    serde_json::from_str(&json).map_err(|err| format!("the input of {name} is no valid JSON: {err}"))
                };
                Some(input.map(|input| Block::ToolUse { id, name, input }))
            }
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(ModelReply { content, stop, usage })
}

/// Takes the counts a usage object has; the answer's count is the total so far.
fn read_usage(from: &Value, usage: &mut Usage) {
    let count = |name: &str| from[name].as_u64();
    if let Some(n) = count("input_tokens") {
        usage.input = n;
    }
    if let Some(n) = count("cache_creation_input_tokens") {
        usage.cache_write = n;
    }
    if let Some(n) = count("cache_read_input_tokens") {
        usage.cache_read = n;
    }
    if let Some(n) = count("output_tokens") {
        usage.output = n;
    }
}
