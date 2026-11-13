//! `<program> mcp`: a command-line program serves its commands for agents
//! through Clarity.AppMcp (docs/ledger/app-mcp.md). A call runs the command
//! as if typed; what it prints is the answer; its questions go to the agent;
//! while the app is closed the person's settings allow, else it is refused.
#![cfg(feature = "mcp")]

use std::collections::VecDeque;

use clarity_app_mcp::Lines;
use clarity_runtime::agents::{self, Agents};
use clarity_runtime::cli::{Kind, Program, Session};
use clarity_runtime::ui::Text;
use serde_json::{Map, Value, json};

fn file() -> Value {
    serde_json::from_str(include_str!("../../../conformance/app-mcp.json")).unwrap()
}

/// The commands of the conformance file's tool list, as the program reads them.
fn program() -> Program {
    serde_json::from_value(json!({
        "name": "shop",
        "commands": [
            { "line": ["customer", "add"], "arguments": [{ "name": "name", "kind": "text" }, { "name": "kind", "kind": "enum", "values": ["Person", "Business"] }],
              "options": [{ "name": "limit", "kind": "money", "default": "1000", "min": "0", "max": "5000" }, { "name": "vip", "kind": "bool" }] },
            { "line": ["customer", "count"], "options": [{ "name": "since", "kind": "integer", "min": "2000" }] },
            { "line": ["customer", "remove"], "arguments": [{ "name": "name", "kind": "text" }] },
            { "line": ["report", "send"] }
        ]
    }))
    .unwrap()
}

fn run(cli: &mut Session<'_>, command: usize, values: Vec<Option<String>>) -> clarity_runtime::Result<i32> {
    let value = |i: usize| values[i].clone().unwrap_or_default();
    match command {
        0 => cli.print(Text::plain(format!("Added {} ({}), limit {}, vip {}", value(0), value(1), value(2), value(3)))),
        1 => cli.print(Text::plain("2 customers")),
        2 => {
            let copies: i64 = cli.ask(Text::plain("How many copies?"), Kind::Integer, &[], None);
            if !cli.confirm(Text::plain(format!("Remove {}?", value(0))), None, None) {
                return Err(clarity_runtime::Error::Refused(Text::plain("Kept")));
            }
            cli.print(Text::plain(format!("Removed {} after {copies} copies", value(0))));
        }
        _ => return Ok(3),
    }
    Ok(0)
}

struct Script {
    sends: VecDeque<String>,
    written: Vec<Value>,
}

impl Lines for Script {
    fn read(&mut self) -> Option<String> {
        self.sends.pop_front()
    }
    fn write(&mut self, line: &str) {
        self.written.push(serde_json::from_str(line).unwrap());
    }
}

fn served(kept: Value, sends: Vec<Value>) -> Vec<Value> {
    let tools = file()["tools"].to_string();
    let agents = Agents { tools: &tools, app: "Shop", program: "shop", version: "1.0" };
    let kept: Map<String, Value> = serde_json::from_value(kept).unwrap();
    let mut script = Script { sends: sends.iter().map(Value::to_string).collect(), written: Vec::new() };
    agents::serve(&program(), &agents, &kept, &mut script, &mut run).unwrap();
    script.written
}

fn call(id: i64, tool: &str, arguments: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "method": "tools/call", "params": { "name": tool, "arguments": arguments } })
}

fn said(reply: &Value) -> (&str, bool) {
    (reply["result"]["content"][0]["text"].as_str().unwrap_or_else(|| panic!("{reply}")), reply["result"]["isError"].as_bool().unwrap())
}

const ALLOWED: &str = r#"{"agentReadsWhileClosed": true, "agentChangesWhileClosed": true}"#;

