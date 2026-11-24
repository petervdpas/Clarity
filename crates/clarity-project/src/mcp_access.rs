//! Each MCP server's switch: whether `clarity` (the installed Clarity) and
//! `clarity-cdev` (the repository's own build) may answer an agent's tool
//! calls. The user: "I want to be able to turn them off/on (restrict access)
//! on demand!" Kept in `mcp.json` in Clarity's own folder, one key per server
//! (`{ "clarity": true, "clarity-cdev": false }`); the person turns them in
//! Clarity's status bar, and each server reads its own switch on every call,
//! so a change holds from the next call (docs/ledger/mcp-tools.md).
//!
//! A missing file, or a server it does not name, is on. A file that is there
//! but is not a JSON object counts as off, so a damaged file never opens
//! access the person closed; setting a switch writes the file again. Other
//! keys in the file stay.

use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

/// The servers, as each names itself and as the person reads them.
pub const SERVERS: [&str; 2] = ["clarity", "clarity-cdev"];

/// The file's name, in Clarity's own folder beside `ide.json`.
pub const FILE_NAME: &str = "mcp.json";

/// Where the switches are kept: `mcp.json` in Clarity's own folder.
pub fn file() -> PathBuf {
    crate::clarity_home().join(FILE_NAME)
}

/// Whether `server` may answer tool calls now.
pub fn is_on(server: &str) -> bool {
    is_on_in(&file(), server)
}

/// Whether `file` lets `server` answer; see the module's notes.
pub fn is_on_in(file: &Path, server: &str) -> bool {
    match read(file) {
        Read::Missing => true,
        Read::Damaged => false,
        Read::Switches(switches) => switches.get(server).and_then(Value::as_bool).unwrap_or(true),
    }
}

/// Every server with whether it is on, in the order of [`SERVERS`].
pub fn states() -> Vec<(String, bool)> {
    states_in(&file())
}

/// Every server with whether `file` has it on.
pub fn states_in(file: &Path) -> Vec<(String, bool)> {
    SERVERS.iter().map(|server| (server.to_string(), is_on_in(file, server))).collect()
}

/// Turns `server` on or off.
pub fn set(server: &str, on: bool) -> Result<(), String> {
    set_in(&file(), server, on)
}

/// Turns `server` on or off in `file`, keeping its other keys.
pub fn set_in(file: &Path, server: &str, on: bool) -> Result<(), String> {
    if !SERVERS.contains(&server) {
        return Err(format!("Clarity has no MCP server {server}; it has {}", SERVERS.join(" and ")));
    }
    let mut switches = match read(file) {
        Read::Switches(switches) => switches,
        Read::Missing | Read::Damaged => Map::new(),
    };
    switches.insert(server.to_string(), Value::Bool(on));
    save(file, &switches)
}

enum Read {
    Missing,
    Damaged,
    Switches(Map<String, Value>),
}

fn read(file: &Path) -> Read {
    let text = match std::fs::read_to_string(file) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Read::Missing,
        Err(_) => return Read::Damaged,
    };
    match serde_json::from_str(&text) {
        Ok(Value::Object(switches)) => Read::Switches(switches),
        _ => Read::Damaged,
    }
}

/// Writes `switches` to `file` through a file beside it, so a server never reads half of it.
fn save(file: &Path, switches: &Map<String, Value>) -> Result<(), String> {
    let failed = |err: std::io::Error| format!("cannot write {}: {err}", file.display());
    if let Some(folder) = file.parent() {
        std::fs::create_dir_all(folder).map_err(failed)?;
    }
    let text = serde_json::to_string_pretty(switches).unwrap_or_default() + "\n";
    let next = file.with_extension("json.next");
    std::fs::write(&next, text).map_err(failed)?;
    std::fs::rename(&next, file).map_err(failed)
}
