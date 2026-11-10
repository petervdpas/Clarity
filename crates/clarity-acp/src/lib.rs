//! Clarity as an ACP client (docs/ACP-PLAN.md): the chat panel talks to the
//! person's own agent program (Codex, Copilot), signed in with that
//! program's own login, so no API key is typed into Clarity.
//!
//! The Agent Client Protocol is JSON-RPC 2.0, one message per line, over the
//! program's stdin and stdout. Clarity speaks it itself with serde_json and
//! no ACP crate: the user said "go non standard use json_something then!"
//! after that crate's serde_json features changed the key order of every
//! JSON Clarity writes.
//!
//! [`Agent::start`] starts the program and initializes it; the panel then
//! signs in when the program asks, opens a session with Clarity's tools
//! (`clarity mcp`) and sends prompts. What the agent says and does arrives as
//! [`Event`]s; a permission it asks for goes to the person, and is allowed
//! once at most, never always. The client offers no terminal, and reads and
//! writes files only inside the project's folder.

pub mod vendors;

use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::sync::{mpsc, oneshot};

/// The protocol version Clarity speaks.
const PROTOCOL_VERSION: u64 = 1;

/// JSON-RPC error codes Clarity reads or writes.
const AUTH_REQUIRED: i64 = -32000;
const INVALID_PARAMS: i64 = -32602;
const METHOD_NOT_FOUND: i64 = -32601;

/// The agent program to start, as the person's settings name it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    pub command: PathBuf,
    pub args: Vec<String>,
    /// Set for the program only, like the mode it starts in (vendors).
    pub env: Vec<(String, String)>,
}

/// Something the agent said or did, for the panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// A piece of what it says.
    Text(String),
    /// A piece of what it thinks, shown folded.
    Thought(String),
    /// A tool it starts.
    Tool { id: String, title: String, status: ToolStatus },
    /// A change to a tool it started.
    ToolUpdate { id: String, title: Option<String>, status: Option<ToolStatus> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolStatus {
    Pending,
    Running,
    Done,
    Failed,
}

/// What the agent asks permission for, as the person reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Permission {
    pub title: String,
}

/// The person's answer to a permission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Allow,
    Deny,
}

/// Where the client sends what the panel shows and asks.
pub struct Host {
    pub events: mpsc::UnboundedSender<Event>,
    /// Each permission, with where its answer goes; no answer is Deny.
    pub permissions: mpsc::UnboundedSender<(Permission, oneshot::Sender<Decision>)>,
}

/// A way the program signs in, as it offers it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignIn {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
}

/// An MCP server the agent gets for a session: Clarity's tools.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolServer {
    pub name: String,
    pub command: PathBuf,
    pub args: Vec<String>,
}

/// How a turn ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ended {
    Done,
    /// It ran out of room or steps.
    TooLong,
    Refused,
    Cancelled,
}

/// Why a request to the agent failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Failure {
    /// The program is not signed in (ACP's `auth_required`): the panel offers Sign in.
    NotSignedIn,
    /// Anything else, as the person reads it.
    Other(String),
}

impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Failure::NotSignedIn => f.write_str("the agent program is not signed in; sign in first"),
            Failure::Other(why) => f.write_str(why),
        }
    }
}

impl From<Failure> for String {
    fn from(failure: Failure) -> String {
        failure.to_string()
    }
}

/// A JSON-RPC error as it came.
#[derive(Debug, Clone)]
struct RpcError {
    code: i64,
    message: String,
}

/// A failed request as a Failure: not signed in, or `what` failed.
fn failure(err: RpcError, what: &str) -> Failure {
    if err.code == AUTH_REQUIRED { Failure::NotSignedIn } else { Failure::Other(format!("{what}: {} ({})", err.message, err.code)) }
}

type Pending = Mutex<HashMap<u64, oneshot::Sender<Result<Value, RpcError>>>>;

