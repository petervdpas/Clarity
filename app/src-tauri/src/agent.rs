//! The agent panel: a conversation with a model that knows Clarity and works
//! on the open project through tools. It reads the project, writes modules
//! and the project file, checks them, makes a new project, and shows a
//! window. Its changes are written at once and checked; one Undo takes a
//! whole turn back. The model is reached through a connection of Clarity's
//! vault, so its key never sits in a file.

use crate::fail_safe::Held;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{RecvTimeoutError, Sender};
use std::time::Duration;

use clarity_agent::{Agent, Event, Message, ToolDef, ToolResult, Tools};
pub use clarity_tools::{ProjectTools, merged};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tauri::{AppHandle, Emitter, Manager, State};
use ts_rs::TS;

use super::{FileChange, HistoryState, WatchState, open_project_of, record};

/// How many steps (model answers) one turn may take.
const MAX_STEPS: usize = 40;

/// What the IDE's agent is told beyond the shared instructions
/// (crates/clarity-tools): its build, backups, Output panel and Preview.
const IDE_LINES: [&str; 5] = [
    "Before you change data in bulk, or when the person asks, take a backup with backup_data; list_backups shows what there is and \
restore_backup puts one back. Both changing tools ask the person first.",
    "Open a diagram you wrote with show_diagram.",
    "Show a finished window with show_window.",
    "Read a failed build's or run's errors with build_output; never ask for them to be pasted.",
    "To run or build the app use the build tool, which may ask the person first: build once the windows are \
written and fix what it reports, and when they deny it, do not ask again in this turn. Without a build tool (Clarity's settings turned it off), ask \
the person to press Run or Build, then read build_output.",
];

/// What the model is told before the conversation: the shared instructions
/// with the IDE's own lines.
fn instructions() -> String {
    clarity_tools::instructions(&IDE_LINES)
}

/// Whether the agent may run or build the application, as Clarity's
/// settings (Agent) say: ask every time, allow without asking, or never.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum Runs {
    #[default]
    Ask,
    Allow,
    Never,
}

/// What the agent does only after the person allows it: running or building
/// the application. The agent executes nothing else.
pub trait Execute {
    /// Asks the person, in the panel, whether the agent may do `what`; true when they allow it.
    fn ask(&mut self, what: &str) -> bool;
    /// Generates and builds the project (with `run`, starts it too), as the
    /// toolbar does, and waits for the end; gives the build's output.
    fn build(&mut self, file: &Path, run: bool) -> Result<String, String>;
    /// Builds the custom part or library `name` of the project in `dir` on
    /// its own, in its folder, and waits for the end; gives the output.
    fn build_part(&mut self, dir: &Path, name: &str) -> Result<String, String>;
    /// Ends the application Run started, which has the database open, before a restore.
    fn stop_app(&mut self) {}
}


/// The IDE's agent's tools: the shared ones that design and generate, and
/// what only the IDE has: building and running after the person allows it,
/// backups, the Output panel and Preview.
pub struct IdeTools {
    pub core: ProjectTools,
    /// The window to show in Preview, when a tool asked for one.
    pub shown: Option<String>,
    /// The diagram to open, by its file name, when a tool asked for one.
    pub shown_diagram: Option<String>,
    /// The Output panel as the turn started: the last build's lines and how it ended.
    pub output: String,
    /// How the build tool asks the person and builds; without it, nothing runs.
    pub execute: Option<Box<dyn Execute>>,
    /// Whether the build tool asks first, runs without asking, or is not there.
    pub runs: Runs,
}

impl IdeTools {
    pub fn new(project: Option<(PathBuf, PathBuf)>, projects_folder: PathBuf) -> IdeTools {
        IdeTools {
            core: ProjectTools::new(project, projects_folder),
            shown: None,
            shown_diagram: None,
            output: crate::build::Output::default().text(),
            execute: None,
            runs: Runs::Ask,
        }
    }

