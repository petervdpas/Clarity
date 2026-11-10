//! Clarity as an MCP server (docs/MCP-PLAN.md). An agent the person already
//! uses (Claude Code, Codex, Copilot) starts `clarity mcp` and designs and
//! generates through Clarity's own tools, the ones the IDE's agent has.
//!
//! The tools always work on the project open in the Clarity IDE, read again
//! on every call, so opening another project in the IDE moves the agent too.
//! Nothing that builds, runs, touches a database or needs the IDE is offered.

use std::path::{Path, PathBuf};

use clarity_agent::{ToolDef, ToolResult, Tools};
use clarity_spec::model::PageKind;
use clarity_project::activity::Call;
use clarity_tools::ProjectTools;
use clarity_tools::turn::{self, Shown};
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, Implementation, ListToolsResult,
    PaginatedRequestParams, ServerCapabilities, ServerConfig, Tool,
};
use rmcp::service::RequestContext;
use rmcp::{ErrorData, RoleServer, ServerHandler, ServiceExt};
use serde_json::{Value, json};

/// The IDE agent's tools left out here: they build, run or touch the
/// database (build, build_output, the backups), or need the IDE (show_window,
/// show_diagram). Build and run stay with the person.
pub const LEFT_OUT: [&str; 7] = ["build", "build_output", "list_backups", "backup_data", "restore_backup", "show_window", "show_diagram"];

/// The tools that work without an open project: Clarity's documents, its
/// parts, and making a new project.
pub const WITHOUT_PROJECT: [&str; 5] = ["manual", "reference", "design", "parts", "new_project"];

/// The tools that still answer when the person turned outside agents off:
/// Clarity's documents and its parts, which touch no project.
pub const WHEN_OFF: [&str; 4] = ["manual", "reference", "design", "parts"];

/// What every other tool answers while outside agents are turned off.
pub const NOT_ALLOWED: &str = "Clarity does not allow outside agents to work on the project now: the person turned it \
off in Clarity's settings (\"Let outside agents work on my project\"). Clarity's manual and references still answer.";

/// What every other tool answers while no project is open in Clarity.
pub const NO_PROJECT: &str = "No project is open in Clarity. Ask the person to open the project in Clarity first; \
these tools work on the project open there.";

/// What the agent is told beyond the shared instructions (crates/clarity-tools).
const MCP_LINES: [&str; 3] = [
    "You reach Clarity through its MCP server. The tools work on the project open in the Clarity IDE; when the person opens \
another project there, the tools follow it.",
    "You cannot build or run the application, take or restore backups, or show windows: ask the person to press Run or \
Build in Clarity, and to tell you what it reports.",
    "A project you make with new_project opens only when the person opens it in Clarity: ask them to open it there.",
];

/// What the agent is told instead when the IDE started the server for a
/// turn in its chat panel (docs/ACP-PLAN.md): it can show what it made.
const TURN_LINES: [&str; 3] = [
    "You reach Clarity through its MCP server, from Clarity's chat panel. The tools work on the project open in Clarity.",
    "You cannot build or run the application or take or restore backups: ask the person to press Run or Build in Clarity, \
and to tell you what it reports. Show a finished window with show_window, and a diagram you wrote with show_diagram; \
Clarity shows them when your turn ends.",
    "A project you make with new_project opens in Clarity when your turn ends; until then the tools work on the project \
open there.",
];

/// The tools a turn of the IDE's chat panel adds: they run nothing, and
/// write what to show into the turn's file, which the IDE reads when the
/// turn ends. Names, descriptions and inputs as the IDE's own agent has them.
pub const TURN_TOOLS: [&str; 2] = ["show_window", "show_diagram"];

/// The server: Clarity's shared tools on the project the IDE has open.
#[derive(Debug, Clone)]
pub struct ClarityServer {
    /// Where the IDE records its open project.
    ide_file: PathBuf,
    /// The turn's file, when the IDE started the server for a turn of its chat panel.
    turn: Option<PathBuf>,
    /// The activity log, beside `ide_file`.
    activity: PathBuf,
}

impl Default for ClarityServer {
    fn default() -> Self {
        ClarityServer::new()
    }
}

impl ClarityServer {
    /// The server on the project the IDE on this computer has open.
    pub fn new() -> ClarityServer {
        ClarityServer::with_ide_file(clarity_project::ide::file())
    }

