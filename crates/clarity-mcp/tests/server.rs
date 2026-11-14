//! Clarity as an MCP server (docs/MCP-PLAN.md): the shared tools minus the
//! ones that execute or need the IDE, always on the project the IDE has open.

use std::path::{Path, PathBuf};

use clarity_agent::Tools;
use clarity_mcp::{ClarityServer, LEFT_OUT, NOT_ALLOWED, NO_PROJECT, TURN_TOOLS, WHEN_OFF};
use clarity_tools::turn::{self, Shown};
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

    // The call is in the activity log, under the client's own name.
    let logged = clarity_project::activity::after(&dir.join("activity.jsonl"), 0);
    assert_eq!(logged.len(), 1, "{logged:?}");
    assert_eq!(logged[0].tool, "project_overview");
    assert!(!logged[0].client.is_empty());

    client.cancel().await.unwrap();
    server.await.unwrap();
}

/// Started by the IDE for a turn of its chat panel, the server also shows a
/// window and a diagram; plain `clarity mcp` never offers them.
#[test]
fn a_turn_adds_the_two_tools_that_show_and_nothing_else() {
    let dir = scratch("turn-tools");
    let plain = ClarityServer::with_ide_file(dir.join("ide.json"));
    assert_eq!(plain.tools().into_iter().map(|t| t.name).collect::<Vec<_>>(), ClarityServer::offered().into_iter().map(|t| t.name).collect::<Vec<_>>());
    let turn = plain.for_turn(dir.join("shown.jsonl"));
    let mut expected: Vec<String> = ClarityServer::offered().into_iter().map(|t| t.name).collect();
    expected.extend(TURN_TOOLS.iter().map(|t| t.to_string()));
    assert_eq!(turn.tools().into_iter().map(|t| t.name).collect::<Vec<_>>(), expected);
    assert_eq!(TURN_TOOLS, ["show_window", "show_diagram"]);
}

#[test]
fn showing_writes_a_line_for_the_ide_and_runs_nothing() {
    let dir = scratch("turn-show");
    let (file, _) = opened(&dir);
    let shown = dir.join("shown.jsonl");
    let server = ClarityServer::with_ide_file(file).for_turn(shown.clone());
    let answer = server.call("show_window", &json!({ "name": "Main" }));
    assert!(!answer.is_error, "{}", answer.text);
    assert!(!server.call("write_diagram", &json!({ "name": "billing", "text": "erDiagram\n" })).is_error);
    let answer = server.call("show_diagram", &json!({ "name": "billing" }));
    assert!(!answer.is_error, "{}", answer.text);
    assert_eq!(turn::shown(&shown), vec![Shown::Window("Main".into()), Shown::Diagram("billing.mermaid".into())]);
}

#[test]
fn a_window_or_diagram_that_is_not_there_is_refused() {
    let dir = scratch("turn-missing");
    let (file, _) = opened(&dir);
    let shown = dir.join("shown.jsonl");
    let server = ClarityServer::with_ide_file(file).for_turn(shown.clone());
    let answer = server.call("show_window", &json!({ "name": "Nowhere" }));
    assert!(answer.is_error);
    assert!(answer.text.contains("Main"), "names the windows there are: {}", answer.text);
    assert!(server.call("show_diagram", &json!({ "name": "missing" })).is_error);
    assert!(!shown.exists(), "nothing written");
}

#[test]
fn showing_needs_an_open_project() {
    let dir = scratch("turn-none");
    let server = ClarityServer::with_ide_file(dir.join("ide.json")).for_turn(dir.join("shown.jsonl"));
    assert_eq!(server.call("show_window", &json!({ "name": "Main" })).text, NO_PROJECT);
}

#[test]
fn a_project_made_in_a_turn_opens_when_the_turn_ends() {
    let dir = scratch("turn-made");
    let (file, _) = opened(&dir);
    let shown = dir.join("shown.jsonl");
    let server = ClarityServer::with_ide_file(file).for_turn(shown.clone());
    let answer = server.call("new_project", &json!({ "name": "shop", "folder": dir.display().to_string() }));
    assert!(!answer.is_error, "{}", answer.text);
    assert!(answer.text.contains("opens in Clarity when this turn ends"), "{}", answer.text);
    let made = turn::shown(&shown);
    assert!(matches!(made.as_slice(), [Shown::Project(p)] if p.file_name().unwrap() == "shop.clarity"), "{made:?}");
}

