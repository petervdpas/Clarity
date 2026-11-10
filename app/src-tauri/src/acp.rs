//! The person's own agent in the chat panel (docs/ACP-PLAN.md): a vault
//! connection of kind `codex` or `copilot` starts that vendor's program over
//! ACP (crates/clarity-acp), signed in with its own login, so no API key.
//!
//! The panel shows its turns as it shows Clarity's own agent's: the same
//! events, the same Allow/Deny, one Undo per turn. A turn is a snapshot of
//! the project before and after (the agent also writes files itself); the
//! agent's tools are `clarity mcp --turn`, whose show-a-window and
//! show-a-diagram the panel carries out when the turn ends.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::mpsc::RecvTimeoutError;
use std::time::Duration;

use clarity_acp::vendors::{SignIn as VendorSignIn, Vendor, find};
use clarity_acp::{Agent, Decision, Ended, Event as AcpEvent, Failure, Host, Permission, ToolServer, ToolStatus};
use clarity_agent::Event;
use clarity_tools::turn::{self, Shown};
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::{Mutex, mpsc, oneshot};

use crate::agent::{AGENT_EVENT, AgentAnswered, AgentAsk, AgentEnded, AgentState};
use crate::fail_safe::Held;
use super::{HistoryState, WatchState, open_project_of, record};

/// The outside agent of the panel, while one runs: one program per panel.
#[derive(Default)]
pub struct AcpState {
    running: Mutex<Option<Running>>,
}

struct Running {
    connection: String,
    project: PathBuf,
    agent: Agent,
    session: String,
    /// Where `clarity mcp --turn` writes what to show.
    shown: PathBuf,
    /// Whether the session has had Clarity's instructions (with its first message).
    instructed: bool,
}

/// What the outside agent is told of its place, beside Clarity's shared
/// instructions: it does not know Clarity, and its own tools reach past the
/// project and skip the checks.
const PANEL_LINES: [&str; 3] = [
    "You work in Clarity's Agent panel. Clarity's tools come from the MCP server named clarity: use them for the project. Write files with its write_file, which checks each write, and run its check before you finish. Never build or run Clarity or the project yourself.",
    "Stay inside the project folder: never search, read or write outside it.",
    "show_window and show_diagram show a window in Preview or a diagram when your turn ends; a project new_project makes opens then too.",
];

/// The vendor of a connection, when it is an outside agent's.
pub fn vendor_of(app: &AppHandle, connection: &str) -> Option<Vendor> {
    let vault = app.state::<super::vault::VaultState>();
    super::vault::resolve(&vault, connection, "kind").ok().and_then(|kind| Vendor::of_kind(&kind))
}

/// Starts a turn of the outside agent of `connection`; reports through the panel's events.
pub fn send(app: AppHandle, connection: String, vendor: Vendor, text: String) -> Result<(), String> {
    let (file, dir) = open_project_of(&app.state::<WatchState>()).map_err(|_| format!("open a project first: {} works on the project open in Clarity", vendor.name()))?;
    let from_settings = {
        let vault = app.state::<super::vault::VaultState>();
        super::vault::resolve(&vault, &connection, "command").ok().filter(|c| !c.trim().is_empty()).map(PathBuf::from)
    };
    let program = find(vendor, from_settings.as_deref())?;
    let state = app.state::<AgentState>();
    if state.running.swap(true, Ordering::SeqCst) {
        return Err("the agent is still working; stop it first".into());
    }
    state.stop.store(false, Ordering::SeqCst);
    tauri::async_runtime::spawn(async move {
        let result = turn(&app, &connection, vendor, &program, &file, &dir, &text).await;
        let nothing = AgentEnded { opened: None, shown: None, diagram: None, changed: false, sign_in: false };
        let ended = match result {
            Ok(ended) => ended,
            // The program says it is not signed in: the panel offers Sign in.
            Err(Failure::NotSignedIn) => {
                let _ = app.emit(AGENT_EVENT, &Event::Failed { message: format!("{} is not signed in. Click Sign in, then ask again.", vendor.name()) });
                AgentEnded { sign_in: true, ..nothing }
            }
            Err(Failure::Other(message)) => {
                let _ = app.emit(AGENT_EVENT, &Event::Failed { message });
                nothing
            }
        };
        app.state::<AgentState>().running.store(false, Ordering::SeqCst);
        let _ = app.emit("agent-ended", &ended);
    });
    Ok(())
}