#[test]
fn a_call_runs_the_command_as_typed_and_answers_with_what_it_printed() {
    let written = served(serde_json::from_str(ALLOWED).unwrap(), vec![call(1, "customer-add", json!({ "name": "Ada", "kind": "Person" })), call(2, "customer-add", json!({ "name": "Bob", "kind": "Business", "limit": 200.5, "vip": true }))]);
    assert_eq!(said(&written[0]), ("Added Ada (Person), limit 1000, vip false", false), "the option's default, as at the terminal");
    assert_eq!(said(&written[1]), ("Added Bob (Business), limit 200.5, vip true", false));
}

#[test]
fn while_the_app_is_closed_the_person_s_settings_decide() {
    let written = served(json!({}), vec![call(1, "customer-add", json!({ "name": "Ada", "kind": "Person" })), call(2, "customer-count", json!({}))]);
    assert_eq!(said(&written[0]), ("Shop is not open; open it to allow this.", true));
    assert_eq!(said(&written[1]), ("Shop is not open; open it to allow this.", true));
    let written = served(json!({ "agentReadsWhileClosed": true }), vec![call(1, "customer-add", json!({ "name": "Ada", "kind": "Person" })), call(2, "customer-count", json!({}))]);
    assert_eq!(said(&written[0]).1, true, "reads only");
    assert_eq!(said(&written[1]), ("2 customers", false));
}

#[test]
fn a_command_asks_the_agent_and_stops_without_an_answer() {
    let initialize = json!({ "jsonrpc": "2.0", "id": 0, "method": "initialize", "params": { "capabilities": { "elicitation": {} } } });
    let answer = |id: &str, content: Value| json!({ "jsonrpc": "2.0", "id": id, "result": { "action": "accept", "content": content } });
    let written = served(serde_json::from_str(ALLOWED).unwrap(), vec![initialize, call(1, "customer-remove", json!({ "name": "Ada" })), answer("ask-1", json!({ "answer": 2 })), answer("ask-2", json!({ "answer": true }))]);
    assert_eq!(written[1]["params"]["requestedSchema"]["properties"]["answer"], json!({ "type": "integer" }));
    assert_eq!(written[2]["params"]["requestedSchema"]["properties"]["answer"], json!({ "type": "boolean" }));
    assert_eq!(said(&written[3]), ("Removed Ada after 2 copies", false));
    // A client that cannot ask: the value without a default has no answer, and the command stops.
    let written = served(serde_json::from_str(ALLOWED).unwrap(), vec![call(1, "customer-remove", json!({ "name": "Ada" }))]);
    assert_eq!(said(&written[0]), ("There is nobody to answer: How many copies?", true));
}

#[test]
fn a_refusal_and_an_exit_code_are_tool_errors() {
    let initialize = json!({ "jsonrpc": "2.0", "id": 0, "method": "initialize", "params": { "capabilities": { "elicitation": {} } } });
    let answer = |id: &str, content: Value| json!({ "jsonrpc": "2.0", "id": id, "result": { "action": "accept", "content": content } });
    let written = served(serde_json::from_str(ALLOWED).unwrap(), vec![initialize, call(1, "customer-remove", json!({ "name": "Ada" })), answer("ask-1", json!({ "answer": 1 })), answer("ask-2", json!({ "answer": false })), call(2, "report-send", json!({}))]);
    assert_eq!(said(&written[3]), ("Kept", true));
    assert_eq!(said(&written[4]), ("exit 3", true));
}

#[test]
fn a_value_past_its_limit_is_refused_by_the_tool_s_inputs() {
    let written = served(serde_json::from_str(ALLOWED).unwrap(), vec![call(1, "customer-add", json!({ "name": "Ada", "kind": "Person", "limit": 9000 }))]);
    assert_eq!(written[0]["error"]["message"], json!("the argument limit of customer-add is at most 5000"));
}

#[test]
fn the_words_of_refusals_are_the_shared_ones() {
    let words = file()["words"].clone();
    assert_eq!(words.as_object().unwrap().len(), agents::WORDS.len());
    for (key, english) in agents::WORDS {
        assert_eq!(words[key], json!(english), "{key}");
    }
}