/// The connection to the program: writing lines, and the answers awaited.
struct Connection {
    writer: tokio::sync::Mutex<Box<dyn AsyncWrite + Send + Unpin>>,
    next: AtomicU64,
    pending: Pending,
    /// The program, stopped when the last handle to it goes.
    _child: Option<tokio::process::Child>,
}

impl Connection {
    async fn send(&self, message: Value) -> Result<(), String> {
        let mut line = message.to_string();
        line.push('\n');
        let mut writer = self.writer.lock().await;
        writer.write_all(line.as_bytes()).await.map_err(|err| format!("cannot write to the agent: {err}"))?;
        writer.flush().await.map_err(|err| format!("cannot write to the agent: {err}"))
    }

    async fn request(&self, method: &str, params: Value) -> Result<Value, RpcError> {
        let id = self.next.fetch_add(1, Ordering::SeqCst) + 1;
        let (reply, answer) = oneshot::channel();
        if let Ok(mut pending) = self.pending.lock() {
            pending.insert(id, reply);
        }
        let stopped = |message: String| RpcError { code: 0, message };
        self.send(json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params })).await.map_err(stopped)?;
        answer.await.map_err(|_| stopped("the agent has stopped".into()))?
    }

    async fn notify(&self, method: &str, params: Value) -> Result<(), String> {
        self.send(json!({ "jsonrpc": "2.0", "method": method, "params": params })).await
    }

    async fn answer(&self, id: Value, result: Result<Value, (i64, String)>) {
        let message = match result {
            Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
            Err((code, message)) => json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } }),
        };
        if let Err(err) = self.send(message).await {
            eprintln!("clarity-acp: {err}");
        }
    }

    /// The program closed its output: every answer still awaited fails.
    fn closed(&self) {
        if let Ok(mut pending) = self.pending.lock() {
            pending.clear();
        }
    }
}

/// A running agent program, initialized. A clone talks to the same program.
#[derive(Clone)]
pub struct Agent {
    name: String,
    sign_ins: Vec<SignIn>,
    /// The project folder its sessions work in.
    root: PathBuf,
    connection: Arc<Connection>,
}

