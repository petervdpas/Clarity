//! The agent panel: a conversation with a model that knows Clarity and works
//! on the open project through tools. It reads the project, writes modules
//! and the project file, checks them, makes a new project, and shows a
//! window. Its changes are written at once and checked; one Undo takes a
//! whole turn back. The model is reached through a connection of Clarity's
//! vault, so its key never sits in a file.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use clarity_agent::{Agent, Event, Message, ToolDef, ToolResult, Tools};
use clarity_project::Stack;
use clarity_spec::Severity;
use serde::Serialize;
use serde_json::{Value, json};
use tauri::{AppHandle, Emitter, Manager, State};
use ts_rs::TS;

use super::{FileChange, HistoryState, WatchState, is_spec_file, open_project_of, record, write_atomically};

/// The language reference, which the model reads before every turn.
const LANGUAGE: &str = include_str!("../../../docs/LANGUAGE.md");
/// Clarity's design document: its aims, its stacks and its roadmap. Too long
/// for every message, so the model reads the sections it needs.
const DESIGN: &str = include_str!("../../../docs/DESIGN.md");

/// The headings (`##` and `###`) of a Markdown document with their levels
/// and lines, leaving out what is in code blocks.
fn headings(doc: &str) -> Vec<(usize, usize, &str)> {
    let mut code = false;
    let mut found = Vec::new();
    for (i, line) in doc.lines().enumerate() {
        if line.trim_start().starts_with("```") {
            code = !code;
        } else if !code {
            let level = line.chars().take_while(|c| *c == '#').count();
            if (2..=3).contains(&level) && line[level..].starts_with(' ') {
                found.push((i, level, line));
            }
        }
    }
    found
}

/// The design tool: without `wanted`, the contents; with it, the section
/// whose number or title it is (or whose title holds it), up to the next
/// section of its level.
fn design_section(doc: &str, wanted: &str) -> Result<String, String> {
    let all = headings(doc);
    let wanted = wanted.trim().to_lowercase();
    if wanted.is_empty() {
        let lines: Vec<&str> = all.iter().map(|(_, _, line)| *line).collect();
        return Ok(format!("The sections of Clarity's design document; ask for one by its number or title.\n\n{}", lines.join("\n")));
    }
    let title = |line: &str| line.trim_start_matches('#').trim().to_lowercase();
    let number = |line: &str| title(line).split_whitespace().next().unwrap_or_default().trim_end_matches('.').to_string();
    let found = all
        .iter()
        .position(|(_, _, line)| number(line) == wanted)
        .or_else(|| all.iter().position(|(_, _, line)| title(line).contains(&wanted)))
        .ok_or_else(|| format!("the design document has no section '{wanted}'; call design without a section for the contents"))?;
    let (start, level, _) = all[found];
    let end = all[found + 1..].iter().find(|(_, l, _)| *l <= level).map_or(usize::MAX, |(i, _, _)| *i);
    let lines: Vec<&str> = doc.lines().skip(start).take(end.saturating_sub(start)).collect();
    Ok(lines.join("\n").trim_end().to_string())
}

/// How many steps (model answers) one turn may take.
const MAX_STEPS: usize = 40;

/// What the model is told before the conversation.
fn instructions() -> String {
    format!(
        "You are the agent inside Clarity, a development environment where applications are written in the Clarity \
language and designed visually; Clarity transpiles them into real applications (Rust, Go, C#, C and C++ stacks). You \
work on the project that is open in Clarity, through your tools, while the person watches the designer follow your \
changes.

How you work:
- Start a task by looking at the project (project_overview), and read a file before you change it.
- A project is one project file (read and write it as the path `project`) and modules: `.clarity` files in the \
project's folder, written as paths like `todo.clarity`.
- Write whole files with write_file. Every write is checked at once: fix every error it reports before you go on, \
and finish only when check reports no errors.
- Follow the language reference below exactly; do not invent syntax. When unsure of a part's properties, use the \
parts tool.
- Lay windows out to use the full width of their grid: inputs and tables reach the last column, labels sit left of \
their inputs, buttons sit together at the bottom right.
- After adding or changing entities, write a migration step with migrate, so the database follows.
- When a window is done, show it with show_window so the person sees it.
- When asked what Clarity can do, which stacks support something, or what is planned, read the design document with \
the design tool (its Stacks and Roadmap sections) rather than guess; never promise what the roadmap says is not built.
- Say briefly what you did when you finish. Write plain, short sentences.

The Clarity language reference:

{LANGUAGE}"
    )
}

