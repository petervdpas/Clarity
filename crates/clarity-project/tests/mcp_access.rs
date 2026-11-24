//! Each MCP server's switch, kept in `mcp.json` in Clarity's own folder: the
//! person turns `clarity` and `clarity-cdev` on and off in Clarity's status
//! bar, and each server reads its switch on every call (docs/ledger/mcp-tools.md).

use std::path::{Path, PathBuf};

use clarity_project::mcp_access::{self, SERVERS};

fn home(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("mcp-access-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn the_servers_are_clarity_and_clarity_cdev() {
    assert_eq!(SERVERS, ["clarity", "clarity-cdev"]);
    assert_eq!(mcp_access::file().file_name().unwrap(), "mcp.json");
}

#[test]
fn without_the_file_both_are_on() {
    let file = home("missing").join("mcp.json");
    for server in SERVERS {
        assert!(mcp_access::is_on_in(&file, server), "{server}");
    }
    assert_eq!(mcp_access::states_in(&file), vec![("clarity".to_string(), true), ("clarity-cdev".to_string(), true)]);
}

#[test]
fn each_is_turned_off_and_on_by_itself() {
    let file = home("each").join("mcp.json");
    mcp_access::set_in(&file, "clarity-cdev", false).unwrap();
    assert!(mcp_access::is_on_in(&file, "clarity"));
    assert!(!mcp_access::is_on_in(&file, "clarity-cdev"));
    mcp_access::set_in(&file, "clarity", false).unwrap();
    mcp_access::set_in(&file, "clarity-cdev", true).unwrap();
    assert_eq!(mcp_access::states_in(&file), vec![("clarity".to_string(), false), ("clarity-cdev".to_string(), true)]);
    let written: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    assert_eq!(written, serde_json::json!({ "clarity": false, "clarity-cdev": true }));
}

#[test]
fn other_keys_in_the_file_stay() {
    let file = home("keys").join("mcp.json");
    std::fs::write(&file, r#"{ "note": "mine" }"#).unwrap();
    mcp_access::set_in(&file, "clarity", false).unwrap();
    let written: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    assert_eq!(written, serde_json::json!({ "note": "mine", "clarity": false }));
}

#[test]
fn a_server_clarity_does_not_have_is_refused() {
    let file = home("unknown").join("mcp.json");
    let refused = mcp_access::set_in(&file, "other", false).unwrap_err();
    assert!(refused.contains("clarity-cdev"), "names the servers there are: {refused}");
    assert!(!file.exists(), "nothing written");
}

/// A file that is there but cannot be read as the switches counts as off,
/// so a damaged file never opens access the person closed; turning a server
/// on or off writes the file again.
#[test]
fn a_damaged_file_counts_as_off_until_a_switch_is_set() {
    let file = home("damaged").join("mcp.json");
    std::fs::write(&file, "not json").unwrap();
    for server in SERVERS {
        assert!(!mcp_access::is_on_in(&file, server), "{server}");
    }
    mcp_access::set_in(&file, "clarity", true).unwrap();
    assert!(mcp_access::is_on_in(&file, "clarity"));
    assert!(mcp_access::is_on_in(&file, "clarity-cdev"), "a server the new file does not name is on");
}
