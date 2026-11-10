//! Clarity's ACP client against a small agent of the tests that speaks the
//! protocol's JSON lines over a pipe in memory, so no vendor program or
//! login is needed: what the agent says reaches the panel, Clarity's tools
//! are handed over, a permission asks the person only for what runs or lies
//! outside the project, and files are read and written only inside it.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use clarity_acp::{Agent, Decision, Ended, Event, Failure, Host, SignIn, ToolServer, ToolStatus};
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, DuplexStream, ReadHalf, WriteHalf};
use tokio::sync::mpsc;

/// What the fake agent saw from the client.
#[derive(Default)]
struct Seen {
    initialize: Option<Value>,
    servers: Vec<Value>,
    cwd: Option<String>,
    signed_in: Option<String>,
    decision: Option<String>,
    read: Option<String>,
    outside_write: Option<bool>,
    inside_edit: Option<String>,
    outside_edit: Option<String>,
    terminal: Option<bool>,
}

/// The agent's end of the pipe: lines in, lines out, and its own requests to the client.
struct Fake {
    lines: tokio::io::Lines<BufReader<ReadHalf<DuplexStream>>>,
    out: WriteHalf<DuplexStream>,
    next: u64,
}

impl Fake {
    async fn send(&mut self, message: Value) {
        let mut line = message.to_string();
        line.push('\n');
        self.out.write_all(line.as_bytes()).await.unwrap();
    }

    async fn say(&mut self, session: &str, update: Value) {
        self.send(json!({ "jsonrpc": "2.0", "method": "session/update", "params": { "sessionId": session, "update": update } })).await;
    }

    /// Asks the client and waits for its answer (the client asks nothing meanwhile in these tests).
    async fn ask(&mut self, method: &str, params: Value) -> Value {
        self.next += 1;
        let id = 1000 + self.next;
        self.send(json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params })).await;
        while let Some(line) = self.lines.next_line().await.unwrap() {
            let message: Value = serde_json::from_str(&line).unwrap();
            if message["id"] == id && message.get("method").is_none() {
                return message;
            }
        }
        panic!("the client closed before it answered {method}");
    }
}

fn options() -> Value {
    json!([
        { "optionId": "always", "name": "Always", "kind": "allow_always" },
        { "optionId": "once", "name": "Once", "kind": "allow_once" },
        { "optionId": "no", "name": "No", "kind": "reject_once" },
    ])
}

fn chosen(answer: &Value) -> String {
    match answer["result"]["outcome"]["outcome"].as_str() {
        Some("selected") => answer["result"]["outcome"]["optionId"].as_str().unwrap_or_default().to_string(),
        _ => "cancelled".into(),
    }
}