impl Agent {
    /// Starts `program`, working in the project folder `root`.
    pub async fn start(program: &Program, root: &Path, host: Host) -> Result<Agent, String> {
        let mut command = tokio::process::Command::new(&program.command);
        command
            .args(&program.args)
            .envs(program.env.iter().cloned())
            .current_dir(root)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true);
        let mut child = command.spawn().map_err(|err| format!("cannot start {}: {err}", program.command.display()))?;
        let stdin = child.stdin.take().ok_or("the agent has no input")?;
        let stdout = child.stdout.take().ok_or("the agent has no output")?;
        Self::connect(stdout, stdin, Some(child), root, host).await
    }

    /// Like `start`, over any pair of streams (the tests use a pipe in memory).
    pub async fn start_on(reader: impl AsyncRead + Send + Unpin + 'static, writer: impl AsyncWrite + Send + Unpin + 'static, root: &Path, host: Host) -> Result<Agent, String> {
        Self::connect(reader, writer, None, root, host).await
    }

    async fn connect(reader: impl AsyncRead + Send + Unpin + 'static, writer: impl AsyncWrite + Send + Unpin + 'static, child: Option<tokio::process::Child>, root: &Path, host: Host) -> Result<Agent, String> {
        let connection = Arc::new(Connection { writer: tokio::sync::Mutex::new(Box::new(writer)), next: AtomicU64::new(0), pending: Mutex::default(), _child: child });
        let client = Client { root: root.to_path_buf(), host, known: Mutex::default() };
        tokio::spawn(read(BufReader::new(reader), Arc::downgrade(&connection), Arc::new(client)));
        // Files read and written, never a terminal: Clarity runs nothing for the agent.
        let init = connection
            .request(
                "initialize",
                json!({
                    "protocolVersion": PROTOCOL_VERSION,
                    "clientCapabilities": { "fs": { "readTextFile": true, "writeTextFile": true }, "terminal": false },
                    "clientInfo": { "name": "clarity", "version": env!("CARGO_PKG_VERSION") },
                }),
            )
            .await
            .map_err(|err| format!("the agent did not start: {}", err.message))?;
        let name = init["agentInfo"]["name"].as_str().unwrap_or_default().to_string();
        let sign_ins = init["authMethods"].as_array().map(|ways| ways.iter().filter_map(sign_in_of).collect()).unwrap_or_default();
        Ok(Agent { name, sign_ins, root: root.to_path_buf(), connection })
    }

    /// The program's name, as it gives it.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The ways the program can sign in; empty when it needs none.
    pub fn sign_ins(&self) -> &[SignIn] {
        &self.sign_ins
    }

    /// Signs in the way `id` names; the program does it (a browser opens for most).
    pub async fn sign_in(&self, id: &str) -> Result<(), Failure> {
        self.connection.request("authenticate", json!({ "methodId": id })).await.map(|_| ()).map_err(|err| failure(err, "signing in failed"))
    }

    /// Opens a session in the project folder with these tools; its id.
    pub async fn new_session(&self, tools: Vec<ToolServer>) -> Result<String, Failure> {
        let servers: Vec<Value> = tools
            .into_iter()
            .map(|s| json!({ "name": s.name, "command": s.command.display().to_string(), "args": s.args, "env": [] }))
            .collect();
        let result = self.connection.request("session/new", json!({ "cwd": self.root.display().to_string(), "mcpServers": servers })).await.map_err(|err| failure(err, "the session did not open"))?;
        result["sessionId"].as_str().map(str::to_string).ok_or_else(|| Failure::Other("the agent gave no session".into()))
    }

    /// Sends what the person typed; how the turn ended.
    pub async fn prompt(&self, session: &str, text: &str) -> Result<Ended, Failure> {
        let result = self
            .connection
            .request("session/prompt", json!({ "sessionId": session, "prompt": [{ "type": "text", "text": text }] }))
            .await
            .map_err(|err| failure(err, "the turn failed"))?;
        Ok(match result["stopReason"].as_str() {
            Some("max_tokens" | "max_turn_requests") => Ended::TooLong,
            Some("refusal") => Ended::Refused,
            Some("cancelled") => Ended::Cancelled,
            _ => Ended::Done,
        })
    }

    /// Asks the agent to stop the turn; the turn then ends as Cancelled.
    pub fn cancel(&self, session: &str) {
        let connection = self.connection.clone();
        let session = session.to_string();
        tokio::spawn(async move {
            let _ = connection.notify("session/cancel", json!({ "sessionId": session })).await;
        });
    }
}

/// What the client keeps and where it reports.
struct Client {
    root: PathBuf,
    host: Host,
    /// What each tool is and touches, as the agent told of it, for its permission later.
    known: Mutex<HashMap<String, Known>>,
}

/// Reads the program's lines: answers to Clarity's requests, the agent's
/// own requests (permissions, files) and its notifications (what it does).
async fn read(mut reader: BufReader<impl AsyncRead + Unpin>, connection: std::sync::Weak<Connection>, client: Arc<Client>) {
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }
        let Ok(message) = serde_json::from_str::<Value>(line.trim()) else { continue };
        let Some(connection) = connection.upgrade() else { break };
        let method = message["method"].as_str().map(str::to_string);
        match (method, message.get("id").cloned()) {
            // An answer to one of Clarity's requests.
            (None, Some(id)) => {
                let waiting = id.as_u64().and_then(|id| connection.pending.lock().ok().and_then(|mut p| p.remove(&id)));
                if let Some(reply) = waiting {
                    let result = match message.get("error") {
                        Some(error) => Err(RpcError { code: error["code"].as_i64().unwrap_or(0), message: error["message"].as_str().unwrap_or_default().to_string() }),
                        None => Ok(message.get("result").cloned().unwrap_or(Value::Null)),
                    };
                    let _ = reply.send(result);
                }
            }
            // A request of the agent's: answered on its own task, so a permission's wait holds up nothing.
            (Some(method), Some(id)) => {
                let client = client.clone();
                let params = message["params"].clone();
                tokio::spawn(async move {
                    let result = client.handle(&method, &params).await;
                    connection.answer(id, result).await;
                });
            }
            // A notification: what the agent says and does.
            (Some(method), None) => {
                if method == "session/update" {
                    client.update(&message["params"]["update"]);
                }
            }
            (None, None) => {}
        }
    }
    if let Some(connection) = connection.upgrade() {
        connection.closed();
    }
}