    /// Runs or builds the application, after the person allows it.
    fn build(&mut self, input: &Value) -> Result<String, String> {
        let run = input["run"].as_bool().unwrap_or(false);
        let (file, dir) = self.core.open()?.clone();
        if self.runs == Runs::Never {
            return Err("running and building are turned off in Clarity's settings (Agent); ask the person to press Run or Build".into());
        }
        // A custom part or a library, built on its own in its folder.
        if let Some(part) = input["part"].as_str().map(str::trim).filter(|p| !p.is_empty()) {
            let folder = self.core.part_folders()?.into_iter().find(|f| f.name == part).ok_or_else(|| format!("there is no custom part or library {part} with a folder"))?;
            let project = clarity_project::load_modules(&dir).map_err(|err| format!("cannot read the project: {err}"))?;
            if !super::parts::builds_alone(&project, part) {
                return Err(format!("{} {part} is built with the program: build the application instead", folder.kind));
            }
            let execute = self.execute.as_mut().ok_or("running and building are not available here")?;
            let what = format!("Build {} {part} on its own, in {}", folder.kind, folder.folder);
            if self.runs == Runs::Ask && !execute.ask(&what) {
                return Ok("The person denied this, so nothing ran. Do not ask again in this turn; say what they can do instead.".into());
            }
            let output = execute.build_part(&dir, part)?;
            self.output = output.clone();
            return Ok(output);
        }
        let execute = self.execute.as_mut().ok_or("running and building are not available here")?;
        let what = if run { "Run the application: generate it, build it and start it" } else { "Build the application: generate it and build it" };
        if self.runs == Runs::Ask && !execute.ask(what) {
            return Ok("The person denied this, so nothing ran. Do not ask again in this turn; say what they can do instead.".into());
        }
        let output = execute.build(&file, run)?;
        self.output = output.clone();
        Ok(output)
    }

    /// The data source a backup tool names, or the first SQLite one the project owns.
    fn backup_source(&self, input: &Value) -> Result<(PathBuf, String), String> {
        let (_, dir) = self.core.open()?.clone();
        let named = input["source"].as_str().map(str::trim).filter(|s| !s.is_empty());
        let (source, _) = clarity_project::database_to_back_up(&dir, named)?;
        Ok((dir, source))
    }

    fn list_backups(&self, input: &Value) -> Result<String, String> {
        let (dir, source) = self.backup_source(input)?;
        let list = super::backups::list(&dir, &source);
        if let Some(why) = list.unavailable {
            return Err(why);
        }
        if list.backups.is_empty() {
            return Ok(format!("{source} has no backups yet."));
        }
        Ok(list.backups.iter().map(|b| format!("{}  {}  {} ({} bytes)", b.file, b.when, b.why, b.bytes)).collect::<Vec<_>>().join("\n"))
    }

    fn backup_data(&mut self, input: &Value) -> Result<String, String> {
        let (dir, source) = self.backup_source(input)?;
        let execute = self.execute.as_mut().ok_or("backing up is not available here")?;
        if !execute.ask(&format!("Back up the database of {source}")) {
            return Ok("The person denied this, so nothing was backed up. Do not ask again in this turn.".into());
        }
        Ok(super::backups::take(&dir, &source)?.done.unwrap_or_default())
    }

    fn restore_backup(&mut self, input: &Value) -> Result<String, String> {
        let (dir, source) = self.backup_source(input)?;
        let file = input["file"].as_str().map(str::trim).filter(|f| !f.is_empty()).ok_or("name the backup by its file, as list_backups gives it")?.to_string();
        let execute = self.execute.as_mut().ok_or("restoring is not available here")?;
        let what = format!("Restore the database of {source} from {file} (what it holds now is backed up first; a running application is stopped)");
        if !execute.ask(&what) {
            return Ok("The person denied this, so nothing was restored. Do not ask again in this turn.".into());
        }
        execute.stop_app();
        Ok(super::backups::restore(&dir, &source, &file)?.done.unwrap_or_default())
    }
}

fn schema(properties: Value, required: &[&str]) -> Value {
    json!({ "type": "object", "properties": properties, "required": required })
}

