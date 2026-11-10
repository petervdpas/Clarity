//! Clarity as an ACP client (docs/ACP-PLAN.md): the chat panel talks to the
//! person's own agent program (Codex, Copilot), signed in with that
//! program's own login, so no API key is typed into Clarity.
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
use std::sync::{Arc, Mutex};

use agent_client_protocol as acp;
use acp::schema::ProtocolVersion;
use acp::schema::v1::{
    AuthMethod, AuthenticateRequest, CancelNotification, ClientCapabilities, ContentBlock, FileSystemCapabilities, InitializeRequest,
    McpServer, McpServerStdio, NewSessionRequest, PermissionOptionKind, PromptRequest, ReadTextFileRequest, ReadTextFileResponse,
    RequestPermissionOutcome, RequestPermissionRequest, RequestPermissionResponse, SelectedPermissionOutcome, SessionId,
    SessionNotification, SessionUpdate, StopReason, TextContent, ToolCallStatus, ToolKind, WriteTextFileRequest, WriteTextFileResponse,
};
use tokio::sync::{mpsc, oneshot};

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

/// A failed request as a Failure: not signed in, or `what` failed.
fn failure(err: acp::Error, what: &str) -> Failure {
    if err.code == acp::ErrorCode::AuthRequired { Failure::NotSignedIn } else { Failure::Other(format!("{what}: {err}")) }
}

enum Command {
    SignIn(String, oneshot::Sender<Result<(), Failure>>),
    NewSession(Vec<ToolServer>, oneshot::Sender<Result<String, Failure>>),
    Prompt(String, String, oneshot::Sender<Result<Ended, Failure>>),
    Cancel(String),
}

/// A running agent program, initialized. A clone talks to the same program.
#[derive(Clone)]
pub struct Agent {
    name: String,
    sign_ins: Vec<SignIn>,
    commands: mpsc::UnboundedSender<Command>,
}

impl Agent {
    /// Starts `program`, working in the project folder `root`.
    pub async fn start(program: &Program, root: &Path, host: Host) -> Result<Agent, String> {
        let config = acp::AcpAgentConfig::new(&program.command).args(program.args.iter().cloned()).envs(program.env.iter().cloned());
        Self::start_on(acp::AcpAgent::new(config), root, host).await
    }