async fn turn(app: &AppHandle, connection: &str, vendor: Vendor, program: &clarity_acp::Program, file: &Path, dir: &Path, text: &str) -> Result<AgentEnded, Failure> {
    let acp = app.state::<AcpState>();
    let mut running = acp.running.lock().await;
    // Another connection or another project: a program of its own.
    if running.as_ref().is_some_and(|r| r.connection != connection || r.project != file) {
        *running = None;
    }
    if running.is_none() {
        *running = Some(started(app, connection, vendor, program, file, dir).await?);
    }
    let r = running.as_mut().ok_or_else(|| Failure::Other("the agent did not start".into()))?;
    // The first message of a session brings Clarity's instructions with it.
    let text = if r.instructed { text.to_string() } else { format!("{}\n\nThe person asks:\n{text}", clarity_tools::instructions(&PANEL_LINES)) };
    r.instructed = true;
    // The turn keeps its own handle, so Stop reaches the agent while it works.
    let (agent, session, shown_file) = (r.agent.clone(), r.session.clone(), r.shown.clone());
    drop(running);
    let _ = std::fs::remove_file(&shown_file);
    let before = turn::take(file, dir);
    let ended = agent.prompt(&session, &text).await;
    if ended == Err(Failure::NotSignedIn) {
        // Signed out since it started: the next turn starts it again, after Sign in.
        forget(app);
        return Err(Failure::NotSignedIn);
    }
    // Whatever the turn changed goes into its migration step, then into one Undo.
    let followed = clarity_tools::steps::follow(file, dir, &[]);
    let after = turn::take(file, dir);
    let changes = turn::changes(&before, &after);
    let event = match &ended {
        Ok(Ended::Done) => Event::Finished,
        Ok(Ended::Cancelled) => Event::Stopped,
        Ok(Ended::TooLong) => Event::Failed { message: format!("{} stopped: the turn grew too long. Ask again to go on.", vendor.name()) },
        Ok(Ended::Refused) => Event::Failed { message: format!("{} refused this request.", vendor.name()) },
        Err(failure) => Event::Failed { message: failure.to_string() },
    };
    let _ = app.emit(AGENT_EVENT, &event);
    if let Err(message) = followed {
        let _ = app.emit(AGENT_EVENT, &Event::Failed { message: format!("the migration step was not written: {message}") });
    }
    if !changes.others.is_empty() {
        let names: Vec<String> = changes.others.iter().map(|p| clarity_project::slashed(p.strip_prefix(dir).unwrap_or(p))).collect();
        let _ = app.emit(AGENT_EVENT, &Event::Failed { message: format!("Undo cannot take back these files, which are not text: {}", names.join(", ")) });
    }
    let changed = !changes.files.is_empty();
    let mut ended = AgentEnded { opened: None, shown: None, diagram: None, changed, sign_in: false };
    for shown in turn::shown(&shown_file) {
        match shown {
            Shown::Window(name) => ended.shown = Some(name),
            Shown::Diagram(name) => ended.diagram = Some(name),
            Shown::Project(path) => ended.opened = Some(path.display().to_string()),
        }
    }
    // A project made during the turn opens with a history of its own.
    if ended.opened.is_none() && changed {
        let _ = record(&app.state::<HistoryState>(), changes.files);
    }
    Ok(ended)
}

