//! `clarity mcp`: Clarity's tools served to an AI agent over MCP on stdin
//! and stdout (docs/MCP-PLAN.md). Only the protocol goes to stdout.

use std::io::Write;
use std::process::{Command, Stdio};

#[test]
fn the_help_lists_mcp() {
    let clarity = env!("CARGO_BIN_EXE_clarity");
    let all = Command::new(clarity).arg("--help").output().unwrap();
    assert!(String::from_utf8_lossy(&all.stdout).contains("mcp"));
    let own = Command::new(clarity).args(["mcp", "--help"]).output().unwrap();
    assert!(own.status.success());
    assert!(String::from_utf8_lossy(&own.stdout).contains("MCP"));
    // The IDE starts it for a turn of its chat panel with the turn's file.
    assert!(String::from_utf8_lossy(&own.stdout).contains("--turn"));
}

#[test]
fn mcp_answers_initialize_and_prints_nothing_else() {
    let home = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("mcp-home");
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();
    let mut server = Command::new(env!("CARGO_BIN_EXE_clarity"))
        .arg("mcp")
        .env("CLARITY_HOME", &home)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let initialize = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1"}}}"#;
    let mut stdin = server.stdin.take().unwrap();
    writeln!(stdin, "{initialize}").unwrap();
    // The client goes: the server stops.
    drop(stdin);
    let out = server.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(lines.len(), 1, "only the answer on stdout: {stdout}");
    let answer: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(answer["id"], 1);
    assert_eq!(answer["result"]["serverInfo"]["name"], "clarity", "{answer}");
    assert!(answer["result"]["capabilities"]["tools"].is_object(), "{answer}");
}