impl Tools for IdeTools {
    fn definitions(&self) -> Vec<ToolDef> {
        let tool = |name: &str, description: &str, input_schema: Value| ToolDef { name: name.into(), description: description.into(), input_schema };
        let mut tools = self.core.definitions();
        tools.extend([
            tool(
                "list_backups",
                "Lists the backups of a data source's database, the newest first, with when (UTC) and why each was taken; or says why Clarity does not back that source up.",
                schema(json!({ "source": { "type": "string", "description": "The data source; the first one by default." } }), &[]),
            ),
            tool(
                "backup_data",
                "Takes a backup of a data source's database, beside it in backups/. It asks the person first. Take one before you change data in bulk or restore.",
                schema(json!({ "source": { "type": "string", "description": "The data source; the first one by default." } }), &[]),
            ),
            tool(
                "restore_backup",
                "Puts a backup in place of a data source's database, after backing up what is there now; a running application is stopped first. It asks the person first. Name the backup by its file, as list_backups gives it.",
                schema(json!({ "file": { "type": "string", "description": "The backup's file name, like shop-20261105T213012Z-manual.db." }, "source": { "type": "string", "description": "The data source; the first one by default." } }), &["file"]),
            ),
            tool(
                "build_output",
                "Reads Clarity's Output panel: the last Run or Build of the application (or of an engine), how it ended, and the toolchain's lines, such as compiler errors in the generated code.",
                schema(json!({}), &[]),
            ),
            tool("show_window", "Shows a window of the project in Clarity's Preview.", schema(json!({ "name": { "type": "string" } }), &["name"])),
            tool("show_diagram", "Opens a written diagram in Clarity, drawn with what it proposes marked.", schema(json!({ "name": { "type": "string" } }), &["name"])),
        ]);
        // Clarity's settings can turn running off; then the agent has no such tool.
        if self.runs != Runs::Never {
            tools.push(tool(
                "build",
                "Runs the application (run: true: generate, build and start it, as the Run button does) or builds it (run: false, as the Build button does). With part, builds that custom part or library on its own in its folder (C and C++ with CMake, Rust with Cargo, Go with go build), to see its compiler's errors. The person may be asked first and may deny it. Gives the build's output.",
                schema(
                    json!({
                        "run": { "type": "boolean", "description": "true to start the application after building it." },
                        "part": { "type": "string", "description": "Optional: a custom part or library to build on its own, like Arena." }
                    }),
                    &[],
                ),
            ));
        }
        tools
    }

    fn call(&mut self, name: &str, input: &Value) -> ToolResult {
        let text = |key: &str| input[key].as_str().unwrap_or_default().to_string();
        let result = match name {
            "list_backups" => self.list_backups(input),
            "backup_data" => self.backup_data(input),
            "restore_backup" => self.restore_backup(input),
            "build_output" => Ok(self.output.clone()),
            "build" => self.build(input),
            "show_window" => {
                self.shown = Some(text("name"));
                Ok(format!("Showing {} in Preview.", text("name")))
            }
            "show_diagram" => self.core.diagram_text(&text("name")).map(|(file, _)| {
                let shown = file.file_name().unwrap_or_default().to_string_lossy().to_string();
                self.shown_diagram = Some(shown.clone());
                format!("Opening diagrams/{shown} in Clarity.")
            }),
            _ => {
                let mut result = self.core.call(name, input);
                // The IDE opens a project its agent made, and the turn goes on with it.
                if name == "new_project" && !result.is_error {
                    result.text.push_str("\n\nIt is the project the tools work on now, and it opens in Clarity when this turn ends.");
                }
                return result;
            }
        };
        match result {
            Ok(text) => ToolResult { text, is_error: false },
            Err(text) => ToolResult { text, is_error: true },
        }
    }
}

/// The conversation, and whether a turn runs.
#[derive(Default)]
pub struct AgentState {
    conversation: Mutex<Vec<Message>>,
    pub(crate) running: AtomicBool,
    pub(crate) stop: Arc<AtomicBool>,
    /// The question waiting for the person's answer, and where the answer goes.
    pub(crate) asking: Mutex<Option<(u32, Sender<bool>)>>,
    pub(crate) last_ask: AtomicU32,
}

/// The agent asks the person whether it may do something.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct AgentAsk {
    pub id: u32,
    pub what: String,
}

/// The person's answer (or a stop, which denies).
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct AgentAnswered {
    pub id: u32,
    pub allowed: bool,
}

/// Asks in the panel and builds with the toolbar's Run and Build.
struct AppExecute {
    app: AppHandle,
    stop: Arc<AtomicBool>,
}