/// Starts the program for the project, signs in when it must, and opens a session with Clarity's tools.
async fn started(app: &AppHandle, connection: &str, vendor: Vendor, program: &clarity_acp::Program, file: &Path, dir: &Path) -> Result<Running, Failure> {
    let (events, mut said) = mpsc::unbounded_channel::<AcpEvent>();
    let (permissions, mut asked) = mpsc::unbounded_channel::<(Permission, oneshot::Sender<Decision>)>();
    let to_panel = app.clone();
    tauri::async_runtime::spawn(async move {
        let mut titles: HashMap<String, String> = HashMap::new();
        while let Some(event) = said.recv().await {
            for event in panel_event(event, &mut titles) {
                let _ = to_panel.emit(AGENT_EVENT, &event);
            }
        }
    });
    let asker = app.clone();
    tauri::async_runtime::spawn(async move {
        while let Some((permission, answer)) = asked.recv().await {
            let app = asker.clone();
            let allowed = tauri::async_runtime::spawn_blocking(move || ask(&app, &permission.title)).await.unwrap_or(false);
            let _ = answer.send(if allowed { Decision::Allow } else { Decision::Deny });
        }
    });
    let shown = std::env::temp_dir().join(format!("clarity-turn-{}-{}.jsonl", std::process::id(), connection.replace(|c: char| !c.is_ascii_alphanumeric(), "_")));
    let tools = ToolServer { name: "clarity".into(), command: clarity_cli(), args: vec!["mcp".into(), "--turn".into(), shown.display().to_string()] };
    // Clarity's tools go with the program's start for a vendor that takes them so (Copilot), else with the session.
    let mut program = program.clone();
    let session_tools = match vendor.tools_at_start(&tools) {
        Some(args) => {
            program.args.extend(args);
            Vec::new()
        }
        None => vec![tools],
    };
    let agent = Agent::start(&program, dir, Host { events, permissions }).await.map_err(Failure::Other)?;
    if let Some(mode) = vendor.ask_first_mode() {
        // The person chose this for Codex (docs/ACP-PLAN.md): it is said once, when the program starts.
        let _ = app.emit(AGENT_EVENT, &Event::Text {
            text: format!("{} works in its {mode} mode: it may run commands that only read, inside its own sandbox, without asking. Anything that changes files or reaches further asks you first.\n\n", vendor.name()),
        });
    }
    let session = match agent.new_session(session_tools.clone()).await {
        Ok(session) => session,
        // Not signed in, and it signs in over ACP (Codex): with its own method, by name, then the session opens.
        Err(Failure::NotSignedIn) if matches!(vendor.sign_in(), VendorSignIn::Acp(_)) => {
            let way = acp_way(&agent, vendor)?;
            let _ = app.emit(AGENT_EVENT, &Event::Text { text: format!("Signing in to {}: {}.\n\n", vendor.name(), way.name) });
            agent.sign_in(&way.id).await?;
            agent.new_session(session_tools).await?
        }
        Err(err) => return Err(err),
    };
    Ok(Running { connection: connection.to_string(), project: file.to_path_buf(), agent, session, shown, instructed: false })
}

/// The vendor's own way to sign in over ACP, by its id: never another the program offers
/// (Codex lists an API key first, which this panel has no use for).
fn acp_way(agent: &Agent, vendor: Vendor) -> Result<clarity_acp::SignIn, Failure> {
    let VendorSignIn::Acp(id) = vendor.sign_in() else {
        return Err(Failure::Other(format!("{} does not sign in here", vendor.name())));
    };
    agent.sign_ins().iter().find(|way| way.id == id).cloned().ok_or_else(|| {
        let offered: Vec<&str> = agent.sign_ins().iter().map(|w| w.id.as_str()).collect();
        Failure::Other(format!("{} offers no sign-in {id} here (it offers {}); is NO_BROWSER set?", vendor.name(), if offered.is_empty() { "none".to_string() } else { offered.join(", ") }))
    })
}