    /// The server on the project `ide_file` records as open.
    pub fn with_ide_file(ide_file: PathBuf) -> ClarityServer {
        let activity = ide_file.with_file_name(clarity_project::activity::FILE_NAME);
        ClarityServer { ide_file, turn: None, activity }
    }

    /// The same server for a turn of the IDE's chat panel: it also offers
    /// show_window and show_diagram, which write into `file`.
    pub fn for_turn(self, file: PathBuf) -> ClarityServer {
        ClarityServer { turn: Some(file), ..self }
    }

    /// The tools this server offers: with a turn, the two that show.
    pub fn tools(&self) -> Vec<ToolDef> {
        let mut tools = ClarityServer::offered();
        if self.turn.is_some() {
            tools.extend(turn_tools());
        }
        tools
    }

    /// The tools offered: the shared ones, less the left-out ones.
    pub fn offered() -> Vec<ToolDef> {
        ProjectTools::new(None, PathBuf::new()).definitions().into_iter().filter(|t| !LEFT_OUT.contains(&t.name.as_str())).collect()
    }

    /// What the agent is told before it starts.
    pub fn instructions(&self) -> String {
        clarity_tools::instructions(if self.turn.is_some() { &TURN_LINES } else { &MCP_LINES })
    }

    /// Calls the tool `name` for the MCP client `client`, and adds the call to
    /// the activity log: the tool, the file it names, how it ended, never a file's text.
    pub fn call_from(&self, client: &str, name: &str, input: &Value) -> ToolResult {
        let result = self.call(name, input);
        let said = if result.is_error { result.text.clone() } else { format!("{} characters", result.text.chars().count()) };
        let path = input.get("path").and_then(Value::as_str).map(str::to_string);
        let call = Call { client: client.to_string(), tool: name.to_string(), path, ok: !result.is_error, said };
        if let Err(why) = clarity_project::activity::record_in(&self.activity, &call) {
            eprintln!("clarity: {why}");
        }
        result
    }

    /// Calls the tool `name` with `input` on the project open in the IDE now.
    pub fn call(&self, name: &str, input: &Value) -> ToolResult {
        let failed = |text: String| ToolResult { text, is_error: true };
        let showing = self.turn.is_some() && TURN_TOOLS.contains(&name);
        if !WHEN_OFF.contains(&name) && !clarity_project::ide::outside_agents_in(&self.ide_file) {
            return failed(NOT_ALLOWED.to_string());
        }
        if LEFT_OUT.contains(&name) && !showing {
            return failed(format!("{name} is not offered here: build, run, backups and showing windows are the person's, in Clarity"));
        }
        let project = clarity_project::ide::open_project_in(&self.ide_file);
        if project.is_none() && !WITHOUT_PROJECT.contains(&name) {
            return failed(NO_PROJECT.to_string());
        }
        let mut tools = ProjectTools::new(project.as_deref().map(open), projects_folder(project.as_deref()));
        if let (true, Some(turn)) = (showing, &self.turn) {
            let shown = match name {
                "show_window" => window(&tools, input["name"].as_str().unwrap_or_default()),
                _ => diagram(&tools, input["name"].as_str().unwrap_or_default()),
            };
            return match shown.and_then(|(shown, said)| turn::show(turn, &shown).map(|()| said)) {
                Ok(text) => ToolResult { text, is_error: false },
                Err(text) => failed(text),
            };
        }
        let mut result = tools.call(name, input);
        // A project the turn made opens in Clarity when the turn ends, as the IDE's own agent's does.
        if let (Some(turn), Some(made)) = (&self.turn, &tools.made)
            && name == "new_project"
            && !result.is_error
        {
            match turn::show(turn, &Shown::Project(made.clone())) {
                Ok(()) => result.text.push_str("\n\nIt opens in Clarity when this turn ends."),
                Err(why) => return failed(why),
            }
        }
        result
    }
}

/// show_window and show_diagram, as the IDE's agent offers them.
fn turn_tools() -> Vec<ToolDef> {
    let named = json!({ "type": "object", "properties": { "name": { "type": "string" } }, "required": ["name"] });
    let tool = |name: &str, description: &str| ToolDef { name: name.into(), description: description.into(), input_schema: named.clone() };
    vec![
        tool("show_window", "Shows a window of the project in Clarity's Preview."),
        tool("show_diagram", "Opens a written diagram in Clarity, drawn with what it proposes marked."),
    ]
}

