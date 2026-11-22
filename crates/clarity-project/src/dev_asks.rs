//! An agent's ask to press a DevTools tab's Build (the MCP tool
//! build_piece), to pick its own build or the release (swap_piece), or to
//! remove a build folder of the Disk tab (remove_folder),
//! and its answer, as files in a folder both read: the
//! agent's side writes `<id>.ask.json`; the IDE takes it (renamed to
//! `<id>.taken.json`), lets the person allow it as they chose for agents,
//! builds in the tab and writes `<id>.answer.json`. Only a Clarity run from
//! its repository takes asks.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// How long the agent's side waits for the IDE to take an ask.
pub const TAKE_WAITS: Duration = Duration::from_secs(20);

/// How long the agent's side waits for the build at most.
pub const BUILD_WAITS: Duration = Duration::from_secs(45 * 60);

/// The folder of the asks: `dev-asks` in Clarity's own folder.
pub fn folder() -> PathBuf {
    crate::clarity_home().join("dev-asks")
}

/// What an agent asks the IDE to do.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Action {
    /// Press a piece's Build.
    #[default]
    Build,
    /// Press a native piece's Build debug.
    BuildDebug,
    /// Remove a build folder of the Disk tab.
    Remove,
    /// Pick a piece's own build (`mine`) or the release, as its tab's Swap does.
    Swap,
}

/// An ask to build a piece or remove a build folder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ask {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub action: Action,
    /// The piece's key (`clarityUi`) or the folder's (`debug`).
    pub key: String,
    /// Who asks: the MCP client's name.
    pub by: String,
    /// For a swap: the own build, else the release.
    #[serde(default)]
    pub mine: bool,
}

/// How the build ended, or why it did not run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Answer {
    pub ok: bool,
    pub text: String,
}

fn path(dir: &Path, id: &str, what: &str) -> PathBuf {
    dir.join(format!("{id}.{what}.json"))
}

fn plain(id: &str) -> bool {
    !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

/// Writes an ask to build the piece `key`; its id.
pub fn ask(dir: &Path, key: &str, by: &str) -> Result<String, String> {
    ask_to(dir, Action::Build, key, by)
}

/// Writes an ask to do `action` with `key`; its id.
pub fn ask_to(dir: &Path, action: Action, key: &str, by: &str) -> Result<String, String> {
    ask_with(dir, action, key, by, false)
}

/// Writes an ask to swap the piece `key` to its own build (`mine`) or the release; its id.
pub fn ask_swap(dir: &Path, key: &str, mine: bool, by: &str) -> Result<String, String> {
    ask_with(dir, Action::Swap, key, by, mine)
}

fn ask_with(dir: &Path, action: Action, key: &str, by: &str, mine: bool) -> Result<String, String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("cannot make {}: {e}", dir.display()))?;
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let id = format!("{nanos}-{}", std::process::id());
    let text = serde_json::to_string(&Ask { id: id.clone(), action, key: key.to_string(), by: by.to_string(), mine }).map_err(|e| e.to_string())?;
    // Written aside first, so the IDE never reads half an ask.
    let aside = path(dir, &id, "writing");
    std::fs::write(&aside, text).map_err(|e| format!("cannot write {}: {e}", aside.display()))?;
    std::fs::rename(&aside, path(dir, &id, "ask")).map_err(|e| format!("cannot ask: {e}"))?;
    Ok(id)
}

/// The asks nobody took yet, oldest first.
pub fn pending(dir: &Path) -> Vec<Ask> {
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut asks: Vec<Ask> = entries
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            let id = name.strip_suffix(".ask.json")?.to_string();
            let mut ask: Ask = serde_json::from_str(&std::fs::read_to_string(e.path()).ok()?).ok()?;
            ask.id = id;
            plain(&ask.id).then_some(ask)
        })
        .collect();
    asks.sort_by(|a, b| a.id.cmp(&b.id));
    asks
}

/// Takes the ask `id`: true only for the one who took it.
pub fn take(dir: &Path, id: &str) -> bool {
    plain(id) && std::fs::rename(path(dir, id, "ask"), path(dir, id, "taken")).is_ok()
}

/// Whether the ask `id` was taken.
pub fn taken(dir: &Path, id: &str) -> bool {
    path(dir, id, "taken").exists() || path(dir, id, "answer").exists()
}

/// Answers the ask `id`.
pub fn answer(dir: &Path, id: &str, answer: &Answer) -> Result<(), String> {
    if !plain(id) {
        return Err(format!("{id} is no ask"));
    }
    let text = serde_json::to_string(answer).map_err(|e| e.to_string())?;
    let aside = path(dir, id, "answering");
    std::fs::write(&aside, text).map_err(|e| format!("cannot write {}: {e}", aside.display()))?;
    std::fs::rename(&aside, path(dir, id, "answer")).map_err(|e| format!("cannot answer: {e}"))?;
    let _ = std::fs::remove_file(path(dir, id, "taken"));
    Ok(())
}

/// The answer of the ask `id`, once there is one.
pub fn answer_of(dir: &Path, id: &str) -> Option<Answer> {
    serde_json::from_str(&std::fs::read_to_string(path(dir, id, "answer")).ok()?).ok()
}

/// Forgets the ask `id` and its answer.
pub fn forget(dir: &Path, id: &str) {
    for what in ["ask", "taken", "answer"] {
        let _ = std::fs::remove_file(path(dir, id, what));
    }
}
