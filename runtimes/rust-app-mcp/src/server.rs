//! The service: lines of JSON read and written, the tool list served, and a
//! call run by the app (its `Host`) once the grant allows it. A command that
//! asks gets its answer from the agent through the client's elicitation.

use std::collections::VecDeque;

use serde_json::{Value, json};

use crate::grants::{Decision, Grants, Kind, Verdict};
use crate::inputs;
use crate::options::Options;
use crate::rpc::{self, Message};
use crate::tools::{Tool, ToolList};

/// The protocol versions the service speaks, the newest first.
pub const VERSIONS: [&str; 4] = ["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"];

/// Where the messages come from and go: standard input and output, a
/// socket, or a test's script. One message a line, without the newline.
pub trait Lines {
    fn read(&mut self) -> Option<String>;
    fn write(&mut self, line: &str);
}

/// What a command did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ran {
    /// What it printed.
    Printed(String),
    /// Its refusal (`refuse`, `allow when`, a rule of the data).
    Refused(String),
}

/// The agent's answer to a question a command asks.
#[derive(Debug, Clone, PartialEq)]
pub enum Answer {
    Given(Value),
    Declined,
    Cancelled,
    /// The agent's client cannot answer questions.
    Unsupported,
}

/// Asks the agent, for a command that asks.
pub trait Ask {
    /// `schema` is the answer's JSON schema (`{"type": "boolean"}`).
    fn ask(&mut self, message: &str, schema: &Value) -> Answer;
}

/// A grant the person is asked for.
#[derive(Debug, Clone, Copy)]
pub struct Grant<'a> {
    /// The agent's name as its client gives it (`initialize`), or empty.
    pub agent: &'a str,
    pub tool: &'a str,
    pub description: &'a str,
    pub kind: Kind,
    pub arguments: &'a Value,
    /// `{data, files, settings, net}`.
    pub changes: &'a Value,
}

impl Grant<'_> {
    /// The grant as JSON, for a host across the C interface.
    pub fn to_json(&self) -> Value {
        json!({ "agent": self.agent, "tool": self.tool, "description": self.description, "kind": self.kind.word(), "arguments": self.arguments, "changes": self.changes })
    }
}

/// The app: it runs a command by its tool's name, and asks the person.
pub trait Host {
    fn run(&mut self, tool: &Tool, arguments: &Value, ask: &mut dyn Ask) -> Ran;
    fn grant(&mut self, grant: &Grant) -> Decision;
}

/// One connection's service.
pub struct Server {
    list: ToolList,
    options: Options,
    grants: Grants,
    elicits: bool,
    /// The agent's name, from its client's `initialize`.
    agent: String,
    asked: u64,
    /// Messages read while a question waited for its answer.
    waiting: VecDeque<String>,
}

impl Server {
    /// From the tool list and the options, both JSON; a closed-app switch the
    /// options leave out (the person never set it) is what the project allows.
    pub fn new(tool_list: &str, options: &str) -> Result<Server, String> {
        let list = ToolList::from_json(tool_list)?;
        let mut options_read = Options::from_json(options)?;
        let given: serde_json::Value = serde_json::from_str(options).unwrap_or_default();
        let set = |key: &str| given.get(key).and_then(serde_json::Value::as_bool);
        (options_read.closed_reads, options_read.closed_changes) = list.closed_allows(set("closedReads"), set("closedChanges"));
        Ok(Server::with(list, options_read))
    }

    pub fn with(list: ToolList, options: Options) -> Server {
        Server { list, options, grants: Grants::default(), elicits: false, agent: String::new(), asked: 0, waiting: VecDeque::new() }
    }

    /// Serves until the lines end.
    pub fn serve(&mut self, lines: &mut dyn Lines, host: &mut dyn Host) {
        loop {
            let Some(line) = self.waiting.pop_front().or_else(|| lines.read()) else { return };
            if line.trim().is_empty() {
                continue;
            }
            if let Some(reply) = self.handle(&line, lines, host) {
                lines.write(&reply.to_string());
            }
        }
    }