/// The window `name` of the open project, to show in Preview; a window or a dialog that is there.
fn window(tools: &ProjectTools, name: &str) -> Result<(Shown, String), String> {
    let (_, dir) = tools.open()?;
    let project = clarity_project::load_modules(dir).map_err(|err| format!("cannot read the project: {err}"))?;
    let windows: Vec<&str> = project
        .files
        .iter()
        .flat_map(|f| &f.pages)
        .filter(|p| matches!(p.kind, PageKind::Window | PageKind::Dialog))
        .map(|p| p.name.name.as_str())
        .collect();
    let name = name.trim();
    if !windows.contains(&name) {
        return Err(format!("the project has no window '{name}'; its windows are: {}", windows.join(", ")));
    }
    Ok((Shown::Window(name.to_string()), format!("Showing {name} in Preview when this turn ends.")))
}

/// The written diagram `name`, to open in Clarity.
fn diagram(tools: &ProjectTools, name: &str) -> Result<(Shown, String), String> {
    let (file, _) = tools.diagram_text(name)?;
    let shown = file.file_name().unwrap_or_default().to_string_lossy().to_string();
    Ok((Shown::Diagram(shown.clone()), format!("Opening diagrams/{shown} in Clarity when this turn ends.")))
}

/// The project file with its folder of modules.
fn open(file: &Path) -> (PathBuf, PathBuf) {
    (file.to_path_buf(), clarity_project::module_dir(file))
}

/// Where a new project goes by default: the projects folder of Clarity
/// settings, else beside the open project, else `Clarity` in the home folder;
/// as the IDE's agent does.
fn projects_folder(project: Option<&Path>) -> PathBuf {
    clarity_project::Folders::current()
        .projects
        .or_else(|| project.and_then(Path::parent).map(Path::to_path_buf))
        .unwrap_or_else(|| std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default().join("Clarity"))
}

/// A tool as MCP describes it.
fn mcp_tool(def: ToolDef) -> Tool {
    let schema = match def.input_schema {
        Value::Object(schema) => schema,
        _ => Default::default(),
    };
    Tool::new(def.name, def.description, schema)
}

impl ServerHandler for ClarityServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("clarity", env!("CARGO_PKG_VERSION")))
            .with_instructions(self.instructions())
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        Ok(ListToolsResult::with_all_items(self.tools().into_iter().map(mcp_tool).collect()))
    }

    fn get_tool(&self, name: &str) -> Option<Tool> {
        self.tools().into_iter().find(|t| t.name == name).map(mcp_tool)
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        // The program that called, by the name it gave when it started.
        let client = context.peer.peer_info().map(|info| info.client_info.name.clone()).unwrap_or_default();
        let input = Value::Object(request.arguments.unwrap_or_default());
        let server = self.clone();
        let name = request.name.to_string();
        // A tool reads and writes files and may generate a whole app: off the protocol's thread.
        let result = tokio::task::spawn_blocking(move || server.call_from(&client, &name, &input))
            .await
            .map_err(|err| ErrorData::internal_error(format!("the tool stopped: {err}"), None))?;
        let content = vec![ContentBlock::text(result.text)];
        Ok(if result.is_error { CallToolResult::error(content) } else { CallToolResult::success(content) }.into())
    }
}

/// Serves MCP over stdin and stdout until the client closes; for a turn of
/// the IDE's chat panel with `turn`, the file show_window and show_diagram write.
pub async fn serve_stdio(turn: Option<PathBuf>) -> Result<(), String> {
    let server = ClarityServer::new();
    let server = match turn {
        Some(file) => server.for_turn(file),
        None => server,
    };
    let running = server.serve(rmcp::transport::stdio()).await.map_err(|err| format!("cannot start the MCP server: {err}"))?;
    running.waiting().await.map_err(|err| format!("the MCP server stopped: {err}"))?;
    Ok(())
}

/// Serves MCP over stdin and stdout until the client closes, on a runtime
/// of its own: what `clarity mcp` calls.
pub fn run(turn: Option<PathBuf>) -> Result<(), String> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|err| format!("cannot start the MCP server: {err}"))?
        .block_on(serve_stdio(turn))
}