impl Client {
    async fn handle(&self, method: &str, params: &Value) -> Result<Value, (i64, String)> {
        match method {
            "session/request_permission" => Ok(json!({ "outcome": self.permission(params).await })),
            "fs/read_text_file" => {
                let path = PathBuf::from(params["path"].as_str().unwrap_or_default());
                let text = inside(&self.root, &path)
                    .and_then(|real| std::fs::read_to_string(&real).map_err(|err| format!("cannot read {}: {err}", path.display())))
                    .map_err(|why| (INVALID_PARAMS, why))?;
                let number = |key: &str| params[key].as_u64().map(|n| n as u32);
                Ok(json!({ "content": lines(&text, number("line"), number("limit")) }))
            }
            "fs/write_text_file" => {
                let path = PathBuf::from(params["path"].as_str().unwrap_or_default());
                let content = params["content"].as_str().unwrap_or_default();
                inside(&self.root, &path)
                    .and_then(|real| {
                        if let Some(folder) = real.parent() {
                            std::fs::create_dir_all(folder).map_err(|err| format!("cannot make {}: {err}", folder.display()))?;
                        }
                        std::fs::write(&real, content).map_err(|err| format!("cannot write {}: {err}", path.display()))
                    })
                    .map_err(|why| (INVALID_PARAMS, why))?;
                Ok(Value::Null)
            }
            // A terminal and the rest are not offered: Clarity runs nothing for the agent.
            other => Err((METHOD_NOT_FOUND, format!("Clarity does not offer {other}"))),
        }
    }

    /// The outcome of a permission: reading and editing inside the project
    /// are the client's to allow (one Undo takes the turn back); anything
    /// else asks the person. Allowed is once, never always; denied is
    /// rejected once, or cancelled when no rejection is offered.
    async fn permission(&self, params: &Value) -> Value {
        let call = &params["toolCall"];
        let decision = if self.allowed_by_itself(call) {
            Decision::Allow
        } else {
            let title = call["title"].as_str().unwrap_or_default().to_string();
            let (answer, decided) = oneshot::channel();
            match self.host.permissions.send((Permission { title }, answer)) {
                Ok(()) => decided.await.unwrap_or(Decision::Deny),
                Err(_) => Decision::Deny,
            }
        };
        let wanted = match decision {
            Decision::Allow => "allow_once",
            Decision::Deny => "reject_once",
        };
        let options = params["options"].as_array().cloned().unwrap_or_default();
        match options.iter().find(|o| o["kind"] == wanted).and_then(|o| o["optionId"].as_str()) {
            Some(id) => json!({ "outcome": "selected", "optionId": id }),
            None => json!({ "outcome": "cancelled" }),
        }
    }

    /// Whether a permission is the client's to give without asking:
    /// reading, searching or changing files, every one of them named and
    /// inside the project. Running anything, fetching, or a file outside (or
    /// none named) goes to the person.
    fn allowed_by_itself(&self, call: &Value) -> bool {
        let id = call["toolCallId"].as_str().unwrap_or_default();
        let mut tool = self.known.lock().ok().and_then(|k| k.get(id).cloned()).unwrap_or_default();
        tool.merge(call);
        let files_only = matches!(tool.kind.as_deref(), Some("read" | "edit" | "delete" | "move" | "search"));
        files_only && !tool.locations.is_empty() && tool.locations.iter().all(|path| inside(&self.root, path).is_ok())
    }

