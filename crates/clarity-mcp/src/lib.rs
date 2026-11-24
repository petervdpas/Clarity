//! Clarity as an MCP server (docs/ledger/mcp-tools.md). An agent the person already
//! uses (Claude Code, Codex, Copilot) starts `clarity mcp` and designs and
//! generates through Clarity's own tools, the ones the IDE's agent has.
//!
//! The tools always work on the project open in the Clarity IDE, read again
//! on every call, so opening another project in the IDE moves the agent too.
//! Nothing that builds, runs, touches a database or needs the IDE is offered.

use std::path::{Path, PathBuf};

use clarity_agent::{ToolDef, ToolResult, Tools};
use clarity_project::activity::Call;
use clarity_tools::ProjectTools;
use clarity_tools::turn::{self, Shown};
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, Icon, Implementation, ListToolsResult,
    PaginatedRequestParams, ServerCapabilities, ServerConfig, Tool,
};
use rmcp::service::RequestContext;
use rmcp::{ErrorData, RoleServer, ServerHandler, ServiceExt};
use serde_json::{Value, json};

// The cDdev's DevTools tools (build_piece, build_state, release_state, disk_state,
// remove_folder): only in a server built from Clarity's repository, never
// compiled into a release, as an installed Clarity has no DevTools.
#[cfg(debug_assertions)]
mod devtools;
#[cfg(debug_assertions)]
pub use devtools::{BUILD_PIECE, BUILD_STATE, BUMP_VERSIONS, DISK_STATE, RELEASE_STATE, REMOVE_FOLDER, SWAP_PIECE, WRITE_RELEASE_PLAN};

/// Clarity's icon, as the installers have it (app/src-tauri/icons): a PNG
/// every MCP client takes and the SVG it is made from.
const ICON_PNG: &[u8] = include_bytes!("../../../app/src-tauri/icons/128x128.png");
const ICON_SVG: &[u8] = include_bytes!("../../../app/src-tauri/icons/icon.svg");

/// The icons the server's info carries, embedded as data URIs so a client
/// shows them without the network.
fn icons() -> Vec<Icon> {
    use base64::Engine;
    let embedded = |mime: &str, bytes: &[u8], size: &str| {
        Icon::new(format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes)))
            .with_mime_type(mime)
            .with_sizes(vec![size.to_string()])
    };
    vec![embedded("image/png", ICON_PNG, "128x128"), embedded("image/svg+xml", ICON_SVG, "any")]
}

/// The IDE agent's tools left out here: they build, run or touch the
/// database (build, build_output, the backups), or need the IDE (show_window,
/// show_diagram). Build and run stay with the person.
pub const LEFT_OUT: [&str; 7] = ["build", "build_output", "list_backups", "backup_data", "restore_backup", "show_window", "show_diagram"];

/// The tools that work without an open project: Clarity's documents, its
/// parts, and making a new project.
pub const WITHOUT_PROJECT: [&str; 3] = ["manual", "parts", "new_project"];

