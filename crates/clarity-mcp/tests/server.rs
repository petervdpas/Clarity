//! Clarity as an MCP server (docs/MCP-PLAN.md): the shared tools minus the
//! ones that execute or need the IDE, always on the project the IDE has open.

use std::path::{Path, PathBuf};

use clarity_agent::Tools;
use clarity_mcp::{ClarityServer, LEFT_OUT, NO_PROJECT};
use clarity_project::ide::{self, Ide};
use rmcp::ServiceExt;
use rmcp::model::CallToolRequestParams;
use serde_json::json;

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("mcp-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A new project in `dir`, recorded in `dir/ide.json` as open in this
/// process, which stands in for the IDE.
fn opened(dir: &Path) -> (PathBuf, PathBuf) {
    let stack = clarity_project::starter("Todo", "todo").stack;
    let project = clarity_tools::new_project::write_new_project(&dir.join("todo"), stack).unwrap();
    let file = dir.join("ide.json");
    ide::set_open_project_in(&file, &project, &Ide::current().unwrap()).unwrap();
    (file, project)
}

/// The agent designs and generates; it never builds, runs or touches the
/// database, and nothing that needs the IDE is offered. A tool the shared
/// set gains is offered only when it is none of these.
#[test]
fn every_shared_tool_is_offered_except_the_left_out_ones() {
    let shared = clarity_tools::ProjectTools::new(None, std::env::temp_dir());
    let expected: Vec<String> = shared.definitions().into_iter().map(|t| t.name).filter(|n| !LEFT_OUT.contains(&n.as_str())).collect();
    let offered: Vec<String> = ClarityServer::offered().into_iter().map(|t| t.name).collect();
    assert_eq!(offered, expected);
    for left in LEFT_OUT {
        assert!(!offered.iter().any(|n| n == left), "{left} must not be offered");
    }
    assert_eq!(LEFT_OUT, ["build", "build_output", "list_backups", "backup_data", "restore_backup", "show_window", "show_diagram"]);
}

#[test]
fn without_an_open_project_the_tools_say_to_open_one_in_clarity() {
    let dir = scratch("none");
    let server = ClarityServer::with_ide_file(dir.join("ide.json"));
    for tool in ["project_overview", "read_file", "check", "generate"] {
        let answer = server.call(tool, &json!({ "path": "todo.clarity" }));
        assert_eq!(answer.text, NO_PROJECT, "{tool}");
        assert!(answer.is_error, "{tool}");
    }
}

#[test]
fn the_manual_needs_no_open_project() {
    let dir = scratch("manual");
    let server = ClarityServer::with_ide_file(dir.join("ide.json"));
    let answer = server.call("manual", &json!({ "topic": "" }));
    assert!(!answer.is_error, "{}", answer.text);
    assert_ne!(answer.text, NO_PROJECT);
}

#[test]
fn a_left_out_tool_is_refused_when_called_by_name() {
    let dir = scratch("refused");
    let (file, _) = opened(&dir);
    let server = ClarityServer::with_ide_file(file);
    for left in LEFT_OUT {
        let answer = server.call(left, &json!({}));
        assert!(answer.is_error, "{left}");
        assert!(answer.text.contains("not offered"), "{left}: {}", answer.text);
    }
}

#[test]
fn the_tools_follow_the_project_the_ide_opens() {
    let dir = scratch("follow");
    let (file, first) = opened(&dir);
    let server = ClarityServer::with_ide_file(file.clone());
    assert!(server.call("project_overview", &json!({})).text.contains(&first.display().to_string()));
    let stack = clarity_project::starter("Shop", "shop").stack;
    let second = clarity_tools::new_project::write_new_project(&dir.join("shop"), stack).unwrap();
    ide::set_open_project_in(&file, &second, &Ide::current().unwrap()).unwrap();
    let overview = server.call("project_overview", &json!({}));
    assert!(overview.text.contains(&second.display().to_string()), "{}", overview.text);
}

/// A client speaks MCP to the server through a pipe: it starts, lists the
/// tools and calls one on the open project.
#[tokio::test]
async fn a_client_lists_and_calls_the_tools_over_mcp() {
    let dir = scratch("round");
    let (file, project) = opened(&dir);
    let (server_end, client_end) = tokio::io::duplex(1 << 16);
    let server = tokio::spawn(async move {
        let running = ClarityServer::with_ide_file(file).serve(server_end).await.unwrap();
        running.waiting().await.unwrap();
    });
    let client = ().serve(client_end).await.unwrap();

    let info = client.peer_info().unwrap();
    assert!(info.instructions.as_deref().unwrap_or_default().contains("Clarity"));

    let names: Vec<String> = client.list_all_tools().await.unwrap().into_iter().map(|t| t.name.to_string()).collect();
    let offered: Vec<String> = ClarityServer::offered().into_iter().map(|t| t.name).collect();
    assert_eq!(names, offered);

    let result = client.call_tool(CallToolRequestParams::new("project_overview")).await.unwrap();
    assert_ne!(result.is_error, Some(true));
    let text = result.content[0].as_text().unwrap().text.clone();
    assert!(text.contains(&project.display().to_string()), "{text}");

    client.cancel().await.unwrap();
    server.await.unwrap();
}
