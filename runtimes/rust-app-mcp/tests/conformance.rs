//! conformance/app-mcp.json through the core itself: every stack runs the
//! same exchanges through its binding of this crate.

use std::collections::VecDeque;

use clarity_app_mcp::{Answer, Ask, Decision, Grant, Host, Lines, Ran, Server, Tool};
use serde_json::{Value, json};

fn cases() -> Value {
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../conformance/app-mcp.json")).unwrap();
    serde_json::from_str(&text).unwrap()
}

/// The agent's side: the messages to send, and what the service wrote.
struct Script {
    sends: VecDeque<String>,
    written: Vec<Value>,
}

impl Lines for Script {
    fn read(&mut self) -> Option<String> {
        self.sends.pop_front()
    }
    fn write(&mut self, line: &str) {
        assert!(!line.contains('\n'), "one message a line: {line}");
        self.written.push(serde_json::from_str(line).unwrap_or_else(|e| panic!("{e}: {line}")));
    }
}

/// The app's side, as the file's `host` says, and the person's answers.
struct Scripted<'a> {
    host: &'a Value,
    grants: VecDeque<Decision>,
    asked: Vec<Value>,
}

impl Host for Scripted<'_> {
    fn run(&mut self, tool: &Tool, _arguments: &Value, ask: &mut dyn Ask) -> Ran {
        let said = &self.host[tool.name.as_str()];
        let text = |key: &str| said[key].as_str().unwrap().to_string();
        if let Some(refused) = said.get("refuse") {
            return Ran::Refused(refused.as_str().unwrap().to_string());
        }
        if let Some(question) = said.get("ask") {
            return match ask.ask(question["message"].as_str().unwrap(), &question["schema"]) {
                Answer::Given(Value::Bool(true)) => Ran::Printed(text("print")),
                Answer::Unsupported => Ran::Refused(text("unanswered")),
                _ => Ran::Refused(text("kept")),
            };
        }
        Ran::Printed(text("print"))
    }

    fn grant(&mut self, grant: &Grant) -> Decision {
        self.asked.push(json!({ "tool": grant.tool, "kind": grant.kind.word() }));
        self.grants.pop_front().expect("a grant asked that the case does not answer")
    }
}

#[test]
fn every_exchange_of_the_conformance_file() {
    let file = cases();
    for case in file["cases"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        // A case may say what the project allows while the app is closed (the tool list's `whileClosed`).
        let mut tools = file["tools"].clone();
        if let Some(closed) = case.get("whileClosed") {
            tools["whileClosed"] = closed.clone();
        }
        let mut server = Server::new(&tools.to_string(), &case["options"].to_string()).unwrap_or_else(|e| panic!("{name}: {e}"));
        let mut sends = VecDeque::new();
        let mut expected = Vec::new();
        for step in case["exchange"].as_array().unwrap() {
            match (step.get("send"), step.get("expect")) {
                (Some(Value::String(raw)), _) => sends.push_back(raw.clone()),
                (Some(message), _) => sends.push_back(message.to_string()),
                (_, Some(message)) => expected.push(message.clone()),
                _ => panic!("{name}: a step sends or expects"),
            }
        }
        let grants = case["grants"].as_array().unwrap().iter().map(|g| Decision::from_word(g.as_str().unwrap()).unwrap()).collect();
        let mut script = Script { sends, written: Vec::new() };
        let mut host = Scripted { host: &file["host"], grants, asked: Vec::new() };
        server.serve(&mut script, &mut host);
        assert_eq!(script.written, expected, "{name}");
        assert_eq!(Value::Array(host.asked), case["asked"], "{name}: the grants asked");
        assert!(host.grants.is_empty(), "{name}: answers left: {:?}", host.grants);
    }
}

#[test]
fn a_call_is_the_command_line_the_program_reads() {
    let file = cases();
    let list = clarity_app_mcp::ToolList::from_json(&file["tools"].to_string()).unwrap();
    for case in file["commandLines"].as_array().unwrap() {
        let tool = list.tool(case["tool"].as_str().unwrap()).unwrap();
        let line = clarity_app_mcp::cli::command_line(tool, &case["arguments"]);
        assert_eq!(json!(line), case["line"], "{}", case["what"]);
    }
}

/// The agent as a question's `reply` says: its answer, or no client that can ask.
struct Replying<'a>(&'a Value, Option<Value>);

impl Ask for Replying<'_> {
    fn ask(&mut self, _message: &str, schema: &Value) -> Answer {
        self.1 = Some(schema.clone());
        match self.0["action"].as_str() {
            None => Answer::Unsupported,
            Some("accept") => Answer::Given(self.0["content"]["answer"].clone()),
            Some("decline") => Answer::Declined,
            Some(_) => Answer::Cancelled,
        }
    }
}

#[test]
fn a_command_s_question_is_asked_of_the_agent() {
    for case in cases()["questions"].as_array().unwrap() {
        let what = case["what"].as_str().unwrap();
        let mut agent = Replying(&case["reply"], None);
        let answer = clarity_app_mcp::cli::ask(&mut agent, "?", &case["form"]);
        assert_eq!(agent.1.as_ref(), Some(&case["schema"]), "{what}: the schema");
        assert_eq!(answer.map(Value::String).unwrap_or(Value::Null), case["answer"], "{what}");
    }
}