#[test]
fn a_project_made_without_a_turn_writes_nothing() {
    let dir = scratch("plain-made");
    let (file, _) = opened(&dir);
    let server = ClarityServer::with_ide_file(file);
    let answer = server.call("new_project", &json!({ "name": "shop", "folder": dir.display().to_string() }));
    assert!(!answer.is_error, "{}", answer.text);
    assert!(!answer.text.contains("when this turn ends"));
}

#[test]
fn turned_off_only_the_manual_and_references_answer() {
    let dir = scratch("off");
    let (file, _) = opened(&dir);
    ide::set_outside_agents_in(&file, false).unwrap();
    let server = ClarityServer::with_ide_file(file.clone()).for_turn(dir.join("shown.jsonl"));
    for tool in ["project_overview", "read_file", "write_file", "check", "generate", "new_project", "show_window"] {
        let answer = server.call(tool, &json!({ "path": "todo.clarity", "name": "Main" }));
        assert!(answer.is_error, "{tool}");
        assert_eq!(answer.text, NOT_ALLOWED, "{tool}");
    }
    for tool in WHEN_OFF {
        assert_ne!(server.call(tool, &json!({ "topic": "", "section": "" })).text, NOT_ALLOWED, "{tool}");
    }
    ide::set_outside_agents_in(&file, true).unwrap();
    assert!(!server.call("project_overview", &json!({})).is_error);
}

#[test]
fn every_call_is_logged_without_what_the_file_holds() {
    let dir = scratch("logged");
    let (file, project) = opened(&dir);
    let module = clarity_project::module_dir(&project).join("todo.clarity");
    let text = std::fs::read_to_string(&module).unwrap();
    let server = ClarityServer::with_ide_file(file);
    assert!(!server.call_from("codex", "read_file", &json!({ "path": "todo.clarity" })).is_error);
    assert!(server.call_from("copilot", "read_file", &json!({ "path": "nowhere.clarity" })).is_error);

    let logged = clarity_project::activity::after(&dir.join("activity.jsonl"), 0);
    assert_eq!(logged.len(), 2);
    assert_eq!((logged[0].client.as_str(), logged[0].tool.as_str(), logged[0].path.as_deref(), logged[0].ok), ("codex", "read_file", Some("todo.clarity"), true));
    assert_eq!(logged[0].said, format!("{} characters", text.chars().count()));
    let first_line = text.lines().find(|l| !l.trim().is_empty()).unwrap();
    assert!(!logged[0].said.contains(first_line.trim()), "no file text in the log");
    assert_eq!((logged[1].client.as_str(), logged[1].ok), ("copilot", false));
}

/// The server tells its client Clarity's own icon, so a client such as VS Code
/// shows it beside the server's name: a PNG every client takes and the SVG it
/// is made from, embedded so they need no network.
#[test]
fn the_server_tells_clarity_s_icon() {
    use base64::Engine;
    use rmcp::ServerHandler;
    let dir = scratch("icon");
    let info = ClarityServer::with_ide_file(dir.join("ide.json")).get_info().server_info;
    let icons = info.icons.expect("the server's info has icons");
    let embedded = |mime: &str| {
        let icon = icons.iter().find(|i| i.mime_type.as_deref() == Some(mime)).unwrap_or_else(|| panic!("no {mime} icon in {icons:?}"));
        let data = icon.src.strip_prefix(&format!("data:{mime};base64,")).unwrap_or_else(|| panic!("the {mime} icon is embedded: {}", icon.src));
        (base64::engine::general_purpose::STANDARD.decode(data).unwrap(), icon.sizes.clone())
    };
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../app/src-tauri/icons");
    assert_eq!(embedded("image/png"), (std::fs::read(root.join("128x128.png")).unwrap(), Some(vec!["128x128".to_string()])));
    assert_eq!(embedded("image/svg+xml"), (std::fs::read(root.join("icon.svg")).unwrap(), Some(vec!["any".to_string()])));
}
