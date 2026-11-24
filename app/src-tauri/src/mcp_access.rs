//! The MCP item of the status bar: each of Clarity's MCP servers (`clarity`,
//! `clarity-cdev`) with its switch, kept in `mcp.json` in Clarity's own folder
//! (clarity_project::mcp_access). A server reads its switch on every tool
//! call, so turning it holds from the agent's next call.

use std::path::Path;

use serde::Serialize;
use ts_rs::TS;

/// One MCP server and whether it answers an agent's tool calls.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct McpServer {
    /// As the server names itself to the agent: `clarity` or `clarity-cdev`.
    pub name: String,
    pub on: bool,
}

/// Every server with its switch, as the status bar shows them.
#[tauri::command]
pub fn mcp_servers() -> Vec<McpServer> {
    servers_in(&clarity_project::mcp_access::file())
}

/// Turns the server `name` on or off; gives every server as it is then.
#[tauri::command]
pub fn set_mcp_server(name: String, on: bool) -> Result<Vec<McpServer>, String> {
    set_in(&clarity_project::mcp_access::file(), &name, on)
}

fn servers_in(file: &Path) -> Vec<McpServer> {
    clarity_project::mcp_access::states_in(file).into_iter().map(|(name, on)| McpServer { name, on }).collect()
}

fn set_in(file: &Path, name: &str, on: bool) -> Result<Vec<McpServer>, String> {
    clarity_project::mcp_access::set_in(file, name, on)?;
    Ok(servers_in(file))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("clarity-mcp-access-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("mcp.json")
    }

    fn server(name: &str, on: bool) -> McpServer {
        McpServer { name: name.to_string(), on }
    }

    #[test]
    fn both_servers_are_shown_on_without_the_file() {
        assert_eq!(servers_in(&file("none")), vec![server("clarity", true), server("clarity-cdev", true)]);
    }

    #[test]
    fn a_switch_turned_is_what_the_bar_shows_next() {
        let file = file("turn");
        assert_eq!(set_in(&file, "clarity-cdev", false).unwrap(), vec![server("clarity", true), server("clarity-cdev", false)]);
        assert_eq!(servers_in(&file), vec![server("clarity", true), server("clarity-cdev", false)]);
        assert!(set_in(&file, "other", false).is_err());
    }
}