/// One change of one file during a turn: its text before (none for a new file) and after.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Written {
    pub file: PathBuf,
    pub before: Option<String>,
    pub after: String,
}

/// The tools, on one project: the project file and its folder of modules.
pub struct ProjectTools {
    project: Option<(PathBuf, PathBuf)>,
    /// What was written during the turn, oldest first.
    pub written: Vec<Written>,
    /// A project made during the turn, to open when it ends.
    pub made: Option<PathBuf>,
    /// The window to show in Preview, when a tool asked for one.
    pub shown: Option<String>,
    /// The folder new projects go in by default.
    projects_folder: PathBuf,
}

impl ProjectTools {
    pub fn new(project: Option<(PathBuf, PathBuf)>, projects_folder: PathBuf) -> ProjectTools {
        ProjectTools { project, written: Vec::new(), made: None, shown: None, projects_folder }
    }

    fn open(&self) -> Result<&(PathBuf, PathBuf), String> {
        self.project.as_ref().ok_or_else(|| "no project is open; make one with new_project".to_string())
    }

    /// The file a tool's path names: `project`, or a `.clarity` file in the project's folder.
    fn file_of(&self, path: &str) -> Result<PathBuf, String> {
        let (file, dir) = self.open()?;
        let path = path.trim();
        if path == "project" {
            return Ok(file.clone());
        }
        let relative = Path::new(path);
        let fine = !path.is_empty()
            && relative.is_relative()
            && relative.components().all(|c| matches!(c, std::path::Component::Normal(_)))
            && is_spec_file(relative);
        if !fine {
            return Err(format!("'{path}' is not a module of the project: use a path like todo.clarity inside the project's folder, or `project`"));
        }
        Ok(dir.join(relative))
    }

