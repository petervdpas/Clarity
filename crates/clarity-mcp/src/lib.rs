//! Clarity as an MCP server (docs/MCP-PLAN.md). An agent the person already
//! uses (Claude Code, Codex, Copilot) starts `clarity mcp` and designs and
//! generates through Clarity's own tools, the ones the IDE's agent has.
//!
//! The tools always work on the project open in the Clarity IDE, read again
//! on every call, so opening another project in the IDE moves the agent too.
//! Nothing that builds, runs, touches a database or needs the IDE is offered.

use std::path::{Path, PathBuf};

use clarity_agent::{ToolDef, ToolResult, Tools};
use clarity_tools::ProjectTools;
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, Implementation, ListToolsResult,
    PaginatedRequestParams, ServerCapabilities, ServerConfig, Tool,
};
use rmcp::service::RequestContext;
use rmcp::{ErrorData, RoleServer, ServerHandler, ServiceExt};
use serde_json::Value;

/// The IDE agent's tools left out here: they build, run or touch the
/// database (build, build_output, the backups), or need the IDE (show_window,
/// show_diagram). Build and run stay with the person.
pub const LEFT_OUT: [&str; 7] = ["build", "build_output", "list_backups", "backup_data", "restore_backup", "show_window", "show_diagram"];

/// The tools that work without an open project: Clarity's documents, its
/// parts, and making a new project.
pub const WITHOUT_PROJECT: [&str; 5] = ["manual", "reference", "design", "parts", "new_project"];

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

/// The server: Clarity's shared tools on the project the IDE has open.
#[derive(Debug, Clone)]
pub struct ClarityServer {
    /// Where the IDE records its open project.
    ide_file: PathBuf,
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
        ClarityServer { ide_file }
    }

    /// The tools offered: the shared ones, less the left-out ones.
    pub fn offered() -> Vec<ToolDef> {
        ProjectTools::new(None, PathBuf::new()).definitions().into_iter().filter(|t| !LEFT_OUT.contains(&t.name.as_str())).collect()
    }

    /// What the agent is told before it starts.
    pub fn instructions() -> String {
        clarity_tools::instructions(&MCP_LINES)
    }

    /// Calls the tool `name` with `input` on the project open in the IDE now.
    pub fn call(&self, name: &str, input: &Value) -> ToolResult {
        let failed = |text: String| ToolResult { text, is_error: true };
        if LEFT_OUT.contains(&name) {
            return failed(format!("{name} is not offered here: build, run, backups and showing windows are the person's, in Clarity"));
        }
        let project = clarity_project::ide::open_project_in(&self.ide_file);
        if project.is_none() && !WITHOUT_PROJECT.contains(&name) {
            return failed(NO_PROJECT.to_string());
        }
        let mut tools = ProjectTools::new(project.as_deref().map(open), projects_folder(project.as_deref()));
        tools.call(name, input)
    }
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
            .with_instructions(ClarityServer::instructions())
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        Ok(ListToolsResult::with_all_items(ClarityServer::offered().into_iter().map(mcp_tool).collect()))
    }

    fn get_tool(&self, name: &str) -> Option<Tool> {
        ClarityServer::offered().into_iter().find(|t| t.name == name).map(mcp_tool)
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        let input = Value::Object(request.arguments.unwrap_or_default());
        let server = self.clone();
        let name = request.name.to_string();
        // A tool reads and writes files and may generate a whole app: off the protocol's thread.
        let result = tokio::task::spawn_blocking(move || server.call(&name, &input))
            .await
            .map_err(|err| ErrorData::internal_error(format!("the tool stopped: {err}"), None))?;
        let content = vec![ContentBlock::text(result.text)];
        Ok(if result.is_error { CallToolResult::error(content) } else { CallToolResult::success(content) }.into())
    }
}

/// Serves MCP over stdin and stdout until the client closes.
pub async fn serve_stdio() -> Result<(), String> {
    let running = ClarityServer::new().serve(rmcp::transport::stdio()).await.map_err(|err| format!("cannot start the MCP server: {err}"))?;
    running.waiting().await.map_err(|err| format!("the MCP server stopped: {err}"))?;
    Ok(())
}

/// Serves MCP over stdin and stdout until the client closes, on a runtime
/// of its own: what `clarity mcp` calls.
pub fn run() -> Result<(), String> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|err| format!("cannot start the MCP server: {err}"))?
        .block_on(serve_stdio())
}
