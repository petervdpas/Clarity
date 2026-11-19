//! Clarity as an MCP server (docs/ledger/mcp-tools.md): the shared tools minus the
//! ones that execute or need the IDE, always on the project the IDE has open.

use std::path::{Path, PathBuf};

use clarity_agent::Tools;
use clarity_mcp::{ClarityServer, LEFT_OUT, NOT_ALLOWED, NO_PROJECT, TURN_TOOLS, WHEN_OFF};
use clarity_tools::turn::{self, Shown};
use clarity_project::ide::{self, Ide};
use rmcp::ServiceExt;
use rmcp::model::CallToolRequestParams;
use serde_json::{Value, json};

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
    let mut offered: Vec<String> = ClarityServer::offered().into_iter().map(|t| t.name).collect();
    // Built from the repository (the tests are), the server also offers the cDdev's DevTools Build.
    offered.push(clarity_mcp::BUILD_PIECE.to_string());
    offered.push(clarity_mcp::RELEASE_STATE.to_string());
    offered.push(clarity_mcp::DISK_STATE.to_string());
    offered.push(clarity_mcp::REMOVE_FOLDER.to_string());
    offered.push(clarity_mcp::WRITE_RELEASE_PLAN.to_string());
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
    let mut plain_tools: Vec<String> = ClarityServer::offered().into_iter().map(|t| t.name).collect();
    plain_tools.push(clarity_mcp::BUILD_PIECE.to_string());
    plain_tools.push(clarity_mcp::RELEASE_STATE.to_string());
    plain_tools.push(clarity_mcp::DISK_STATE.to_string());
    plain_tools.push(clarity_mcp::REMOVE_FOLDER.to_string());
    plain_tools.push(clarity_mcp::WRITE_RELEASE_PLAN.to_string());
    assert_eq!(plain.tools().into_iter().map(|t| t.name).collect::<Vec<_>>(), plain_tools);
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