    /// The project's problems, as `file:line:col: error: message` lines.
    fn problems(&self) -> Result<String, String> {
        let (_, dir) = self.open()?;
        let project = clarity_project::load_modules(dir).map_err(|err| format!("cannot read the project: {err}"))?;
        let lines: Vec<String> = project
            .diagnostics
            .iter()
            .take(60)
            .map(|d| {
                let file = Path::new(&d.path).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "project".into());
                let severity = if d.severity == Severity::Error { "error" } else { "warning" };
                format!("{file}:{}:{}: {severity}: {}", d.span.line, d.span.col, d.message)
            })
            .collect();
        let errors = project.diagnostics.iter().filter(|d| d.severity == Severity::Error).count();
        let warnings = project.diagnostics.len() - errors;
        Ok(match (errors, warnings) {
            (0, 0) => "No problems.".into(),
            (0, n) => format!("No errors; {n} warning(s):\n{}", lines.join("\n")),
            (e, _) => format!("{e} error(s):\n{}", lines.join("\n")),
        })
    }

    fn overview(&self) -> Result<String, String> {
        let (file, dir) = self.open()?;
        let mut out = format!("Project file ({}):\n{}\n\nModules in {}:\n", file.display(), std::fs::read_to_string(file).unwrap_or_default(), dir.display());
        let mut modules: Vec<PathBuf> = Vec::new();
        collect(dir, dir, &mut modules);
        modules.sort();
        for m in &modules {
            out.push_str(&format!("- {}\n", m.display()));
        }
        out.push_str(&format!("\nProblems:\n{}", self.problems()?));
        Ok(out)
    }

    fn read(&self, path: &str) -> Result<String, String> {
        let file = self.file_of(path)?;
        std::fs::read_to_string(&file).map_err(|_| format!("{path} does not exist yet"))
    }

    fn write(&mut self, path: &str, text: &str) -> Result<String, String> {
        let file = self.file_of(path)?;
        let before = std::fs::read_to_string(&file).ok();
        if let Some(folder) = file.parent() {
            std::fs::create_dir_all(folder).map_err(|err| format!("cannot make {}: {err}", folder.display()))?;
        }
        let text = if text.ends_with('\n') { text.to_string() } else { format!("{text}\n") };
        write_atomically(&file, &text)?;
        self.written.push(Written { file, before, after: text });
        Ok(format!("Wrote {path}. {}", self.problems()?))
    }

    /// Writes what the data dictionary changed since the last migration step
    /// as the next step, as the Migrate button does.
    fn migrate(&mut self, input: &Value) -> Result<String, String> {
        let (file, dir) = self.open()?.clone();
        let project = clarity_project::load_modules(&dir).map_err(|err| format!("cannot read the project: {err}"))?;
        if project.diagnostics.iter().any(|d| d.severity == Severity::Error) {
            return Err("fix the errors first: a step is written from a project that checks clean".into());
        }
        let first = clarity_project::load(&file).settings.and_then(|s| s.datasources.first().map(|d| d.name.clone())).unwrap_or_else(|| "main".into());
        let source = input["source"].as_str().filter(|s| !s.trim().is_empty()).map(String::from).unwrap_or(first);
        let renames = clarity_spec::migrate::proposed_renames(&project, &source);
        let ops = clarity_spec::migrate::pending(&project, &source, &renames);
        if ops.is_empty() {
            return Ok(format!("Nothing to migrate in {source}: every change is in a step already."));
        }
        let title = input["title"].as_str().filter(|t| !t.trim().is_empty()).unwrap_or("Changes by the agent");
        let path = clarity_spec::write_step(&dir, &source, title.trim(), &ops).map_err(|err| format!("cannot write the step: {err}"))?;
        let text = std::fs::read_to_string(&path).map_err(|err| format!("cannot read {}: {err}", path.display()))?;
        self.written.push(Written { file: path.clone(), before: None, after: text.clone() });
        Ok(format!("Wrote the step {}:\n{text}\n{}", path.display(), self.problems()?))
    }

    fn new_project(&mut self, input: &Value) -> Result<String, String> {
        let name = input["name"].as_str().unwrap_or_default().trim();
        if name.is_empty() {
            return Err("a project needs a name, like todo".into());
        }
        let folder = input["folder"].as_str().map(PathBuf::from).unwrap_or_else(|| self.projects_folder.clone());
        let stack: Stack = match input.get("stack") {
            Some(stack) if !stack.is_null() => serde_json::from_value(stack.clone()).map_err(|err| format!("the stack does not fit: {err}"))?,
            _ => clarity_project::starter("App", "app").stack,
        };
        let file = super::write_new_project(&folder.join(name), stack)?;
        let dir = clarity_project::module_dir(&file);
        self.project = Some((file.clone(), dir));
        self.made = Some(file.clone());
        Ok(format!("Made the project {}; it is the open project now.\n\n{}", file.display(), self.overview()?))
    }
}

/// Every `.clarity` file under `folder`, relative to `root`.
fn collect(root: &Path, folder: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(folder) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(root, &path, out);
        } else if is_spec_file(&path) {
            out.push(path.strip_prefix(root).unwrap_or(&path).to_path_buf());
        }
    }
}

fn schema(properties: Value, required: &[&str]) -> Value {
    json!({ "type": "object", "properties": properties, "required": required })
}

