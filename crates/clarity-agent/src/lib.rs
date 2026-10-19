//! Clarity's agent: a conversation with a model that works on a project
//! through tools. The model answers, asks for tools (read a file, write
//! one, check the project), gets their results and goes on, until it has
//! answered or a turn reaches its limit of steps.
//!
//! The loop knows no model and no project: a model is anything that
//! implements [`Model`] (Anthropic's comes with it), and the tools are
//! whatever implements [`Tools`]. Clarity's app gives the tools that work on
//! the open project.

pub mod anthropic;

use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
}

/// A piece of a message: text, a tool call, or a tool's result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Block {
    Text {
        text: String,
    },
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
    ToolResult {
        tool_use_id: String,
        content: String,
        #[serde(default)]
        is_error: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: Vec<Block>,
}

/// A tool as the model is told of it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    /// The JSON Schema of its input.
    pub input_schema: Value,
}

/// What a model is asked: its instructions, the conversation and the tools.
#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    pub system: String,
    pub messages: Vec<Message>,
    pub tools: Vec<ToolDef>,
}

/// Why a model stopped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StopReason {
    EndTurn,
    ToolUse,
    MaxTokens,
    Other(String),
}

/// The tokens one answer of a model cost, as the model counts them. The
/// prompt is three parts: what was read uncached, what was written to the
/// cache, and what was read from it (the cheap part).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct Usage {
    /// The prompt's tokens read without the cache.
    #[ts(type = "number")]
    pub input: u64,
    /// The prompt's tokens written to the cache.
    #[ts(type = "number")]
    pub cache_write: u64,
    /// The prompt's tokens read from the cache.
    #[ts(type = "number")]
    pub cache_read: u64,
    /// The answer's tokens.
    #[ts(type = "number")]
    pub output: u64,
}

impl Usage {
    /// The whole prompt, cached or not.
    pub fn prompt(&self) -> u64 {
        self.input + self.cache_write + self.cache_read
    }

    /// What costs full price: the prompt not read from the cache, and the
    /// answer. What a turn's budget counts.
    pub fn billed(&self) -> u64 {
        self.input + self.cache_write + self.output
    }

    /// Whether the model told anything.
    pub fn is_empty(&self) -> bool {
        *self == Usage::default()
    }
}

impl std::ops::Add for Usage {
    type Output = Usage;

    fn add(self, other: Usage) -> Usage {
        Usage {
            input: self.input + other.input,
            cache_write: self.cache_write + other.cache_write,
            cache_read: self.cache_read + other.cache_read,
            output: self.output + other.output,
        }
    }
}

/// A model's answer: its blocks, why it stopped, and what it cost.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelReply {
    pub content: Vec<Block>,
    pub stop: StopReason,
    pub usage: Usage,
}

/// A model to ask. `on_text` gets the answer's text as it comes.
pub trait Model {
    fn reply(&mut self, request: &Request, on_text: &mut dyn FnMut(&str)) -> Result<ModelReply, String>;
}

/// What a tool gives back to the model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolResult {
    pub text: String,
    pub is_error: bool,
}

/// The tools the model may use.
pub trait Tools {
    fn definitions(&self) -> Vec<ToolDef>;
    fn call(&mut self, name: &str, input: &Value) -> ToolResult;
}

/// What happens during a turn, for the screen.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export, rename = "AgentEvent")]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Event {
    /// A piece of the answer's text.
    Text { text: String },
    /// What an answer of the model cost, before its tools run.
    Usage { usage: Usage },
    /// A tool is called with this input.
    ToolStart {
        id: String,
        name: String,
        #[ts(type = "unknown")]
        input: Value,
    },
    /// A tool gave this back.
    ToolDone {
        id: String,
        name: String,
        text: String,
        #[serde(rename = "isError")]
        #[ts(rename = "isError")]
        is_error: bool,
    },
    /// The model has answered.
    Finished,
    /// The person stopped the turn.
    Stopped,
    /// The turn spent its budget of tokens at full price (`Usage::billed`).
    #[serde(rename_all = "camelCase")]
    OverBudget {
        #[ts(type = "number")]
        spent: u64,
        #[ts(type = "number")]
        budget: u64,
    },
    /// The turn ended on an error.
    Failed { message: String },
}

/// How the agent works: its instructions, and the limits of a turn: its
/// steps, and the tokens it may spend at full price.
pub struct Agent {
    pub system: String,
    pub max_steps: usize,
    pub budget: Option<u64>,
}

impl Agent {
    /// One turn: `user` joins the conversation, and the model answers with
    /// tools until it is done, `stop` is set, or the steps run out. The
    /// conversation keeps every message, ready for the next turn.
    pub fn run(
        &self,
        model: &mut dyn Model,
        tools: &mut dyn Tools,
        conversation: &mut Vec<Message>,
        user: &str,
        stop: &AtomicBool,
        events: &mut dyn FnMut(Event),
    ) -> Result<(), String> {
        conversation.push(Message { role: Role::User, content: vec![Block::Text { text: user.into() }] });
        let definitions = tools.definitions();
        let mut spent = 0;
        for _ in 0..self.max_steps {
            if stop.load(Ordering::SeqCst) {
                events(Event::Stopped);
                return Ok(());
            }
            let request = Request { system: self.system.clone(), messages: conversation.clone(), tools: definitions.clone() };
            let reply = match model.reply(&request, &mut |text| events(Event::Text { text: text.into() })) {
                Ok(reply) => reply,
                Err(message) => {
                    events(Event::Failed { message: message.clone() });
                    return Err(message);
                }
            };
            if !reply.usage.is_empty() {
                events(Event::Usage { usage: reply.usage });
            }
            spent += reply.usage.billed();
            conversation.push(Message { role: Role::Assistant, content: reply.content.clone() });
            let calls: Vec<(String, String, Value)> = reply
                .content
                .iter()
                .filter_map(|b| match b {
                    Block::ToolUse { id, name, input } => Some((id.clone(), name.clone(), input.clone())),
                    _ => None,
                })
                .collect();
            if calls.is_empty() {
                events(match reply.stop {
                    StopReason::MaxTokens => Event::Failed { message: "the answer reached its length limit".into() },
                    _ => Event::Finished,
                });
                return Ok(());
            }
            // Every call gets its result, so the conversation stays whole even when stopped.
            let mut results = Vec::new();
            for (id, name, input) in calls {
                events(Event::ToolStart { id: id.clone(), name: name.clone(), input: input.clone() });
                let result = tools.call(&name, &input);
                events(Event::ToolDone { id: id.clone(), name, text: result.text.clone(), is_error: result.is_error });
                results.push(Block::ToolResult { tool_use_id: id, content: result.text, is_error: result.is_error });
            }
            conversation.push(Message { role: Role::User, content: results });
            // Every call has its result, so the next turn can go on from here.
            if let Some(budget) = self.budget
                && spent >= budget
            {
                events(Event::OverBudget { spent, budget });
                return Ok(());
            }
        }
        if stop.load(Ordering::SeqCst) {
            events(Event::Stopped);
            return Ok(());
        }
        events(Event::Failed { message: format!("stopped after {} steps; ask again to go on", self.max_steps) });
        Ok(())
    }
}