impl Execute for AppExecute {
    fn ask(&mut self, what: &str) -> bool {
        let state = self.app.state::<AgentState>();
        let id = state.last_ask.fetch_add(1, Ordering::SeqCst) + 1;
        let (send, answer) = std::sync::mpsc::channel();
        if let Ok(mut asking) = state.asking.held() {
            *asking = Some((id, send));
        }
        let _ = self.app.emit("agent-ask", AgentAsk { id, what: what.to_string() });
        // Waits for Allow or Deny; Stop denies.
        let allowed = loop {
            match answer.recv_timeout(Duration::from_millis(200)) {
                Ok(allowed) => break allowed,
                Err(RecvTimeoutError::Timeout) if !self.stop.load(Ordering::SeqCst) => {}
                Err(_) => break false,
            }
        };
        if let Ok(mut asking) = state.asking.held() {
            *asking = None;
        }
        let _ = self.app.emit("agent-answered", AgentAnswered { id, allowed });
        allowed
    }

    fn build(&mut self, file: &Path, run: bool) -> Result<String, String> {
        // As the toolbar: Run is a debug build that starts, Build a release build.
        crate::build::start(self.app.clone(), file.to_path_buf(), !run, run, None)?;
        Ok(self.finished())
    }

    fn build_part(&mut self, dir: &Path, name: &str) -> Result<String, String> {
        crate::parts::start_part_build(self.app.clone(), dir, name)?;
        Ok(self.finished())
    }

    fn stop_app(&mut self) {
        crate::build::stop(&self.app);
    }
}

impl AppExecute {
    /// Waits for the build to end; gives the Output panel's text.
    fn finished(&self) -> String {
        let builds = self.app.state::<crate::build::BuildState>();
        while builds.busy.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(200));
        }
        builds.output.held().map(|o| o.text()).unwrap_or_default()
    }
}

/// The person's answer to the agent's question.
#[tauri::command]
pub fn agent_answer(id: u32, allowed: bool, agent: State<AgentState>) -> Result<(), String> {
    let asking = agent.asking.held().map_err(|_| "the question is not available".to_string())?;
    match asking.as_ref() {
        Some((asked, send)) if *asked == id => {
            let _ = send.send(allowed);
            Ok(())
        }
        _ => Err("the agent is no longer asking this".into()),
    }
}

/// Sent to the screen while a turn runs, and when it ends.
pub(crate) const AGENT_EVENT: &str = "agent-event";

/// What the screen does when a turn ends.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct AgentEnded {
    /// The project made during the turn, to open.
    pub opened: Option<String>,
    /// The window to show in Preview.
    pub shown: Option<String>,
    /// The written diagram to open, by its file name.
    pub diagram: Option<String>,
    /// Whether files changed, so Undo can take the turn back.
    pub changed: bool,
    /// Whether the person's own agent program must sign in first: the panel offers Sign in.
    #[serde(rename = "signIn")]
    #[ts(rename = "signIn")]
    pub sign_in: bool,
}

/// The model of a connection: its kind, address, model, key and length
/// limit. None when its Microsoft sign-in is missing (a `foundry` connection).
fn model_of(app: &AppHandle, connection: &str) -> Result<Option<clarity_agent::anthropic::Anthropic>, String> {
    let vault = app.state::<super::vault::VaultState>();
    let field = |key: &str| super::vault::resolve(&vault, connection, key);
    let max_tokens = field("maxTokens")?.trim().parse().unwrap_or(16384);
    // How long the model keeps the conversation cached: 5m (the default) or 1h.
    let ttl = field("cacheTtl")?;
    let cache = clarity_agent::anthropic::Cache::from_field(&ttl).ok_or_else(|| format!("the connection {connection} has cacheTtl {ttl}; use 5m or 1h"))?;
    let kind = field("kind")?;
    if kind.trim().eq_ignore_ascii_case("foundry") {
        return crate::foundry::model(app, connection, max_tokens, cache);
    }
    if !kind.is_empty() && !kind.eq_ignore_ascii_case("anthropic") {
        return Err(format!("the connection {connection} is of kind {kind}; Clarity's agent speaks to Anthropic models and Microsoft Foundry for now"));
    }
    let api_key = field("apikey")?;
    if api_key.is_empty() {
        return Err(format!("the connection {connection} has no apikey"));
    }
    let base_url = Some(field("baseUrl")?).filter(|u| !u.is_empty()).unwrap_or_else(|| "https://api.anthropic.com".into());
    let model = field("model")?;
    if model.is_empty() {
        return Err(format!("the connection {connection} names no model"));
    }
    clarity_agent::anthropic::Anthropic::new(&base_url, clarity_agent::anthropic::Auth::Key(api_key), &model, max_tokens, cache).map(Some)
}