impl Tools for ProjectTools {
    fn definitions(&self) -> Vec<ToolDef> {
        let tool = |name: &str, description: &str, input_schema: Value| ToolDef { name: name.into(), description: description.into(), input_schema };
        vec![
            tool("project_overview", "The open project: its project file, its modules and its problems.", schema(json!({}), &[])),
            tool(
                "read_file",
                "Reads a module (a path like todo.clarity, relative to the project's folder) or the project file (`project`).",
                schema(json!({ "path": { "type": "string" } }), &["path"]),
            ),
            tool(
                "write_file",
                "Writes a whole module (a path like todo.clarity) or the project file (`project`), new or replacing it. Gives the project's problems after the write.",
                schema(json!({ "path": { "type": "string" }, "text": { "type": "string", "description": "The whole file." } }), &["path", "text"]),
            ),
            tool("check", "Checks the whole project and gives its problems.", schema(json!({}), &[])),
            tool(
                "migrate",
                "Writes the changes of the data dictionary since the last migration step as the next step, so the database follows. Use it after adding or changing entities, when check warns that they are not in a migration yet.",
                schema(json!({ "title": { "type": "string", "description": "What the step does, like \"Tasks\"." }, "source": { "type": "string", "description": "The data source; the first one by default." } }), &[]),
            ),
            tool("parts", "The parts a window can hold, with their properties, sizes and events, as JSON.", schema(json!({}), &[])),
            tool(
                "design",
                "Reads Clarity's design document: what Clarity aims at, which stacks there are and what each supports, and the roadmap of what is done and what is planned. Without a section it gives the contents.",
                schema(json!({ "section": { "type": "string", "description": "A section's number (like 19 or 1.2) or a word of its title (like roadmap)." } }), &[]),
            ),
            tool(
                "new_project",
                "Makes a new project with its first module, an app and a main window, and opens it. Use it when the person asks for a new application.",
                schema(
                    json!({
                        "name": { "type": "string", "description": "The project's file name, like todo." },
                        "folder": { "type": "string", "description": "The folder to make it in; the folder of the open project by default." },
                        "stack": { "type": "object", "description": "Optional: { backend: rust|go|csharp|c|cpp, ui: web|avalonia, targets: [desktop] }." }
                    }),
                    &["name"],
                ),
            ),
            tool("show_window", "Shows a window of the project in Clarity's Preview.", schema(json!({ "name": { "type": "string" } }), &["name"])),
        ]
    }

    fn call(&mut self, name: &str, input: &Value) -> ToolResult {
        let text = |key: &str| input[key].as_str().unwrap_or_default().to_string();
        let result = match name {
            "project_overview" => self.overview(),
            "read_file" => self.read(&text("path")),
            "write_file" => self.write(&text("path"), &text("text")),
            "check" => self.problems(),
            "parts" => serde_json::to_string(&clarity_spec::widgets::catalog()).map_err(|err| err.to_string()),
            "design" => design_section(DESIGN, &text("section")),
            "new_project" => self.new_project(input),
            "migrate" => self.migrate(input),
            "show_window" => {
                self.shown = Some(text("name"));
                Ok(format!("Showing {} in Preview.", text("name")))
            }
            other => Err(format!("there is no tool {other}")),
        };
        match result {
            Ok(text) => ToolResult { text, is_error: false },
            Err(text) => ToolResult { text, is_error: true },
        }
    }
}

/// The changes of a turn as one change for Undo: each file once, from its
/// text before the turn to its text after.
pub fn merged(written: &[Written]) -> Vec<Written> {
    let mut out: Vec<Written> = Vec::new();
    for w in written {
        match out.iter_mut().find(|o| o.file == w.file) {
            Some(o) => o.after = w.after.clone(),
            None => out.push(w.clone()),
        }
    }
    out
}

/// The conversation, and whether a turn runs.
#[derive(Default)]
pub struct AgentState {
    conversation: Mutex<Vec<Message>>,
    running: AtomicBool,
    stop: Arc<AtomicBool>,
}

/// Sent to the screen while a turn runs, and when it ends.
const AGENT_EVENT: &str = "agent-event";

/// What the screen does when a turn ends.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct AgentEnded {
    /// The project made during the turn, to open.
    pub opened: Option<String>,
    /// The window to show in Preview.
    pub shown: Option<String>,
    /// Whether files changed, so Undo can take the turn back.
    pub changed: bool,
}