/// An agent that offers one sign-in, opens a session once signed in and, on
/// a prompt, talks, calls a tool, asks permissions, reads and writes files
/// through the client and ends its turn.
fn fake_agent(read: ReadHalf<DuplexStream>, out: WriteHalf<DuplexStream>, seen: Arc<Mutex<Seen>>, root: PathBuf) {
    tokio::spawn(async move {
        let mut fake = Fake { lines: BufReader::new(read).lines(), out, next: 0 };
        while let Some(line) = fake.lines.next_line().await.unwrap_or(None) {
            let message: Value = serde_json::from_str(&line).unwrap();
            let id = message["id"].clone();
            let params = message["params"].clone();
            let reply = |result: Value| json!({ "jsonrpc": "2.0", "id": id, "result": result });
            match message["method"].as_str() {
                Some("initialize") => {
                    {
                        let mut s = seen.lock().unwrap();
                        s.initialize = Some(params.clone());
                        s.terminal = params["clientCapabilities"]["terminal"].as_bool();
                    }
                    let answer = reply(json!({
                        "protocolVersion": 1,
                        "agentInfo": { "name": "fake-agent", "version": "1.0" },
                        "authMethods": [{ "id": "chatgpt", "name": "Sign in with ChatGPT" }],
                    }));
                    fake.send(answer).await;
                }
                Some("authenticate") => {
                    seen.lock().unwrap().signed_in = params["methodId"].as_str().map(str::to_string);
                    fake.send(reply(json!({}))).await;
                }
                Some("session/new") => {
                    // As a vendor's program does: no session before it is signed in.
                    let signed_in = seen.lock().unwrap().signed_in.is_some();
                    if !signed_in {
                        fake.send(json!({ "jsonrpc": "2.0", "id": id, "error": { "code": -32000, "message": "Authentication required" } })).await;
                        continue;
                    }
                    {
                        let mut s = seen.lock().unwrap();
                        s.cwd = params["cwd"].as_str().map(str::to_string);
                        s.servers = params["mcpServers"].as_array().cloned().unwrap_or_default();
                    }
                    fake.send(reply(json!({ "sessionId": "s1" }))).await;
                }
                Some("session/prompt") => {
                    let session = params["sessionId"].as_str().unwrap_or_default().to_string();
                    fake.say(&session, json!({ "sessionUpdate": "agent_message_chunk", "content": { "type": "text", "text": "Making the window." } })).await;
                    fake.say(&session, json!({ "sessionUpdate": "tool_call", "toolCallId": "t1", "title": "write_file design/Main.cxaml", "kind": "other", "status": "pending" })).await;
                    fake.say(&session, json!({ "sessionUpdate": "tool_call_update", "toolCallId": "t1", "status": "completed" })).await;
                    // An edit inside the project, told of first, then asked for by its id alone.
                    let main = root.join("design/Main.cxaml").display().to_string();
                    fake.say(&session, json!({ "sessionUpdate": "tool_call", "toolCallId": "t3", "title": "Update file", "kind": "edit", "status": "pending", "locations": [{ "path": main }] })).await;
                    let inside = fake.ask("session/request_permission", json!({ "sessionId": session, "toolCall": { "toolCallId": "t3", "title": "Update file" }, "options": options() })).await;
                    // An edit outside the project.
                    let elsewhere = root.join("../elsewhere.md").display().to_string();
                    let outside_edit = fake
                        .ask("session/request_permission", json!({ "sessionId": session, "toolCall": { "toolCallId": "t4", "title": "Update outside", "kind": "edit", "locations": [{ "path": elsewhere }] }, "options": options() }))
                        .await;
                    // A command.
                    let run = fake.ask("session/request_permission", json!({ "sessionId": session, "toolCall": { "toolCallId": "t2", "title": "Run cargo build", "kind": "execute" }, "options": options() })).await;
                    let read = fake.ask("fs/read_text_file", json!({ "sessionId": session, "path": root.join("notes.md").display().to_string() })).await;
                    fake.ask("fs/write_text_file", json!({ "sessionId": session, "path": root.join("design/New.cxaml").display().to_string(), "content": "<Window />\n" })).await;
                    let escape = fake.ask("fs/write_text_file", json!({ "sessionId": session, "path": root.join("../escape.txt").display().to_string(), "content": "x" })).await;
                    let terminal = fake.ask("terminal/create", json!({ "sessionId": session, "command": "ls" })).await;
                    assert_eq!(terminal["error"]["code"], -32601, "no terminal: {terminal}");
                    {
                        let mut s = seen.lock().unwrap();
                        s.inside_edit = Some(chosen(&inside));
                        s.outside_edit = Some(chosen(&outside_edit));
                        s.decision = Some(chosen(&run));
                        s.read = read["result"]["content"].as_str().map(str::to_string);
                        s.outside_write = Some(escape.get("error").is_none());
                    }
                    fake.send(reply(json!({ "stopReason": "end_turn" }))).await;
                }
                _ => {}
            }
        }
    });
}

struct Run {
    agent: Agent,
    seen: Arc<Mutex<Seen>>,
    events: mpsc::UnboundedReceiver<Event>,
    root: PathBuf,
    /// What the person was asked, in order.
    asked: Arc<Mutex<Vec<String>>>,
}

