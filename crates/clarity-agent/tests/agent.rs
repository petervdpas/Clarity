//! The agent's loop: the model answers, asks for tools, gets their results
//! and goes on until it has answered. A scripted model and tools stand in
//! for a real model and a project.

use std::sync::atomic::{AtomicBool, Ordering};

use clarity_agent::{Agent, Block, Event, Message, Model, ModelReply, Request, Role, StopReason, ToolDef, ToolResult, Tools};
use serde_json::{Value, json};

/// A model that gives its replies in order, and keeps what it was asked.
struct Script {
    replies: Vec<ModelReply>,
    asked: Vec<Request>,
}

impl Model for Script {
    fn reply(&mut self, request: &Request, on_text: &mut dyn FnMut(&str)) -> Result<ModelReply, String> {
        self.asked.push(request.clone());
        let reply = self.replies.remove(0);
        for block in &reply.content {
            if let Block::Text { text } = block {
                on_text(text);
            }
        }
        Ok(reply)
    }
}

/// Tools that keep the files written to them.
#[derive(Default)]
struct Files {
    written: Vec<(String, String)>,
}

impl Tools for Files {
    fn definitions(&self) -> Vec<ToolDef> {
        vec![ToolDef {
            name: "write_file".into(),
            description: "Writes a file.".into(),
            input_schema: json!({"type": "object", "properties": {"path": {"type": "string"}, "text": {"type": "string"}}}),
        }]
    }

    fn call(&mut self, name: &str, input: &Value) -> ToolResult {
        match name {
            "write_file" => {
                self.written.push((input["path"].as_str().unwrap_or_default().into(), input["text"].as_str().unwrap_or_default().into()));
                ToolResult { text: "written; no problems".into(), is_error: false }
            }
            other => ToolResult { text: format!("there is no tool {other}"), is_error: true },
        }
    }
}

fn tool_use(id: &str, name: &str, input: Value) -> ModelReply {
    ModelReply { content: vec![Block::ToolUse { id: id.into(), name: name.into(), input }], stop: StopReason::ToolUse }
}

fn answer(text: &str) -> ModelReply {
    ModelReply { content: vec![Block::Text { text: text.into() }], stop: StopReason::EndTurn }
}

fn agent() -> Agent {
    Agent { system: "You work on Clarity projects.".into(), max_steps: 10 }
}

#[test]
fn the_model_uses_tools_until_it_has_answered() {
    let mut model = Script {
        replies: vec![tool_use("t1", "write_file", json!({"path": "todo.clarity", "text": "module todo"})), answer("Made the todo module.")],
        asked: vec![],
    };
    let mut tools = Files::default();
    let mut conversation = Vec::new();
    let mut events = Vec::new();
    agent().run(&mut model, &mut tools, &mut conversation, "Make a todo app", &AtomicBool::new(false), &mut |e| events.push(e)).unwrap();

    assert_eq!(tools.written, [("todo.clarity".to_string(), "module todo".to_string())]);
    // The model got the instructions, the tools, and then the tool's result.
    assert_eq!(model.asked[0].system, "You work on Clarity projects.");
    assert_eq!(model.asked[0].tools[0].name, "write_file");
    let last = model.asked[1].messages.last().unwrap();
    assert_eq!(last.role, Role::User);
    assert_eq!(last.content, [Block::ToolResult { tool_use_id: "t1".into(), content: "written; no problems".into(), is_error: false }]);

    // The conversation goes on from here next time.
    let roles: Vec<Role> = conversation.iter().map(|m: &Message| m.role).collect();
    assert_eq!(roles, [Role::User, Role::Assistant, Role::User, Role::Assistant]);
    assert_eq!(
        events,
        [
            Event::ToolStart { id: "t1".into(), name: "write_file".into(), input: json!({"path": "todo.clarity", "text": "module todo"}) },
            Event::ToolDone { id: "t1".into(), name: "write_file".into(), text: "written; no problems".into(), is_error: false },
            Event::Text { text: "Made the todo module.".into() },
            Event::Finished,
        ]
    );
}

#[test]
fn a_tool_that_fails_tells_the_model_and_the_loop_goes_on() {
    let mut model = Script { replies: vec![tool_use("t1", "format_disk", json!({})), answer("That tool is not there.")], asked: vec![] };
    let mut conversation = Vec::new();
    let mut events = Vec::new();
    agent().run(&mut model, &mut Files::default(), &mut conversation, "Go", &AtomicBool::new(false), &mut |e| events.push(e)).unwrap();
    let result = &model.asked[1].messages.last().unwrap().content[0];
    assert_eq!(result, &Block::ToolResult { tool_use_id: "t1".into(), content: "there is no tool format_disk".into(), is_error: true });
    assert_eq!(events.last(), Some(&Event::Finished));
}

#[test]
fn stopping_ends_the_turn_before_the_next_step() {
    let stop = AtomicBool::new(false);
    let mut model = Script { replies: vec![tool_use("t1", "write_file", json!({"path": "a", "text": ""})), answer("never")], asked: vec![] };
    let mut tools = Files::default();
    let mut conversation = Vec::new();
    let mut events = Vec::new();
    agent()
        .run(&mut model, &mut tools, &mut conversation, "Go", &stop, &mut |e| {
            if matches!(e, Event::ToolDone { .. }) {
                stop.store(true, Ordering::SeqCst);
            }
            events.push(e);
        })
        .unwrap();
    assert_eq!(model.asked.len(), 1, "no step after the stop");
    assert_eq!(events.last(), Some(&Event::Stopped));
    // The conversation stays whole: every tool call has its result.
    assert!(matches!(conversation.last().unwrap().content[0], Block::ToolResult { .. }));
}

#[test]
fn a_turn_has_a_limit_of_steps() {
    let replies = (0..5).map(|i| tool_use(&format!("t{i}"), "write_file", json!({"path": "a", "text": ""}))).collect();
    let mut model = Script { replies, asked: vec![] };
    let mut events = Vec::new();
    let limited = Agent { max_steps: 3, ..agent() };
    limited.run(&mut model, &mut Files::default(), &mut Vec::new(), "Go", &AtomicBool::new(false), &mut |e| events.push(e)).unwrap();
    assert_eq!(model.asked.len(), 3);
    assert_eq!(events.last(), Some(&Event::Failed { message: "stopped after 3 steps; ask again to go on".into() }));
}