/// The model of a connection: its kind, address, model, key and length limit.
fn model_of(app: &AppHandle, connection: &str) -> Result<clarity_agent::anthropic::Anthropic, String> {
    let vault = app.state::<super::vault::VaultState>();
    let field = |key: &str| super::vault::resolve(&vault, connection, key);
    let kind = field("kind")?;
    if !kind.is_empty() && !kind.eq_ignore_ascii_case("anthropic") {
        return Err(format!("the connection {connection} is of kind {kind}; Clarity's agent speaks to Anthropic models for now"));
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
    let max_tokens = field("maxTokens")?.trim().parse().unwrap_or(16384);
    clarity_agent::anthropic::Anthropic::new(&base_url, &api_key, &model, max_tokens)
}

/// Starts a turn: the person's words go to the model, which works on the
/// open project. Returns at once; the turn reports through events.
#[tauri::command]
pub fn agent_send(text: String, connection: String, app: AppHandle, agent: State<AgentState>) -> Result<(), String> {
    // The key is read before the turn starts, so a locked vault asks for its password first.
    let mut model = model_of(&app, &connection)?;
    if agent.running.swap(true, Ordering::SeqCst) {
        return Err("the agent is still working; stop it first".into());
    }
    agent.stop.store(false, Ordering::SeqCst);
    let stop = agent.stop.clone();
    let project = open_project_of(&app.state::<WatchState>()).ok();
    let projects_folder = project
        .as_ref()
        .and_then(|(file, _)| file.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default().join("Clarity"));
    std::thread::spawn(move || {
        let state = app.state::<AgentState>();
        let mut tools = ProjectTools::new(project, projects_folder);
        let mut conversation = state.conversation.lock().map(|c| c.clone()).unwrap_or_default();
        let agent = Agent { system: instructions(), max_steps: MAX_STEPS };
        let _ = agent.run(&mut model, &mut tools, &mut conversation, &text, &stop, &mut |event| {
            let _ = app.emit(AGENT_EVENT, &event);
        });
        if let Ok(mut kept) = state.conversation.lock() {
            *kept = conversation;
        }
        let changes: Vec<FileChange> =
            merged(&tools.written).into_iter().map(|w| FileChange { file: w.file, before: w.before, after: w.after }).collect();
        let changed = !changes.is_empty();
        // A project made during the turn opens with a history of its own.
        if tools.made.is_none() && changed {
            let _ = record(&app.state::<HistoryState>(), changes);
        }
        state.running.store(false, Ordering::SeqCst);
        let ended = AgentEnded { opened: tools.made.map(|f| f.display().to_string()), shown: tools.shown, changed };
        let _ = app.emit("agent-ended", &ended);
    });
    Ok(())
}

/// Stops the turn after the step it is in.
#[tauri::command]
pub fn agent_stop(agent: State<AgentState>) {
    agent.stop.store(true, Ordering::SeqCst);
}

/// Forgets the conversation, to start a new one.
#[tauri::command]
pub fn agent_forget(agent: State<AgentState>) -> Result<(), String> {
    if agent.running.load(Ordering::SeqCst) {
        return Err("the agent is still working; stop it first".into());
    }
    agent.conversation.lock().map_err(|_| "the conversation is not available".to_string())?.clear();
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
        let file = super::super::write_new_project(&root.join("todo"), clarity_project::starter("Todo", "todo").stack).unwrap();
        let dir = clarity_project::module_dir(&file);
        (root, file, dir)
    }

    #[test]
    fn the_tools_read_write_and_check_the_project() {
        let (root, file, dir) = project("tools");
        let mut tools = ProjectTools::new(Some((file.clone(), dir.clone())), root.clone());
        let overview = tools.call("project_overview", &json!({}));
        assert!(!overview.is_error);
        assert!(overview.text.contains("- todo.clarity") && overview.text.contains("No problems."), "{}", overview.text);

        let read = tools.call("read_file", &json!({"path": "todo.clarity"}));
        assert!(read.text.starts_with("module todo"), "{}", read.text);
        assert!(!tools.call("read_file", &json!({"path": "project"})).is_error);

        // A write is checked at once: the model sees what it broke.
        let broken = tools.call("write_file", &json!({"path": "tasks.clarity", "text": "module tasks\n\nentity Task\n    title: nonsense\n"}));
        assert!(!broken.is_error);
        assert!(broken.text.contains("error(s)") && broken.text.contains("tasks.clarity:"), "{}", broken.text);
        let fixed = tools.call("write_file", &json!({"path": "tasks.clarity", "text": "module tasks\n\nentity Task\n    title: text(120), required"}));
        assert!(fixed.text.contains("No errors") && fixed.text.contains("not in a migration yet"), "{}", fixed.text);
        // A migration step takes the warning away.
        let step = tools.call("migrate", &json!({"title": "Tasks"}));
        assert!(!step.is_error && step.text.contains("add entity Task") && step.text.ends_with("No problems."), "{}", step.text);
        assert_eq!(std::fs::read_to_string(dir.join("tasks.clarity")).unwrap(), "module tasks\n\nentity Task\n    title: text(120), required\n");

        // Paths stay inside the project.
        for path in ["../escape.clarity", "/etc/passwd", "notes.txt", ""] {
            assert!(tools.call("write_file", &json!({"path": path, "text": "x"})).is_error, "{path}");
        }
        assert!(tools.call("format_disk", &json!({})).is_error);

        // The turn is one change for Undo: each file from before the turn to after it.
        let one = merged(&tools.written);
        assert_eq!(one.len(), 2, "the module and the step");
        assert_eq!(one[0].before, None);
        assert!(one[0].after.contains("text(120)"));
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_new_project_becomes_the_open_one() {
        let root = std::env::temp_dir().join(format!("clarity-agent-new-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let mut tools = ProjectTools::new(None, root.clone());
        assert!(tools.call("check", &json!({})).text.contains("no project is open"));
        let made = tools.call("new_project", &json!({"name": "todo"}));
        assert!(!made.is_error, "{}", made.text);
        assert_eq!(tools.made, Some(root.join("todo.clarity")));
        assert!(made.text.contains("No problems."), "{}", made.text);
        assert!(!tools.call("read_file", &json!({"path": "todo.clarity"})).is_error);
        tools.call("show_window", &json!({"name": "Main"}));
        assert_eq!(tools.shown.as_deref(), Some("Main"));
        std::fs::remove_dir_all(&root).unwrap();
    }

    const DOC: &str = "# Design\n\nIntro.\n\n## 1. Purpose\n\nWhy.\n\n### 1.2 Stacks\n\nRust and Go.\n\n```\n## not a heading\n```\n\n### 1.3 Data\n\nSQLite.\n\n## 19. Roadmap\n\nG5 is next.\n\n## 20. Open questions\n\nMany.\n";

    #[test]
    fn the_design_tool_gives_the_contents_and_a_section() {
        // Without a section: the headings, to pick from.
        let contents = design_section(DOC, "").unwrap();
        assert_eq!(contents.lines().filter(|l| l.starts_with('#')).collect::<Vec<_>>(), ["## 1. Purpose", "### 1.2 Stacks", "### 1.3 Data", "## 19. Roadmap", "## 20. Open questions"]);
        // By number or by title: the section up to the next one of its level.
        assert_eq!(design_section(DOC, "19").unwrap(), "## 19. Roadmap\n\nG5 is next.");
        assert_eq!(design_section(DOC, "roadmap").unwrap(), "## 19. Roadmap\n\nG5 is next.");
        assert_eq!(design_section(DOC, "1.2").unwrap(), "### 1.2 Stacks\n\nRust and Go.\n\n```\n## not a heading\n```");
        // A chapter holds its subsections.
        assert!(design_section(DOC, "1").unwrap().contains("### 1.3 Data"));
        assert!(design_section(DOC, "teleport").unwrap_err().contains("no section"));
        // The real document has its roadmap.
        assert!(design_section(DESIGN, "roadmap").unwrap().starts_with("## 19. Roadmap"));
    }

    #[test]
    fn the_model_is_told_the_language() {
        let told = instructions();
        assert!(told.contains("# The Clarity language"));
        assert!(told.contains("write_file"));
        // The design document is there to read, not in every message.
        assert!(told.contains("design tool") && !told.contains("## 19. Roadmap"));
    }
}