/// build_piece presses a DevTools tab's Build in a Clarity run from its
/// repository: the IDE takes the ask, builds in the tab and answers.
#[test]
fn build_piece_asks_the_ide_and_gives_its_answer() {
    use clarity_project::dev_asks;
    let dir = scratch("build-piece");
    let (file, _) = opened(&dir);
    ide::set_outside_agents_in(&file, true).unwrap();
    let server = ClarityServer::with_ide_file(file.clone()).with_waits(std::time::Duration::from_millis(1500), std::time::Duration::from_secs(20));
    assert!(server.tools().iter().any(|t| t.name == "build_piece"), "offered by a server run from the repository");
    let unknown = server.call_from("Claude Code", "build_piece", &json!({ "piece": "nothing" }));
    assert!(unknown.is_error && unknown.text.contains("clarityUi"), "{}", unknown.text);
    // Nobody takes it: no Clarity from the repository runs.
    let alone = server.call_from("Claude Code", "build_piece", &json!({ "piece": "clarityUi" }));
    assert!(alone.is_error && alone.text.contains("./dev.sh"), "{}", alone.text);
    assert!(dev_asks::pending(&server.asks()).is_empty(), "an ask nobody took is taken back");
    // The IDE takes it and answers.
    let asks = server.asks();
    let ide = std::thread::spawn(move || {
        for _ in 0..100 {
            if let Some(ask) = dev_asks::pending(&asks).into_iter().next() {
                assert_eq!((ask.key.as_str(), ask.by.as_str()), ("clarityUi", "Claude Code"));
                assert!(dev_asks::take(&asks, &ask.id));
                dev_asks::answer(&asks, &ask.id, &dev_asks::Answer { ok: true, text: "Finished release".into() }).unwrap();
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        panic!("no ask came");
    });
    let built = server.call_from("Claude Code", "build_piece", &json!({ "piece": "clarityUi" }));
    ide.join().unwrap();
    assert!(!built.is_error && built.text.contains("Finished release"), "{}", built.text);
}

/// Working on Clarity itself, the agent reads how its releases stand to
/// write the Release Helper's plan: what changed per release by kind, the
/// chain of steps, and how the IDE's release checks last ended.
#[test]
fn release_state_says_how_the_releases_and_their_checks_stand() {
    let dir = scratch("release-state");
    let (file, _) = opened(&dir);
    ide::set_outside_agents_in(&file, true).unwrap();
    let server = ClarityServer::with_ide_file(file);
    assert!(server.tools().iter().any(|t| t.name == clarity_mcp::RELEASE_STATE), "offered by a server run from the repository");
    let said = server.call_from("Claude Code", clarity_mcp::RELEASE_STATE, &json!({}));
    assert!(!said.is_error, "{}", said.text);
    let state: Value = serde_json::from_str(&said.text).unwrap();
    let releases = state["releases"].as_array().unwrap();
    assert_eq!(releases.len(), clarity_gen::tools::devtools::release::UNITS.len());
    for r in releases {
        for kind in ["code", "manifest", "tests", "docs"] {
            assert!(r["changes"][kind].is_array(), "{} {kind}", r["name"]);
        }
        assert!(r["workflow"].is_string() || r["workflow"].is_null());
    }
    assert_eq!(state["chain"][0]["title"], "Checks on main", "the chain starts with the checks on main");
    assert!(state["checks"]["total"].as_u64().unwrap() > 0);
    for part in ["passed", "failed", "notRun"] {
        assert!(state["checks"][part].is_array(), "{part}");
    }
}

/// The agent reads how big the build folders are and the room left.
#[test]
fn disk_state_says_the_room_left_and_each_build_folder() {
    let dir = scratch("disk-state");
    let (file, _) = opened(&dir);
    ide::set_outside_agents_in(&file, true).unwrap();
    let server = ClarityServer::with_ide_file(file);
    let said = server.call_from("Claude Code", clarity_mcp::DISK_STATE, &json!({}));
    assert!(!said.is_error, "{}", said.text);
    let state: Value = serde_json::from_str(&said.text).unwrap();
    assert!(state["free"].as_u64().is_some_and(|f| f > 0) && state["low"].is_boolean());
    let folders = state["folders"].as_array().unwrap();
    assert!(folders.iter().any(|f| f["folder"] == "debug"));
    assert!(folders.iter().all(|f| f["cost"].is_string() && f["bytes"].is_u64()));
}

/// A removal goes to the IDE as an ask, and the agent hears the person's answer.
#[test]
fn remove_folder_asks_the_ide_and_gives_its_answer() {
    use clarity_project::dev_asks::{self, Action};
    let dir = scratch("remove-folder");
    let (file, _) = opened(&dir);
    ide::set_outside_agents_in(&file, true).unwrap();
    let server = ClarityServer::with_ide_file(file).with_waits(std::time::Duration::from_millis(1500), std::time::Duration::from_secs(20));
    let unknown = server.call_from("Claude Code", clarity_mcp::REMOVE_FOLDER, &json!({ "folder": "src" }));
    assert!(unknown.is_error && unknown.text.contains("debug"), "only the Disk tab's folders: {}", unknown.text);
    let alone = server.call_from("Claude Code", clarity_mcp::REMOVE_FOLDER, &json!({ "folder": "testScratch" }));
    assert!(alone.is_error && alone.text.contains("./dev.sh"), "{}", alone.text);
    let asks = server.asks();
    let ide = std::thread::spawn(move || {
        for _ in 0..100 {
            if let Some(ask) = dev_asks::pending(&asks).into_iter().next() {
                assert_eq!((ask.action, ask.key.as_str()), (Action::Remove, "testScratch"));
                assert!(dev_asks::take(&asks, &ask.id));
                dev_asks::answer(&asks, &ask.id, &dev_asks::Answer { ok: false, text: "The person did not allow removing this folder.".into() }).unwrap();
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        panic!("no ask came");
    });
    let said = server.call_from("Claude Code", clarity_mcp::REMOVE_FOLDER, &json!({ "folder": "testScratch" }));
    ide.join().unwrap();
    assert!(said.is_error && said.text.contains("did not allow"), "{}", said.text);
}

/// The DevTools tools are compiled only into a server built from the
/// repository: a release has none, as an installed Clarity has no DevTools.
#[test]
fn the_devtools_tools_are_never_compiled_into_a_release() {
    let lib = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/lib.rs")).unwrap();
    assert!(lib.contains("#[cfg(debug_assertions)]\nmod devtools;"), "the module is behind #[cfg(debug_assertions)]");
    for name in ["build_piece", "release_state", "disk_state", "remove_folder", "write_release_plan", "clarity_gen::tools::devtools", "dev_asks::ask"] {
        let outside = lib.lines().filter(|l| !l.trim_start().starts_with("//")).any(|l| l.contains(name));
        assert!(!outside, "{name} is used outside src/devtools.rs");
    }
    assert!(!lib.contains("cfg!(debug_assertions)"), "no tool hidden only at run time: kept out at compile time");
}

/// The developer's build (debug, `./mcp.sh`, `clarity-cdev` in .mcp.json)
/// says it is: its name and its instructions. A turn of the IDE's chat
/// panel is the app developer's, and is plain `clarity` (the release is
/// always `clarity`: the name is compiled in only with the DevTools tools).
#[test]
fn the_developers_build_names_itself_clarity_cdev() {
    use rmcp::ServerHandler;
    let dir = scratch("cdev-name");
    let server = ClarityServer::with_ide_file(dir.join("ide.json"));
    assert_eq!(server.get_info().server_info.name, "clarity-cdev");
    let said = server.instructions();
    assert!(said.contains("clarity-cdev") && said.contains("developer's build"), "{said}");
    let turn = server.for_turn(dir.join("shown.jsonl"));
    assert_eq!(turn.get_info().server_info.name, "clarity", "a chat turn is the app developer's");
    assert!(!turn.instructions().contains("clarity-cdev"));
}

/// A release plan is written only through write_release_plan, and only while
/// every release check passed on the tree as it is: else it says which hold
/// it back. A plan's commit message is changed at any time. The tree under test has uncommitted test changes of its own, so
/// its checks have not all passed on it.
#[test]
fn write_release_plan_refuses_while_a_check_holds_it_back() {
    let dir = scratch("release-plan");
    let (file, _) = opened(&dir);
    ide::set_outside_agents_in(&file, true).unwrap();
    let server = ClarityServer::with_ide_file(file);
    let plan = json!({ "stamp": "2026-11-17-2359", "plan": { "title": "t", "commit_message": "m", "steps": [{ "title": "Checks on main", "workflow": "checks.yml" }] } });
    let refused = server.call_from("Claude Code", clarity_mcp::WRITE_RELEASE_PLAN, &plan);
    assert!(refused.is_error && refused.text.contains("held back") && refused.text.contains("Run release checks"), "{}", refused.text);
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    assert!(!repo.join("releases/2026-11-17-2359.json").exists(), "nothing written");
    // A plan's commit message changes nothing the checks look at, so a fix is never held back (here it finds no such plan).
    for message in ["Fix the failed Clarity 1.4.0 release", "Go sums of runtime v0.65.0\n\n- version.go says v0.65.0."] {
        let fix = server.call_from("Claude Code", clarity_mcp::WRITE_RELEASE_PLAN, &json!({ "fix": "2026-11-17-2359", "commit_message": message }));
        assert!(fix.is_error && !fix.text.contains("held back") && fix.text.contains("no plan"), "a commit message is not held back: {}", fix.text);
    }
    let neither = server.call_from("Claude Code", clarity_mcp::WRITE_RELEASE_PLAN, &json!({}));
    assert!(neither.is_error && neither.text.contains("plan"), "{}", neither.text);
}