    /// The answer to one line, if it gets one.
    fn handle(&mut self, line: &str, lines: &mut dyn Lines, host: &mut dyn Host) -> Option<Value> {
        let (id, method, params) = match rpc::parse(line) {
            Err(reply) => return Some(reply),
            Ok(Message::Notification | Message::Response { .. }) => return None,
            Ok(Message::Request { id, method, params }) => (id, method, params),
        };
        Some(match method.as_str() {
            "initialize" => rpc::result(&id, self.initialize(&params)),
            "ping" => rpc::result(&id, json!({})),
            "tools/list" => rpc::result(&id, json!({ "tools": self.list.tools.iter().map(|t| t.listed()).collect::<Vec<_>>() })),
            "tools/call" => match self.call(&params, lines, host) {
                Ok(result) => rpc::result(&id, result),
                Err(message) => rpc::error(&id, rpc::BAD_PARAMS, &message),
            },
            _ => rpc::error(&id, rpc::NO_METHOD, &format!("there is no method {method}")),
        })
    }

    fn initialize(&mut self, params: &Value) -> Value {
        self.elicits = params["capabilities"].get("elicitation").is_some();
        self.agent = params["clientInfo"]["name"].as_str().unwrap_or_default().to_string();
        let asked = params["protocolVersion"].as_str().unwrap_or("");
        let version = VERSIONS.iter().find(|v| **v == asked).unwrap_or(&VERSIONS[0]);
        json!({
            "protocolVersion": version,
            "capabilities": { "tools": { "listChanged": false } },
            "serverInfo": { "name": self.list.name, "version": self.options.version },
            "instructions": self.list.instructions,
        })
    }

    /// A tool call: a protocol error when it does not fit its tool, else
    /// the tool's answer, refused or run.
    fn call(&mut self, params: &Value, lines: &mut dyn Lines, host: &mut dyn Host) -> Result<Value, String> {
        let name = params["name"].as_str().ok_or("a tool call names its tool")?;
        let tool = self.list.tool(name).ok_or_else(|| format!("there is no tool {name}"))?.clone();
        let arguments = params.get("arguments").cloned().unwrap_or(Value::Null);
        if let Some(misfit) = inputs::misfit(&tool, &arguments) {
            return Err(misfit);
        }
        let arguments = if arguments.is_null() { json!({}) } else { arguments };
        let kind = tool.kind();
        let runs = match self.grants.verdict(&self.options, &tool.name, kind) {
            Verdict::Run => true,
            Verdict::Refuse(text) => return Ok(rpc::tool_result(&text, true)),
            Verdict::Ask => {
                let grant = Grant { agent: &self.agent, tool: &tool.name, description: &tool.description, kind, arguments: &arguments, changes: &tool.changes };
                let decision = host.grant(&grant);
                self.grants.decided(&tool.name, kind, decision)
            }
        };
        if !runs {
            return Ok(rpc::tool_result(&self.options.refused, true));
        }
        let mut asker = Asker { lines, elicits: self.elicits, asked: &mut self.asked, waiting: &mut self.waiting };
        Ok(match host.run(&tool, &arguments, &mut asker) {
            Ran::Printed(text) => rpc::tool_result(&text, false),
            Ran::Refused(text) => rpc::tool_result(&text, true),
        })
    }
}

/// Asks the agent over the same lines while a call runs: pings are
/// answered, other messages wait until the call is done.
struct Asker<'a> {
    lines: &'a mut dyn Lines,
    elicits: bool,
    asked: &'a mut u64,
    waiting: &'a mut VecDeque<String>,
}

impl Ask for Asker<'_> {
    fn ask(&mut self, message: &str, schema: &Value) -> Answer {
        if !self.elicits {
            return Answer::Unsupported;
        }
        *self.asked += 1;
        let id = Value::String(format!("ask-{}", self.asked));
        let params = json!({ "message": message, "requestedSchema": { "type": "object", "properties": { "answer": schema }, "required": ["answer"] } });
        self.lines.write(&rpc::request(&id, "elicitation/create", params).to_string());
        while let Some(line) = self.lines.read() {
            match rpc::parse(&line) {
                Ok(Message::Response { id: answered, result }) if answered == id => {
                    let result = result.unwrap_or(Value::Null);
                    return match result["action"].as_str() {
                        Some("accept") => Answer::Given(result["content"]["answer"].clone()),
                        Some("decline") => Answer::Declined,
                        _ => Answer::Cancelled,
                    };
                }
                Ok(Message::Request { id, method, .. }) if method == "ping" => self.lines.write(&rpc::result(&id, json!({})).to_string()),
                _ => self.waiting.push_back(line),
            }
        }
        Answer::Cancelled
    }
}