    /// Like `start`, over any transport (the tests use a channel in memory).
    pub async fn start_on(transport: impl acp::ConnectTo<acp::Client> + Send + 'static, root: &Path, host: Host) -> Result<Agent, String> {
        let root = root.to_path_buf();
        let (commands, mut queue) = mpsc::unbounded_channel::<Command>();
        let (ready, started) = oneshot::channel::<Result<(String, Vec<SignIn>), String>>();
        let Host { events, permissions } = host;
        let (read_root, write_root, ask_root, session_root) = (root.clone(), root.clone(), root.clone(), root);
        // What each tool is and touches, as the agent told of it, for its permission later.
        let known: Arc<Mutex<HashMap<String, Known>>> = Arc::default();
        let told = known.clone();
        tokio::spawn(async move {
            let ended = acp::Client
                .builder()
                .name("clarity")
                .on_receive_notification(
                    async move |note: SessionNotification, _cx| {
                        remember(&told, &note.update);
                        if let Some(event) = event_of(note.update) {
                            let _ = events.send(event);
                        }
                        Ok(())
                    },
                    acp::on_receive_notification!(),
                )
                .on_receive_request(
                    async move |asked: RequestPermissionRequest, responder, cx: acp::ConnectionTo<acp::Agent>| {
                        let permissions = permissions.clone();
                        // Reading and editing inside the project are the client's to allow: one Undo takes the turn back.
                        if allowed_by_itself(&ask_root, &known, &asked) {
                            return responder.respond(RequestPermissionResponse::new(outcome(&asked, Decision::Allow)));
                        }
                        cx.spawn(async move {
                            let title = asked.tool_call.fields.title.clone().unwrap_or_default();
                            let (answer, decided) = oneshot::channel();
                            let decision = match permissions.send((Permission { title }, answer)) {
                                Ok(()) => decided.await.unwrap_or(Decision::Deny),
                                Err(_) => Decision::Deny,
                            };
                            responder.respond(RequestPermissionResponse::new(outcome(&asked, decision)))
                        })
                    },
                    acp::on_receive_request!(),
                )
                .on_receive_request(
                    async move |read: ReadTextFileRequest, responder, _cx| match inside(&read_root, &read.path).and_then(|path| {
                        std::fs::read_to_string(&path).map_err(|err| format!("cannot read {}: {err}", read.path.display()))
                    }) {
                        Ok(text) => responder.respond(ReadTextFileResponse::new(lines(&text, read.line, read.limit))),
                        Err(why) => responder.respond_with_error(acp::Error::invalid_params().data(why)),
                    },
                    acp::on_receive_request!(),
                )
                .on_receive_request(
                    async move |write: WriteTextFileRequest, responder, _cx| match inside(&write_root, &write.path).and_then(|path| {
                        if let Some(folder) = path.parent() {
                            std::fs::create_dir_all(folder).map_err(|err| format!("cannot make {}: {err}", folder.display()))?;
                        }
                        std::fs::write(&path, &write.content).map_err(|err| format!("cannot write {}: {err}", write.path.display()))
                    }) {
                        Ok(()) => responder.respond(WriteTextFileResponse::new()),
                        Err(why) => responder.respond_with_error(acp::Error::invalid_params().data(why)),
                    },
                    acp::on_receive_request!(),
                )
                .connect_with(transport, async move |cx: acp::ConnectionTo<acp::Agent>| {
                    // Files read and written, never a terminal: Clarity runs nothing for the agent.
                    let capabilities = ClientCapabilities::new().fs(FileSystemCapabilities::new().read_text_file(true).write_text_file(true)).terminal(false);
                    let init = cx.send_request(InitializeRequest::new(ProtocolVersion::V1).client_capabilities(capabilities)).block_task().await;
                    let init = match init {
                        Ok(init) => init,
                        Err(err) => {
                            let _ = ready.send(Err(format!("the agent did not start: {err}")));
                            return Ok(());
                        }
                    };
                    let name = init.agent_info.as_ref().map(|i| i.name.clone()).unwrap_or_default();
                    let _ = ready.send(Ok((name, init.auth_methods.iter().map(sign_in_of).collect())));
                    while let Some(command) = queue.recv().await {
                        let cx = cx.clone();
                        match command {
                            Command::SignIn(id, reply) => {
                                cx.clone().spawn(async move {
                                    let result = cx.send_request(AuthenticateRequest::new(id)).block_task().await;
                                    let _ = reply.send(result.map(|_| ()).map_err(|err| failure(err, "signing in failed")));
                                    Ok(())
                                })?;
                            }
                            Command::NewSession(servers, reply) => {
                                let request = NewSessionRequest::new(session_root.clone()).mcp_servers(servers.into_iter().map(server_of).collect());
                                cx.clone().spawn(async move {
                                    let result = cx.send_request(request).block_task().await;
                                    let _ = reply.send(result.map(|r| r.session_id.to_string()).map_err(|err| failure(err, "the session did not open")));
                                    Ok(())
                                })?;
                            }
                            Command::Prompt(session, text, reply) => {
                                let request = PromptRequest::new(SessionId::new(session), vec![ContentBlock::Text(TextContent::new(text))]);
                                cx.clone().spawn(async move {
                                    let result = cx.send_request(request).block_task().await;
                                    let _ = reply.send(result.map(|r| ended_of(r.stop_reason)).map_err(|err| failure(err, "the turn failed")));
                                    Ok(())
                                })?;
                            }
                            Command::Cancel(session) => cx.send_notification(CancelNotification::new(SessionId::new(session)))?,
                        }
                    }
                    Ok(())
                })
                .await;
            if let Err(err) = ended {
                eprintln!("clarity-acp: the agent's connection ended: {err}");
            }
        });
        let (name, sign_ins) = started.await.map_err(|_| "the agent stopped before it started".to_string())??;
        Ok(Agent { name, sign_ins, commands })
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
        self.ask(|reply| Command::SignIn(id.to_string(), reply)).await
    }

    /// Opens a session in the project folder with these tools; its id.
    pub async fn new_session(&self, tools: Vec<ToolServer>) -> Result<String, Failure> {
        self.ask(|reply| Command::NewSession(tools, reply)).await
    }

    /// Sends what the person typed; how the turn ended.
    pub async fn prompt(&self, session: &str, text: &str) -> Result<Ended, Failure> {
        self.ask(|reply| Command::Prompt(session.to_string(), text.to_string(), reply)).await
    }

    /// Asks the agent to stop the turn; the turn then ends as Cancelled.
    pub fn cancel(&self, session: &str) {
        let _ = self.commands.send(Command::Cancel(session.to_string()));
    }