async fn started(name: &str, decide: Decision) -> Run {
    let root = std::env::temp_dir().join(format!("clarity-acp-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("design")).unwrap();
    std::fs::write(root.join("notes.md"), "line one\nline two\n").unwrap();
    let (client_end, agent_end) = tokio::io::duplex(1 << 16);
    let (agent_read, agent_write) = tokio::io::split(agent_end);
    let (client_read, client_write) = tokio::io::split(client_end);
    let seen = Arc::new(Mutex::new(Seen::default()));
    fake_agent(agent_read, agent_write, seen.clone(), root.clone());
    let (events_tx, events) = mpsc::unbounded_channel();
    let (permissions, mut asked) = mpsc::unbounded_channel::<(clarity_acp::Permission, tokio::sync::oneshot::Sender<Decision>)>();
    let titles = Arc::new(Mutex::new(Vec::new()));
    let heard = titles.clone();
    tokio::spawn(async move {
        while let Some((permission, reply)) = asked.recv().await {
            heard.lock().unwrap().push(permission.title);
            let _ = reply.send(decide);
        }
    });
    let agent = Agent::start_on(client_read, client_write, &root, Host { events: events_tx, permissions }).await.expect("the agent starts");
    Run { agent, seen, events, root, asked: titles }
}

fn drained(events: &mut mpsc::UnboundedReceiver<Event>) -> Vec<Event> {
    let mut out = Vec::new();
    while let Ok(e) = events.try_recv() {
        out.push(e);
    }
    out
}

#[tokio::test]
async fn a_turn_reaches_the_panel_with_clarity_s_tools() {
    let mut run = started("turn", Decision::Allow).await;
    assert_eq!(run.agent.name(), "fake-agent");
    assert_eq!(run.agent.sign_ins(), [SignIn { id: "chatgpt".into(), name: "Sign in with ChatGPT".into(), description: None }]);
    run.agent.sign_in("chatgpt").await.unwrap();

    let tools = ToolServer { name: "clarity".into(), command: PathBuf::from("/opt/clarity/clarity"), args: vec!["mcp".into(), "--turn".into(), "/tmp/turn.jsonl".into()] };
    let session = run.agent.new_session(vec![tools]).await.unwrap();
    let ended = run.agent.prompt(&session, "Make a window").await.unwrap();
    assert_eq!(ended, Ended::Done);

    let seen = run.seen.lock().unwrap();
    let init = seen.initialize.as_ref().unwrap();
    assert_eq!(init["protocolVersion"], 1);
    assert_eq!(init["clientCapabilities"]["fs"], json!({ "readTextFile": true, "writeTextFile": true }), "the client reads and writes files");
    assert_eq!(seen.terminal, Some(false), "no terminal: Clarity runs nothing for the agent");
    assert_eq!(seen.signed_in.as_deref(), Some("chatgpt"));
    // A local server as ACP writes it: no type, name, command, args and env.
    assert_eq!(seen.servers, [json!({ "name": "clarity", "command": "/opt/clarity/clarity", "args": ["mcp", "--turn", "/tmp/turn.jsonl"], "env": [] })]);
    assert_eq!(seen.cwd.as_deref(), Some(run.root.display().to_string().as_str()));
    // Allowed once, never always: the next command asks again.
    assert_eq!(seen.decision.as_deref(), Some("once"));
    // An edit inside the project is the client's to allow (once): Undo takes it back.
    assert_eq!(seen.inside_edit.as_deref(), Some("once"));
    assert_eq!(seen.outside_edit.as_deref(), Some("once"), "allowed by the person here");
    assert_eq!(seen.read.as_deref(), Some("line one\nline two\n"));
    assert_eq!(seen.outside_write, Some(false), "a write outside the project is refused");
    drop(seen);
    assert_eq!(*run.asked.lock().unwrap(), ["Update outside", "Run cargo build"], "the person is asked only for what is outside the project or runs");
    assert_eq!(std::fs::read_to_string(run.root.join("design/New.cxaml")).unwrap(), "<Window />\n");
    assert!(!run.root.join("../escape.txt").exists());

    let events = drained(&mut run.events);
    assert!(events.contains(&Event::Text("Making the window.".into())), "{events:?}");
    assert!(events.contains(&Event::Tool { id: "t1".into(), title: "write_file design/Main.cxaml".into(), status: ToolStatus::Pending }), "{events:?}");
    assert!(events.contains(&Event::ToolUpdate { id: "t1".into(), title: None, status: Some(ToolStatus::Done) }), "{events:?}");
    let _ = std::fs::remove_dir_all(&run.root);
}

#[tokio::test]
async fn denied_is_rejected_once() {
    let run = started("deny", Decision::Deny).await;
    run.agent.sign_in("chatgpt").await.unwrap();
    let session = run.agent.new_session(Vec::new()).await.unwrap();
    run.agent.prompt(&session, "Build it").await.unwrap();
    assert_eq!(run.seen.lock().unwrap().decision.as_deref(), Some("no"));
    let _ = std::fs::remove_dir_all(&run.root);
}

/// A program that is not signed in says so in ACP's own words, and the
/// client tells it apart, so the panel offers Sign in only then.
#[tokio::test]
async fn not_signed_in_is_told_apart() {
    let run = started("signin", Decision::Deny).await;
    assert_eq!(run.agent.new_session(Vec::new()).await, Err(Failure::NotSignedIn));
    run.agent.sign_in("chatgpt").await.unwrap();
    assert!(run.agent.new_session(Vec::new()).await.is_ok());
    let _ = std::fs::remove_dir_all(&run.root);
}

#[test]
fn only_files_inside_the_project_are_reached() {
    let root = std::env::temp_dir().join(format!("clarity-acp-inside-{}", std::process::id()));
    std::fs::create_dir_all(root.join("design")).unwrap();
    std::fs::write(root.join("design/Main.cxaml"), "<Window />").unwrap();
    assert_eq!(clarity_acp::inside(&root, &root.join("design/Main.cxaml")).unwrap(), root.canonicalize().unwrap().join("design/Main.cxaml"), "a file that is there, as it is");
    assert!(clarity_acp::inside(&root, &root.join("new/folder/file.md")).is_ok(), "a new folder inside");
    assert!(clarity_acp::inside(&root, &root.join("../x")).is_err());
    assert!(clarity_acp::inside(&root, Path::new("relative.md")).is_err(), "ACP paths are absolute");
    assert!(clarity_acp::inside(&root, Path::new("/etc/passwd")).is_err());
    let _ = std::fs::remove_dir_all(&root);
}

/// The client depends on no ACP crate, so nothing switches on serde_json's
/// preserve_order for the workspace (it changed the key order of every JSON
/// Clarity writes, docs/ACP-PLAN.md).
#[test]
fn the_client_brings_no_acp_crate() {
    let manifest = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml")).unwrap();
    assert!(!manifest.contains("agent-client-protocol"), "{manifest}");
}