/// The budget of a turn when the connection names none: tokens at full
/// price (cache reads are not counted).
const DEFAULT_BUDGET: u64 = 200_000;

/// A connection's `turnBudget`: tokens like `250000`, `80k` or `1.5M`, the
/// default when empty, or `none` for no budget.
fn turn_budget(text: &str) -> Result<Option<u64>, String> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(Some(DEFAULT_BUDGET));
    }
    if text.eq_ignore_ascii_case("none") {
        return Ok(None);
    }
    let (number, scale) = match text.chars().last() {
        Some('k' | 'K') => (&text[..text.len() - 1], 1_000.0),
        Some('m' | 'M') => (&text[..text.len() - 1], 1_000_000.0),
        _ => (text, 1.0),
    };
    number
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|n| *n > 0.0)
        .map(|n| Some((n * scale).round() as u64))
        .ok_or_else(|| format!("turnBudget {text} is no number of tokens; write it like 200k, 250000 or none"))
}

/// Starts a turn: the person's words go to the model, which works on the
/// open project. Returns at once; the turn reports through events.
#[tauri::command]
pub fn agent_send(text: String, connection: String, runs: Runs, app: AppHandle, agent: State<AgentState>) -> Result<(), String> {
    // A connection of kind codex or copilot is the person's own agent program, signed in its own way (docs/ACP-PLAN.md).
    if let Some(vendor) = crate::acp::vendor_of(&app, &connection) {
        return crate::acp::send(app, connection, vendor, text);
    }
    // The key is read before the turn starts, so a locked vault asks for its password first.
    let Some(mut model) = model_of(&app, &connection)? else {
        // A Microsoft sign-in is missing: the panel offers Sign in, as for Codex and Copilot.
        let _ = app.emit(AGENT_EVENT, &Event::Failed { message: format!("The connection {connection} is not signed in to Microsoft. Click Sign in, then ask again.") });
        let _ = app.emit("agent-ended", &AgentEnded { opened: None, shown: None, diagram: None, changed: false, sign_in: true });
        return Ok(());
    };
    let budget = {
        let vault = app.state::<super::vault::VaultState>();
        turn_budget(&super::vault::resolve(&vault, &connection, "turnBudget")?).map_err(|err| format!("the connection {connection}: {err}"))?
    };
    if agent.running.swap(true, Ordering::SeqCst) {
        return Err("the agent is still working; stop it first".into());
    }
    agent.stop.store(false, Ordering::SeqCst);
    let stop = agent.stop.clone();
    let project = open_project_of(&app.state::<WatchState>()).ok();
    // New projects go to the projects folder of Clarity settings, else beside the open one.
    let projects_folder = clarity_project::Folders::current()
        .projects
        .or_else(|| project.as_ref().and_then(|(file, _)| file.parent().map(Path::to_path_buf)))
        .unwrap_or_else(|| std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default().join("Clarity"));
    std::thread::spawn(move || {
        let state = app.state::<AgentState>();
        let mut tools = IdeTools::new(project, projects_folder);
        tools.execute = Some(Box::new(AppExecute { app: app.clone(), stop: stop.clone() }));
        tools.runs = runs;
        if let Ok(output) = app.state::<crate::build::BuildState>().output.held() {
            tools.output = output.text();
        }
        let mut conversation = state.conversation.held().map(|c| c.clone()).unwrap_or_default();
        let agent = Agent { system: instructions(), max_steps: MAX_STEPS, budget };
        let _ = agent.run(&mut model, &mut tools, &mut conversation, &text, &stop, &mut |event| {
            let _ = app.emit(AGENT_EVENT, &event);
        });
        if let Ok(mut kept) = state.conversation.held() {
            *kept = conversation;
        }
        let changes: Vec<FileChange> =
            merged(&tools.core.written).into_iter().map(|w| FileChange { file: w.file, before: w.before, after: w.after }).collect();
        let changed = !changes.is_empty();
        // A project made during the turn opens with a history of its own.
        if tools.core.made.is_none() && changed {
            let _ = record(&app.state::<HistoryState>(), changes);
        }
        state.running.store(false, Ordering::SeqCst);
        let ended = AgentEnded { opened: tools.core.made.map(|f| f.display().to_string()), shown: tools.shown, diagram: tools.shown_diagram, changed, sign_in: false };
        let _ = app.emit("agent-ended", &ended);
    });
    Ok(())
}