    /// A `session/update`: kept for permissions, and told to the panel.
    fn update(&self, update: &Value) {
        let id = update["toolCallId"].as_str().unwrap_or_default().to_string();
        let event = match update["sessionUpdate"].as_str() {
            Some("agent_message_chunk") => update["content"]["text"].as_str().map(|t| Event::Text(t.to_string())),
            Some("agent_thought_chunk") => update["content"]["text"].as_str().map(|t| Event::Thought(t.to_string())),
            Some("tool_call") => {
                if let Ok(mut known) = self.known.lock() {
                    known.insert(id.clone(), Known::default().merged(update));
                }
                Some(Event::Tool { id, title: update["title"].as_str().unwrap_or_default().to_string(), status: status_of(&update["status"]).unwrap_or(ToolStatus::Pending) })
            }
            Some("tool_call_update") => {
                if let Ok(mut known) = self.known.lock() {
                    known.entry(id.clone()).or_default().merge(update);
                }
                Some(Event::ToolUpdate { id, title: update["title"].as_str().map(str::to_string), status: status_of(&update["status"]) })
            }
            _ => None,
        };
        if let Some(event) = event {
            let _ = self.host.events.send(event);
        }
    }
}

/// What a tool is and the files it touches, as the agent told of it.
#[derive(Debug, Clone, Default)]
struct Known {
    kind: Option<String>,
    locations: Vec<PathBuf>,
}

impl Known {
    /// Takes what a tool's announcement, update or permission says of it.
    fn merge(&mut self, call: &Value) {
        if let Some(kind) = call["kind"].as_str() {
            self.kind = Some(kind.to_string());
        }
        if let Some(locations) = call["locations"].as_array() {
            self.locations = locations.iter().filter_map(|l| l["path"].as_str().map(PathBuf::from)).collect();
        }
    }

    fn merged(mut self, call: &Value) -> Known {
        self.merge(call);
        self
    }
}

fn status_of(status: &Value) -> Option<ToolStatus> {
    match status.as_str()? {
        "pending" => Some(ToolStatus::Pending),
        "in_progress" => Some(ToolStatus::Running),
        "completed" => Some(ToolStatus::Done),
        "failed" => Some(ToolStatus::Failed),
        _ => Some(ToolStatus::Running),
    }
}

/// A sign-in way as the program offers it: by the agent itself (no `type`) or in a terminal.
fn sign_in_of(way: &Value) -> Option<SignIn> {
    Some(SignIn {
        id: way["id"].as_str()?.to_string(),
        name: way["name"].as_str().unwrap_or_default().to_string(),
        description: way["description"].as_str().map(str::to_string),
    })
}

/// `path` when it lies inside the project folder `root`: absolute, without
/// `..`, and not leading out through a link. A file that is not there yet
/// may be in a folder that is not there yet.
pub fn inside(root: &Path, path: &Path) -> Result<PathBuf, String> {
    let refused = || format!("{} is outside the project; the agent reaches only its files", path.display());
    if !path.is_absolute() || path.components().any(|c| c == Component::ParentDir) {
        return Err(refused());
    }
    let root = root.canonicalize().map_err(|err| format!("cannot reach the project folder: {err}"))?;
    // The nearest part of the path that is there, followed through links.
    let mut there = path;
    while !there.exists() {
        there = there.parent().ok_or_else(refused)?;
    }
    let real = there.canonicalize().map_err(|_| refused())?;
    if !real.starts_with(&root) {
        return Err(refused());
    }
    // The part not there yet, under the real one; nothing to add for a file that is there.
    let rest = path.strip_prefix(there).unwrap_or(Path::new(""));
    Ok(if rest.as_os_str().is_empty() { real } else { real.join(rest) })
}

/// From line `line` (counted from 1), `limit` lines at most.
fn lines(text: &str, line: Option<u32>, limit: Option<u32>) -> String {
    if line.is_none() && limit.is_none() {
        return text.to_string();
    }
    let skip = line.map_or(0, |l| l.saturating_sub(1) as usize);
    let take = limit.map_or(usize::MAX, |l| l as usize);
    text.split_inclusive('\n').skip(skip).take(take).collect()
}
