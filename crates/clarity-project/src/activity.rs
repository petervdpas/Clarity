//! What outside agents did through `clarity mcp` (docs/ACP-PLAN.md, "Agent
//! activity"): one JSON line per tool call in `activity.jsonl` in Clarity's
//! own folder, which the IDE follows. It names the program, the tool, the
//! file and how the call ended, never what a file holds.
//!
//! Each call is one write of a whole line to the end of the file, so a
//! reader sees whole lines (a line still being written has no line end yet
//! and is left for the next read). The log keeps the last calls: past twice
//! `KEPT` lines it is cut back to `KEPT`, written beside it and renamed into place.
//!
//! Several agents can call at once (one in Clarity's panel, one outside),
//! so writers take turns through the system's lock on `activity.jsonl.lock`
//! (on Linux, Windows and macOS alike). The system lets go of it when the
//! process ends, so a program that stopped leaves no lock behind; one that
//! hangs holding it is waited on until nothing was written to the log for
//! `LOCK_WAIT`, then the call is left out; a busy log keeps it waiting.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// How many calls the log keeps at least after cutting it back.
pub const KEPT: usize = 500;

/// How long a writer waits for its turn before it leaves its call out.
pub const LOCK_WAIT: Duration = Duration::from_secs(2);

/// How many characters of the result line are kept.
pub const SAID_LIMIT: usize = 160;

/// One tool call as the server saw it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Call {
    /// The MCP client's name, as it gave it when it started (like `codex`).
    pub client: String,
    pub tool: String,
    /// The file the call named, when it named one.
    pub path: Option<String>,
    pub ok: bool,
    /// A short line about the result: never a file's text.
    pub said: String,
}

/// One line of the log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Activity {
    /// The call's number: one more than the call before it.
    pub seq: u64,
    /// When, in seconds since 1970.
    pub time: u64,
    pub client: String,
    pub tool: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    pub ok: bool,
    pub said: String,
}

/// The log's file name, in Clarity's own folder beside `ide.json`.
pub const FILE_NAME: &str = "activity.jsonl";

/// Where the log is kept: `activity.jsonl` in Clarity's own folder.
pub fn file() -> PathBuf {
    crate::clarity_home().join(FILE_NAME)
}

/// Adds `call` to the log.
pub fn record(call: &Call) -> Result<Activity, String> {
    record_in(&file(), call)
}

/// The lock file writers of the log `file` take turns through.
pub fn lock_file(file: &Path) -> PathBuf {
    file.with_extension("jsonl.lock")
}

/// Waits for the turn to write the log `file`; it lasts as long as the
/// file handle it gives.
fn turn(file: &Path) -> Result<std::fs::File, String> {
    let failed = |err: std::io::Error| format!("cannot write {}: {err}", file.display());
    if let Some(folder) = file.parent() {
        std::fs::create_dir_all(folder).map_err(failed)?;
    }
    let lock = std::fs::OpenOptions::new().create(true).truncate(false).write(true).open(lock_file(file)).map_err(failed)?;
    // A busy log is no hung one: the wait starts again whenever the log
    // changed, so a writer gives up only when nothing was written for LOCK_WAIT.
    let written = || std::fs::metadata(file).ok().map(|m| (m.len(), m.modified().ok()));
    let mut last = written();
    let mut started = Instant::now();
    loop {
        match lock.try_lock() {
            Ok(()) => return Ok(lock),
            Err(std::fs::TryLockError::WouldBlock) => {
                let now = written();
                if now != last {
                    last = now;
                    started = Instant::now();
                } else if started.elapsed() >= LOCK_WAIT {
                    return Err(format!("{} stayed in use for {} seconds without a line written; this call is left out of it", file.display(), LOCK_WAIT.as_secs()));
                }
                std::thread::sleep(Duration::from_millis(5));
            }
            Err(std::fs::TryLockError::Error(err)) => return Err(failed(err)),
        }
    }
}

/// Adds `call` to the log `file`, numbered after its last line.
pub fn record_in(file: &Path, call: &Call) -> Result<Activity, String> {
    let failed = |err: std::io::Error| format!("cannot write {}: {err}", file.display());
    // Held until this call is written; the system lets go when it is dropped.
    let _turn = turn(file)?;
    let lines = whole_lines(file);
    let seq = lines.last().map_or(0, |a| a.seq) + 1;
    let said: String = call.said.lines().next().unwrap_or_default().chars().take(SAID_LIMIT).collect();
    let time = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let activity = Activity { seq, time, client: call.client.clone(), tool: call.tool.clone(), path: call.path.clone(), ok: call.ok, said };
    let line = serde_json::to_string(&activity).map_err(|err| err.to_string())? + "\n";
    if lines.len() + 1 > 2 * KEPT {
        // Cut back to the last calls, this one with them, through a file beside it.
        let mut text: String = lines[lines.len() + 1 - KEPT..].iter().filter_map(|a| serde_json::to_string(a).ok()).map(|l| l + "\n").collect();
        text.push_str(&line);
        let next = file.with_extension("jsonl.next");
        std::fs::write(&next, text).map_err(failed)?;
        std::fs::rename(&next, file).map_err(failed)?;
    } else {
        let mut log = std::fs::OpenOptions::new().create(true).append(true).open(file).map_err(failed)?;
        log.write_all(line.as_bytes()).map_err(failed)?;
    }
    Ok(activity)
}

/// The calls in the log after the one numbered `seq` (0 for all), oldest first.
pub fn after(file: &Path, seq: u64) -> Vec<Activity> {
    whole_lines(file).into_iter().filter(|a| a.seq > seq).collect()
}

/// The whole lines of `file` that read as calls.
fn whole_lines(file: &Path) -> Vec<Activity> {
    let text = std::fs::read_to_string(file).unwrap_or_default();
    let whole = match text.rfind('\n') {
        Some(end) => &text[..end],
        None => "",
    };
    whole.lines().filter_map(|line| serde_json::from_str(line).ok()).collect()
}