/// Asks in the panel, as Clarity's own agent asks; Stop denies.
fn ask(app: &AppHandle, what: &str) -> bool {
    let state = app.state::<AgentState>();
    let id = state.last_ask.fetch_add(1, Ordering::SeqCst) + 1;
    let (send, answer) = std::sync::mpsc::channel();
    if let Ok(mut asking) = state.asking.held() {
        *asking = Some((id, send));
    }
    let _ = app.emit("agent-ask", AgentAsk { id, what: what.to_string() });
    let allowed = loop {
        match answer.recv_timeout(Duration::from_millis(200)) {
            Ok(allowed) => break allowed,
            Err(RecvTimeoutError::Timeout) if !state.stop.load(Ordering::SeqCst) => {}
            Err(_) => break false,
        }
    };
    if let Ok(mut asking) = state.asking.held() {
        *asking = None;
    }
    let _ = app.emit("agent-answered", AgentAnswered { id, allowed });
    allowed
}

/// The outside agent's event as the panel shows Clarity's own agent's: a
/// tool line starts, and is marked done by its id.
fn panel_event(event: AcpEvent, titles: &mut HashMap<String, String>) -> Vec<Event> {
    let done = |id: String, name: String, status: ToolStatus| Event::ToolDone { id, name, text: String::new(), is_error: status == ToolStatus::Failed };
    match event {
        AcpEvent::Text(text) => vec![Event::Text { text }],
        AcpEvent::Thought(_) => Vec::new(),
        AcpEvent::Tool { id, title, status } => {
            titles.insert(id.clone(), title.clone());
            let start = Event::ToolStart { id: id.clone(), name: title.clone(), input: Value::Null };
            match status {
                // Told of when it is done already: its line, then its mark.
                ToolStatus::Done | ToolStatus::Failed => vec![start, done(id, title, status)],
                _ => vec![start],
            }
        }
        AcpEvent::ToolUpdate { id, title, status } => {
            if let Some(title) = &title {
                titles.insert(id.clone(), title.clone());
            }
            match status {
                Some(status @ (ToolStatus::Done | ToolStatus::Failed)) => {
                    let name = titles.get(&id).cloned().unwrap_or_default();
                    vec![done(id, name, status)]
                }
                _ => Vec::new(),
            }
        }
    }
}

/// The `clarity` command the IDE ships beside itself; in development, the one built beside it.
fn clarity_cli() -> PathBuf {
    let name = if cfg!(windows) { "clarity.exe" } else { "clarity" };
    std::env::current_exe().ok().and_then(|exe| exe.parent().map(|dir| dir.join(name))).filter(|p| p.is_file()).unwrap_or_else(|| PathBuf::from(name))
}

/// Stops the outside agent's turn, when one runs.
pub fn cancel(app: &AppHandle) {
    let acp = app.state::<AcpState>();
    if let Ok(running) = acp.running.try_lock()
        && let Some(r) = running.as_ref()
    {
        r.agent.cancel(&r.session);
    }
}

/// Lets the outside agent go, for a new conversation: its program stops.
pub fn forget(app: &AppHandle) {
    let acp = app.state::<AcpState>();
    if let Ok(mut running) = acp.running.try_lock() {
        *running = None;
    }
}

/// Signs in to the outside agent of `connection` the vendor's own way: a
/// browser opens. Run only when the person asks for it (the panel's Sign in).
#[tauri::command]
pub async fn agent_sign_in(connection: String, app: AppHandle) -> Result<String, String> {
    let vendor = vendor_of(&app, &connection).ok_or_else(|| format!("the connection {connection} is not an outside agent's; its kind is codex or copilot"))?;
    let from_settings = {
        let vault = app.state::<super::vault::VaultState>();
        super::vault::resolve(&vault, &connection, "command").ok().filter(|c| !c.trim().is_empty()).map(PathBuf::from)
    };
    let program = find(vendor, from_settings.as_deref())?;
    match vendor.sign_in() {
        VendorSignIn::Command(args) => {
            let status = tauri::async_runtime::spawn_blocking(move || std::process::Command::new(&program.command).args(args).envs(program.env.iter().cloned()).status())
                .await
                .map_err(|err| err.to_string())?
                .map_err(|err| format!("cannot start {}: {err}", vendor.name()))?;
            if !status.success() {
                return Err(format!("{} did not sign in ({status})", vendor.name()));
            }
        }
        VendorSignIn::Acp(_) => {
            let dir = open_project_of(&app.state::<WatchState>()).map(|(_, dir)| dir).unwrap_or_else(|_| std::env::temp_dir());
            let (events, _said) = mpsc::unbounded_channel();
            let (permissions, _asked) = mpsc::unbounded_channel();
            let agent = Agent::start(&program, &dir, Host { events, permissions }).await?;
            let way = acp_way(&agent, vendor)?;
            agent.sign_in(&way.id).await?;
        }
    }
    Ok(format!("Signed in to {}.", vendor.name()))
}