/// What every tool answers while the person turned this server off in
/// Clarity's status bar (clarity_project::mcp_access).
pub fn turned_off(server: &str) -> String {
    format!(
        "Clarity's MCP access for {server} is turned off in Clarity: nothing was read or changed. Ask the person to turn \
{server} on in Clarity's status bar (MCP)."
    )
}

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
/// turn in its chat panel (docs/ledger/acp.md): it can show what it made.
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
    /// The folder of the asks to the IDE's DevTools (clarity_project::dev_asks), beside `ide_file`.
    #[cfg(debug_assertions)]
    asks: PathBuf,
    /// How long an ask waits for the IDE to take it, and for what it asked.
    #[cfg(debug_assertions)]
    take_waits: std::time::Duration,
    #[cfg(debug_assertions)]
    build_waits: std::time::Duration,
    /// The turn's file, when the IDE started the server for a turn of its chat panel.
    turn: Option<PathBuf>,
    /// The activity log, beside `ide_file`.
    activity: PathBuf,
    /// The servers' switches (clarity_project::mcp_access), beside `ide_file`.
    switches: PathBuf,
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
        let switches = ide_file.with_file_name(clarity_project::mcp_access::FILE_NAME);
        ClarityServer {
            #[cfg(debug_assertions)]
            asks: ide_file.with_file_name("dev-asks"),
            #[cfg(debug_assertions)]
            take_waits: clarity_project::dev_asks::TAKE_WAITS,
            #[cfg(debug_assertions)]
            build_waits: clarity_project::dev_asks::BUILD_WAITS,
            ide_file,
            turn: None,
            activity,
            switches,
        }
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
        // A server built from Clarity's repository offers the cDdev's DevTools tools; a turn of the IDE's chat panel does not.
        #[cfg(debug_assertions)]
        if self.turn.is_none() {
            tools.extend(devtools::tools());
        }
        tools
    }

    /// The tools offered: the shared ones, less the left-out ones.
    pub fn offered() -> Vec<ToolDef> {
        ProjectTools::new(None, PathBuf::new()).definitions().into_iter().filter(|t| !LEFT_OUT.contains(&t.name.as_str())).collect()
    }

    /// The name it gives: `clarity-cdev` for the developer's build (debug,
    /// with the DevTools tools), `clarity` for the release and for a turn
    /// of the IDE's chat panel.
    pub fn name(&self) -> &'static str {
        #[cfg(debug_assertions)]
        if self.turn.is_none() {
            return devtools::NAME;
        }
        "clarity"
    }

    /// What the agent is told before it starts.
    pub fn instructions(&self) -> String {
        if self.turn.is_some() {
            return clarity_tools::instructions(&TURN_LINES);
        }
        #[cfg(debug_assertions)]
        let lines: Vec<&str> = MCP_LINES.iter().copied().chain([devtools::LINE]).collect();
        #[cfg(not(debug_assertions))]
        let lines: Vec<&str> = MCP_LINES.to_vec();
        clarity_tools::instructions(&lines)
    }

    /// Calls the tool `name` for the MCP client `client`, and adds the call to
    /// the activity log: the tool, the file it names, how it ended, never a file's text.
    pub fn call_from(&self, client: &str, name: &str, input: &Value) -> ToolResult {
        if let Some(refused) = self.switched_off() {
            self.record(client, name, input, &refused);
            return refused;
        }
        #[cfg(debug_assertions)]
        let devtools = if self.turn.is_none() { self.devtools_call(client, name, input) } else { None };
        #[cfg(not(debug_assertions))]
        let devtools: Option<ToolResult> = None;
        let result = devtools.unwrap_or_else(|| self.call(name, input));
        self.record(client, name, input, &result);
        result
    }

    /// The refusal while the person turned this server off in Clarity's status
    /// bar, read again on every call; none while it is on. A turn of the IDE's
    /// chat panel is the person's own agent in Clarity, which the switches do not hold.
    fn switched_off(&self) -> Option<ToolResult> {
        let off = self.turn.is_none() && !clarity_project::mcp_access::is_on_in(&self.switches, self.name());
        off.then(|| ToolResult { text: turned_off(self.name()), is_error: true })
    }

    /// Adds a call to the activity log.
    fn record(&self, client: &str, name: &str, input: &Value, result: &ToolResult) {
        let said = if result.is_error { result.text.clone() } else { format!("{} characters", result.text.chars().count()) };
        let path = input.get("path").and_then(Value::as_str).map(str::to_string);
        let call = Call { client: client.to_string(), tool: name.to_string(), path, ok: !result.is_error, said };
        if let Err(why) = clarity_project::activity::record_in(&self.activity, &call) {
            eprintln!("clarity: {why}");
        }
    }

    /// Calls the tool `name` with `input` on the project open in the IDE now.
    pub fn call(&self, name: &str, input: &Value) -> ToolResult {
        let failed = |text: String| ToolResult { text, is_error: true };
        if let Some(refused) = self.switched_off() {
            return refused;
        }
        let showing = self.turn.is_some() && TURN_TOOLS.contains(&name);
        if LEFT_OUT.contains(&name) && !showing {
            return failed(format!("{name} is not offered here: build, run, backups and showing windows are the person's, in Clarity"));
        }
        let project = clarity_project::ide::open_project_in(&self.ide_file);
        if project.is_none() && !WITHOUT_PROJECT.contains(&name) {
            return failed(NO_PROJECT.to_string());
        }
        let mut tools = ProjectTools::new(project.as_deref().map(open), clarity_tools::new_project::projects_folder(project.as_deref()));
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

/// The window `name` of the open project, to show in Preview (clarity_tools::turn::window).
fn window(tools: &ProjectTools, name: &str) -> Result<(Shown, String), String> {
    let (_, dir) = tools.open()?;
    let shown = turn::window(dir, name)?;
    Ok((shown, format!("Showing {} in Preview when this turn ends.", name.trim())))
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
            .with_server_info(Implementation::new(self.name(), env!("CARGO_PKG_VERSION")).with_icons(icons()))
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