    async fn ask<T>(&self, command: impl FnOnce(oneshot::Sender<Result<T, Failure>>) -> Command) -> Result<T, Failure> {
        let stopped = || Failure::Other("the agent has stopped".to_string());
        let (reply, answer) = oneshot::channel();
        self.commands.send(command(reply)).map_err(|_| stopped())?;
        answer.await.map_err(|_| stopped())?
    }
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

/// What a tool is and the files it touches, as the agent told of it.
#[derive(Debug, Clone, Default)]
struct Known {
    kind: Option<ToolKind>,
    locations: Vec<PathBuf>,
}

/// Keeps what a tool is and touches, from its announcement and its updates.
fn remember(known: &Mutex<HashMap<String, Known>>, update: &SessionUpdate) {
    let Ok(mut known) = known.lock() else { return };
    match update {
        SessionUpdate::ToolCall(call) => {
            known.insert(call.tool_call_id.to_string(), Known { kind: Some(call.kind.clone()), locations: call.locations.iter().map(|l| l.path.clone()).collect() });
        }
        SessionUpdate::ToolCallUpdate(update) => {
            let entry = known.entry(update.tool_call_id.to_string()).or_default();
            if let Some(kind) = &update.fields.kind {
                entry.kind = Some(kind.clone());
            }
            if let Some(locations) = &update.fields.locations {
                entry.locations = locations.iter().map(|l| l.path.clone()).collect();
            }
        }
        _ => {}
    }
}

/// Whether a permission is the client's to give without asking: reading,
/// searching or changing files, every one of them named and inside the
/// project. Running anything, fetching, or a file outside (or none named)
/// goes to the person.
fn allowed_by_itself(root: &Path, known: &Mutex<HashMap<String, Known>>, asked: &RequestPermissionRequest) -> bool {
    let mut tool = known.lock().ok().and_then(|k| k.get(&asked.tool_call.tool_call_id.to_string()).cloned()).unwrap_or_default();
    if let Some(kind) = &asked.tool_call.fields.kind {
        tool.kind = Some(kind.clone());
    }
    if let Some(locations) = &asked.tool_call.fields.locations {
        tool.locations = locations.iter().map(|l| l.path.clone()).collect();
    }
    let files_only = matches!(tool.kind, Some(ToolKind::Read | ToolKind::Edit | ToolKind::Delete | ToolKind::Move | ToolKind::Search));
    files_only && !tool.locations.is_empty() && tool.locations.iter().all(|path| inside(root, path).is_ok())
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

/// Allowed is once, never always, so the next command asks again; denied
/// is rejected once, or the request cancelled when it offers no rejection.
fn outcome(asked: &RequestPermissionRequest, decision: Decision) -> RequestPermissionOutcome {
    let wanted = match decision {
        Decision::Allow => PermissionOptionKind::AllowOnce,
        Decision::Deny => PermissionOptionKind::RejectOnce,
    };
    match asked.options.iter().find(|o| o.kind == wanted) {
        Some(option) => RequestPermissionOutcome::Selected(SelectedPermissionOutcome::new(option.option_id.clone())),
        None => RequestPermissionOutcome::Cancelled,
    }
}

fn event_of(update: SessionUpdate) -> Option<Event> {
    let text = |block: ContentBlock| match block {
        ContentBlock::Text(t) => Some(t.text),
        _ => None,
    };
    match update {
        SessionUpdate::AgentMessageChunk(chunk) => text(chunk.content).map(Event::Text),
        SessionUpdate::AgentThoughtChunk(chunk) => text(chunk.content).map(Event::Thought),
        SessionUpdate::ToolCall(call) => Some(Event::Tool { id: call.tool_call_id.to_string(), title: call.title, status: status_of(call.status) }),
        SessionUpdate::ToolCallUpdate(update) => Some(Event::ToolUpdate {
            id: update.tool_call_id.to_string(),
            title: update.fields.title,
            status: update.fields.status.map(status_of),
        }),
        _ => None,
    }
}

fn status_of(status: ToolCallStatus) -> ToolStatus {
    match status {
        ToolCallStatus::Pending => ToolStatus::Pending,
        ToolCallStatus::InProgress => ToolStatus::Running,
        ToolCallStatus::Completed => ToolStatus::Done,
        ToolCallStatus::Failed => ToolStatus::Failed,
        _ => ToolStatus::Running,
    }
}

fn ended_of(reason: StopReason) -> Ended {
    match reason {
        StopReason::EndTurn => Ended::Done,
        StopReason::MaxTokens | StopReason::MaxTurnRequests => Ended::TooLong,
        StopReason::Refusal => Ended::Refused,
        StopReason::Cancelled => Ended::Cancelled,
        _ => Ended::Done,
    }
}

fn sign_in_of(method: &AuthMethod) -> SignIn {
    match method {
        AuthMethod::Agent(m) => SignIn { id: m.id.to_string(), name: m.name.clone(), description: m.description.clone() },
        AuthMethod::Terminal(m) => SignIn { id: m.id.to_string(), name: m.name.clone(), description: m.description.clone() },
        _ => SignIn { id: String::new(), name: String::new(), description: None },
    }
}

fn server_of(server: ToolServer) -> McpServer {
    McpServer::Stdio(McpServerStdio::new(server.name, server.command).args(server.args))
}