/// One call an outside agent made through `clarity mcp`, for the activity list.
#[derive(Debug, Clone, serde::Serialize, ts_rs::TS)]
#[ts(export)]
pub struct AgentActivity {
    #[ts(type = "number")]
    pub seq: u64,
    /// When, in seconds since 1970.
    #[ts(type = "number")]
    pub time: u64,
    /// The program that called, as it named itself (like codex).
    pub client: String,
    pub tool: String,
    pub path: Option<String>,
    pub ok: bool,
    pub said: String,
}

/// Whether outside agents may work on the open project (Clarity settings, Agent).
#[tauri::command]
pub fn outside_agents() -> bool {
    clarity_project::ide::outside_agents()
}

/// Lets outside agents work on the project, or not; holds from their next call.
#[tauri::command]
pub fn set_outside_agents(allowed: bool) -> Result<bool, String> {
    clarity_project::ide::set_outside_agents(allowed)?;
    Ok(clarity_project::ide::outside_agents())
}

/// The calls outside agents made after call `after`, oldest first.
#[tauri::command]
pub fn agent_activity(after: u64) -> Vec<AgentActivity> {
    clarity_project::activity::after(&clarity_project::activity::file(), after)
        .into_iter()
        .map(|a| AgentActivity { seq: a.seq, time: a.time, client: a.client, tool: a.tool, path: a.path, ok: a.ok, said: a.said })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The outside agent's events read as Clarity's own agent's in the panel:
    /// a tool starts, ends by its id with the title it started with, and
    /// thinking is not shown.
    /// The outside agent is told to keep to Clarity's tools and the project folder, in Clarity's house style.
    #[test]
    fn the_outside_agent_is_told_its_place() {
        let told = clarity_tools::instructions(&PANEL_LINES);
        for words in ["MCP server named clarity", "write_file", "check before you finish", "never search, read or write outside it", "Never build or run"] {
            assert!(told.contains(words), "{words}");
        }
        assert!(!told.contains('\u{2014}') && !told.contains('\u{2013}'));
    }

    #[test]
    fn outside_events_read_as_the_panel_s_own() {
        let mut titles = HashMap::new();
        let start = panel_event(AcpEvent::Tool { id: "t1".into(), title: "write_file design/Main.cxaml".into(), status: ToolStatus::Pending }, &mut titles);
        assert!(matches!(start.as_slice(), [Event::ToolStart { id, name, .. }] if id == "t1" && name == "write_file design/Main.cxaml"));
        assert!(panel_event(AcpEvent::ToolUpdate { id: "t1".into(), title: None, status: Some(ToolStatus::Running) }, &mut titles).is_empty());
        let done = panel_event(AcpEvent::ToolUpdate { id: "t1".into(), title: None, status: Some(ToolStatus::Failed) }, &mut titles);
        assert!(matches!(done.as_slice(), [Event::ToolDone { name, is_error: true, .. }] if name == "write_file design/Main.cxaml"));
        assert!(matches!(panel_event(AcpEvent::Text("Done.".into()), &mut titles).as_slice(), [Event::Text { text }] if text == "Done."));
        assert!(panel_event(AcpEvent::Thought("hmm".into()), &mut titles).is_empty());
        // Told of when it is done already: its line, then its mark.
        let at_once = panel_event(AcpEvent::Tool { id: "t2".into(), title: "check".into(), status: ToolStatus::Done }, &mut titles);
        assert!(matches!(at_once.as_slice(), [Event::ToolStart { .. }, Event::ToolDone { is_error: false, .. }]));
    }
}