/// Stops the turn after the step it is in.
#[tauri::command]
pub fn agent_stop(app: AppHandle, agent: State<AgentState>) {
    agent.stop.store(true, Ordering::SeqCst);
    crate::acp::cancel(&app);
}

/// Forgets the conversation, to start a new one.
#[tauri::command]
pub fn agent_forget(app: AppHandle, agent: State<AgentState>) -> Result<(), String> {
    if agent.running.load(Ordering::SeqCst) {
        return Err("the agent is still working; stop it first".into());
    }
    // An outside agent's program stops; the next message starts a new conversation with it.
    crate::acp::forget(&app);
    agent.conversation.held().map_err(|_| "the conversation is not available".to_string())?.clear();
    Ok(())
}

/// Silences the unused-event warning of the TypeScript export.
#[allow(dead_code)]
fn exported(_: Event) {}

#[cfg(test)]
mod tests {
    use super::*;

    /// A project in a scratch folder: the shop example's project file and modules.
    fn project(name: &str) -> (PathBuf, PathBuf, PathBuf) {
        let root = std::env::temp_dir().join(format!("clarity-agent-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let file = clarity_tools::new_project::write_new_project(&root.join("todo"), clarity_project::starter("Todo", "todo").stack).unwrap();
        let dir = clarity_project::module_dir(&file);
        (root, file, dir)
    }

    /// The agent executes nothing by itself: its tools read, write project
    /// files and check them. Only `build` runs something, and it asks the
    /// person first. Any other tool that runs something needs this list
    /// changed on purpose.
    #[test]
    fn only_the_build_tool_executes_and_it_asks_first() {
        let tools = IdeTools::new(None, std::env::temp_dir());
        let mut names: Vec<String> = tools.definitions().into_iter().map(|t| t.name).collect();
        names.sort();
        let allowed = [
            // Reading.
            "api", "build_output", "check", "design", "diagram_proposals", "diagram_view", "manual", "missing_translations", "parts", "project_overview", "read_file",
            "read_generated", "reference",
            // Writing project files, checked at once and undone with one Undo.
            "apply_diagram", "migrate", "new_project", "pick_api", "rename", "write_diagram", "write_file",
            // Writing the generated source tree, as `clarity gen` does; it builds nothing.
            "generate",
            // Showing a window in Preview or a diagram, which draw without running anything.
            "show_diagram", "show_window",
            // Reading the database's backups.
            "list_backups",
            // Backing up and restoring the database, only after the person allows it.
            "backup_data", "restore_backup",
            // Running or building, only after the person allows it.
            "build",
            // The structural tools are added below: each makes one checked designer edit and runs nothing.
        ];
        let mut allowed: Vec<String> = allowed.iter().map(|n| n.to_string()).chain(clarity_tools::structural::definitions().into_iter().map(|t| t.name)).collect();
        allowed.sort();
        assert_eq!(names, allowed);
    }

    /// Stands in for the person and the toolchain.
    struct Asked {
        allow: bool,
        asked: Vec<String>,
        built: Vec<bool>,
    }

    /// A project with a custom part in Rust, its crate written by Clarity's starter.
    fn with_part(name: &str) -> (PathBuf, PathBuf, PathBuf, IdeTools) {
        let (root, file, dir) = project(name);
        let mut tools = IdeTools::new(Some((file.clone(), dir.clone())), root.clone());
        let declared = tools.call("write_file", &json!({"path": "arena.clarity", "text": "module arena\n\npart Arena\n    crate \"parts/Arena\"\n    hears restart\n    size 4 x 3\n"}));
        assert!(!declared.is_error, "{}", declared.text);
        let project = clarity_project::load_modules(&dir).unwrap();
        clarity_gen::parts::write_part_folders(&dir, &project).unwrap();
        (root, file, dir, tools)
    }

    #[test]
    fn the_build_tool_builds_a_part_on_its_own_after_asking() {
        let (root, _, _, mut tools) = with_part("part-build");
        let person = std::rc::Rc::new(std::cell::RefCell::new(Asked { allow: true, asked: vec![], built: vec![] }));
        tools.execute = Some(Box::new(person.clone()));
        let built = tools.call("build", &json!({"part": "Arena"}));
        assert!(!built.is_error, "{}", built.text);
        assert_eq!(person.borrow().asked, ["Build part Arena on its own, in parts/Arena"]);
        assert!(tools.call("build", &json!({"part": "Nothing"})).is_error);
        std::fs::remove_dir_all(&root).unwrap();
    }

    impl Execute for std::rc::Rc<std::cell::RefCell<Asked>> {
        fn ask(&mut self, what: &str) -> bool {
            let mut asked = self.borrow_mut();
            asked.asked.push(what.to_string());
            asked.allow
        }

        fn build(&mut self, _file: &Path, run: bool) -> Result<String, String> {
            self.borrow_mut().built.push(run);
            Ok("The last build succeeded: running. Its output (1 line):\ndotnet build".into())
        }

        fn build_part(&mut self, _dir: &Path, name: &str) -> Result<String, String> {
            Ok(format!("The last build succeeded: {name} built."))
        }
    }

    #[test]
    fn the_build_tool_asks_the_person_first() {
        let (root, file, dir) = project("build");
        let person = std::rc::Rc::new(std::cell::RefCell::new(Asked { allow: false, asked: vec![], built: vec![] }));
        let mut tools = IdeTools::new(Some((file.clone(), dir.clone())), root.clone());
        // Without a way to ask, nothing runs.
        assert!(tools.call("build", &json!({"run": true})).is_error);

        tools.execute = Some(Box::new(person.clone()));
        let denied = tools.call("build", &json!({"run": true}));
        assert!(!denied.is_error && denied.text.contains("denied"), "{}", denied.text);
        assert_eq!(person.borrow().asked, ["Run the application: generate it, build it and start it"]);
        assert!(person.borrow().built.is_empty(), "a denied build does not run");

        person.borrow_mut().allow = true;
        let built = tools.call("build", &json!({"run": false}));
        assert!(!built.is_error && built.text.ends_with("dotnet build"), "{}", built.text);
        assert_eq!(person.borrow().asked[1], "Build the application: generate it and build it");
        assert_eq!(person.borrow().built, [false]);
        std::fs::remove_dir_all(&root).unwrap();
    }

    /// Backing up and restoring a database always ask the person first, whatever the settings say of building.
    #[test]
    fn backing_up_and_restoring_ask_the_person_first() {
        let (root, file, dir) = project("backups");
        let (_, path) = clarity_project::database_to_back_up(&dir, None).unwrap();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let db = rusqlite::Connection::open(&path).unwrap();
        db.execute_batch("CREATE TABLE IF NOT EXISTS item (id INTEGER PRIMARY KEY);").unwrap();
        drop(db);
        let person = std::rc::Rc::new(std::cell::RefCell::new(Asked { allow: false, asked: vec![], built: vec![] }));
        let mut tools = IdeTools::new(Some((file.clone(), dir.clone())), root.clone());
        tools.runs = Runs::Allow;
        assert!(tools.call("backup_data", &json!({})).is_error, "without a way to ask, nothing is backed up");
        tools.execute = Some(Box::new(person.clone()));
        let denied = tools.call("backup_data", &json!({}));
        assert!(denied.text.contains("denied"), "{}", denied.text);
        assert!(tools.call("list_backups", &json!({})).text.contains("no backups"));
        person.borrow_mut().allow = true;
        let taken = tools.call("backup_data", &json!({}));
        assert!(!taken.is_error && taken.text.starts_with("Backed up"), "{}", taken.text);
        let listed = tools.call("list_backups", &json!({})).text;
        let backup = listed.split_whitespace().next().unwrap().to_string();
        let restored = tools.call("restore_backup", &json!({"file": backup}));
        assert!(!restored.is_error && restored.text.starts_with("Restored"), "{}", restored.text);
        let asked = person.borrow().asked.clone();
        assert_eq!(asked.len(), 3, "{asked:?}");
        assert!(asked[2].starts_with("Restore the database of main from "), "{asked:?}");
        std::fs::remove_dir_all(&root).unwrap();
    }

    /// Clarity's settings say whether the agent asks, may run without asking, or never runs.
    #[test]
    fn the_settings_say_whether_the_agent_asks() {
        let (root, file, dir) = project("runs");
        let person = std::rc::Rc::new(std::cell::RefCell::new(Asked { allow: false, asked: vec![], built: vec![] }));
        let mut tools = IdeTools::new(Some((file.clone(), dir.clone())), root.clone());
        tools.execute = Some(Box::new(person.clone()));

        tools.runs = Runs::Allow;
        assert!(!tools.call("build", &json!({"run": true})).is_error);
        assert!(person.borrow().asked.is_empty(), "allowed in the settings: no question");
        assert_eq!(person.borrow().built, [true]);

        // Never: the agent is not even offered the tool, and a call does nothing.
        tools.runs = Runs::Never;
        assert!(!tools.definitions().iter().any(|t| t.name == "build"));
        let refused = tools.call("build", &json!({"run": true}));
        assert!(refused.is_error && refused.text.contains("settings"), "{}", refused.text);
        assert_eq!(person.borrow().built, [true]);
        assert_eq!(serde_json::from_value::<Runs>(json!("ask")).unwrap(), Runs::Ask);
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn the_agent_reads_the_build_output() {
        let mut tools = IdeTools::new(None, std::env::temp_dir());
        assert_eq!(tools.call("build_output", &json!({})).text, "Nothing was built or run since Clarity started.");
        tools.output = "The last build failed: the build failed. Its output (1 line):\nerror CS0103".into();
        let read = tools.call("build_output", &json!({}));
        assert!(!read.is_error && read.text.ends_with("error CS0103"), "{}", read.text);
        // The model is told to read it rather than ask for a paste.
        assert!(instructions().contains("build_output"));
    }

    #[test]
    fn a_turn_budget_reads_as_tokens() {
        assert_eq!(turn_budget(""), Ok(Some(200_000)));
        assert_eq!(turn_budget("250000"), Ok(Some(250_000)));
        assert_eq!(turn_budget(" 80k "), Ok(Some(80_000)));
        assert_eq!(turn_budget("1.5M"), Ok(Some(1_500_000)));
        assert_eq!(turn_budget("none"), Ok(None));
        assert!(turn_budget("lots").unwrap_err().contains("lots"));
    }

    /// The IDE's agent shows what it made: a window in Preview, a diagram drawn.
    #[test]
    fn the_agent_shows_a_window_and_a_diagram() {
        let (root, file, dir) = project("show");
        let mut tools = IdeTools::new(Some((file.clone(), dir.clone())), root.clone());
        tools.call("show_window", &json!({"name": "Main"}));
        assert_eq!(tools.shown.as_deref(), Some("Main"));
        assert!(!tools.call("write_diagram", &json!({"name": "billing", "text": "erDiagram\n"})).is_error);
        tools.call("show_diagram", &json!({"name": "billing"}));
        assert_eq!(tools.shown_diagram.as_deref(), Some("billing.mermaid"));
        assert!(tools.call("show_diagram", &json!({"name": "missing"})).is_error);
        std::fs::remove_dir_all(&root).unwrap();
    }

    /// The shared instructions with the IDE's own lines stay small enough for every message.
    #[test]
    fn the_ide_s_instructions_name_its_own_tools_and_stay_small() {
        let told = instructions();
        for tool in ["build tool", "backup_data", "show_diagram", "show_window", "build_output"] {
            assert!(told.contains(tool), "the IDE's instructions lack {tool}");
        }
        for dash in ['\u{2014}', '\u{2013}'] {
            assert!(!told.contains(dash), "the instructions hold {dash:?}");
        }
        assert!(told.len() < 16_000, "the instructions are {} bytes", told.len());
    }
}
