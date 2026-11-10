//! The agent programs Clarity's chat panel starts (docs/ACP-PLAN.md, step 4):
//! found on the PATH or at the path the person set, with the arguments that
//! make them speak ACP.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use clarity_acp::vendors::{self, Vendor};

fn folder(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("vendors-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A program file named as the vendor's is on this system, that can be started.
fn fake_program(dir: &Path, vendor: Vendor) -> PathBuf {
    let file = dir.join(vendors::program_names(vendor)[0]);
    std::fs::write(&file, "#!/bin/sh\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    file
}

fn path_of(folders: &[&Path]) -> OsString {
    std::env::join_paths(folders).unwrap()
}

#[test]
fn a_vendor_is_named_by_its_connection_kind_in_any_case() {
    assert_eq!(Vendor::of_kind("codex"), Some(Vendor::Codex));
    assert_eq!(Vendor::of_kind("Copilot"), Some(Vendor::Copilot));
    assert_eq!(Vendor::of_kind(" COPILOT "), Some(Vendor::Copilot));
    assert_eq!(Vendor::of_kind("anthropic"), None);
    assert_eq!(Vendor::Codex.name(), "Codex");
    assert_eq!(Vendor::Copilot.name(), "GitHub Copilot");
    for vendor in [Vendor::Codex, Vendor::Copilot] {
        assert!(vendor.install_page().starts_with("https://"));
    }
}

#[test]
fn the_program_is_found_on_the_path_with_its_acp_arguments() {
    let empty = folder("path-empty");
    let tools = folder("path-tools");
    let copilot = fake_program(&tools, Vendor::Copilot);
    let found = vendors::find_in(Vendor::Copilot, None, Some(&path_of(&[&empty, &tools]))).unwrap();
    assert_eq!(found.command, copilot);
    assert_eq!(found.args, vec!["--acp".to_string(), "--stdio".to_string()]);
    assert!(found.env.is_empty());
}

#[test]
fn a_missing_program_names_where_to_install_it() {
    let empty = folder("missing");
    for vendor in [Vendor::Codex, Vendor::Copilot] {
        let why = vendors::find_in(vendor, None, Some(&path_of(&[&empty]))).unwrap_err();
        assert!(why.contains(vendor.install_page()), "{why}");
        assert!(why.contains(vendor.name()), "{why}");
    }
    assert!(vendors::find_in(Vendor::Codex, None, None).is_err(), "no PATH at all");
}

#[test]
fn a_path_from_the_settings_is_used_as_it_is() {
    let dir = folder("settings");
    let chosen = fake_program(&dir, Vendor::Codex);
    let empty = folder("settings-empty");
    let found = vendors::find_in(Vendor::Codex, Some(&chosen), Some(&path_of(&[&empty]))).unwrap();
    assert_eq!(found.command, chosen);
    // Started asking before it changes anything (its ACP mode read-only), never in its default mode.
    assert_eq!(found.env, vec![("INITIAL_AGENT_MODE".to_string(), "read-only".to_string())]);
    assert!(found.args.is_empty());
    let gone = dir.join("nowhere");
    let why = vendors::find_in(Vendor::Codex, Some(&gone), Some(&path_of(&[&empty]))).unwrap_err();
    assert!(why.contains(&gone.display().to_string()), "{why}");
}

/// A folder of the same name, or (on Linux and macOS) a file that cannot be
/// started, is not the program.
#[test]
fn only_a_program_that_can_be_started_counts() {
    let dir = folder("not-program");
    std::fs::create_dir_all(dir.join(vendors::program_names(Vendor::Copilot)[0])).unwrap();
    assert!(vendors::find_in(Vendor::Copilot, None, Some(&path_of(&[&dir]))).is_err());
    #[cfg(unix)]
    {
        let plain = folder("not-executable");
        std::fs::write(plain.join(vendors::program_names(Vendor::Copilot)[0]), "text").unwrap();
        assert!(vendors::find_in(Vendor::Copilot, None, Some(&path_of(&[&plain]))).is_err());
    }
}

/// Copilot takes MCP servers over ACP only through HTTP (its log: "Rejecting
/// non-http/sse MCP server"), so Clarity's tools go to it at its start, as
/// `--additional-mcp-config`, allowed as a whole since they run nothing.
/// Codex takes them with the session.
#[test]
fn copilot_takes_clarity_s_tools_at_its_start() {
    let tools = clarity_acp::ToolServer { name: "clarity".into(), command: PathBuf::from("/opt/clarity/clarity"), args: vec!["mcp".into(), "--turn".into(), "/tmp/turn.jsonl".into()] };
    assert_eq!(Vendor::Codex.tools_at_start(&tools), None);
    let args = Vendor::Copilot.tools_at_start(&tools).expect("Copilot's start arguments");
    assert_eq!(args[0], "--additional-mcp-config");
    let config: serde_json::Value = serde_json::from_str(&args[1]).unwrap();
    let server = &config["mcpServers"]["clarity"];
    assert_eq!(server["type"], "local");
    assert_eq!(server["command"], "/opt/clarity/clarity");
    assert_eq!(server["args"], serde_json::json!(["mcp", "--turn", "/tmp/turn.jsonl"]));
    assert_eq!(server["tools"], serde_json::json!(["*"]));
    // Copilot hands a server only PATH; Clarity's tools find the open project through HOME.
    if let Some(home) = std::env::var_os("HOME") {
        assert_eq!(server["env"]["HOME"], home.to_string_lossy().as_ref());
    }
    assert_eq!(&args[2..], ["--allow-tool=clarity"]);
}

/// Each vendor signs in its own way, by name: Codex with its ChatGPT method
/// (`chat-gpt`, never the `api-key` it lists first), Copilot with its login command.
#[test]
fn each_vendor_signs_in_by_name() {
    assert_eq!(Vendor::Codex.sign_in(), vendors::SignIn::Acp("chat-gpt"));
    assert_eq!(Vendor::Copilot.sign_in(), vendors::SignIn::Command(&["login", "--web-flow"]));
}

/// A missing program says where its path goes: the connection in the vault.
#[test]
fn a_missing_program_points_at_the_connection() {
    let err = vendors::find_in(Vendor::Codex, None, Some(OsString::from("").as_os_str())).unwrap_err();
    assert!(err.contains("field command of its connection in the vault"), "{err}");
    assert!(!err.contains("Clarity's settings"), "{err}");
}
