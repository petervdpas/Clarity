//! The cDdev's DevTools tools, for working on Clarity itself: press a
//! piece's Build (build_piece), read how Clarity's releases and their checks
//! stand (release_state), read how big the build folders are and how much
//! room is left (disk_state), and ask to remove a build folder
//! (remove_folder). Compiled only into a server built from Clarity's
//! repository (lib.rs, `#[cfg(debug_assertions)]`): a release has none, as
//! an installed Clarity has no DevTools. A build and a removal go to the IDE
//! as asks (clarity_project::dev_asks), which the person allows.

use std::path::PathBuf;

use clarity_agent::{ToolDef, ToolResult};
use clarity_gen::tools::devtools::{disk, release, swap};
use clarity_project::dev_asks::{self, Action};
use serde_json::{Value, json};

use crate::{ClarityServer, NOT_ALLOWED};

/// The tool that presses a DevTools tab's Build.
pub const BUILD_PIECE: &str = "build_piece";
/// The tool that says how Clarity's releases and their checks stand.
pub const RELEASE_STATE: &str = "release_state";
/// The tool that says how big the build folders are and how much room is left.
pub const DISK_STATE: &str = "disk_state";
/// The tool that asks to remove a build folder.
pub const REMOVE_FOLDER: &str = "remove_folder";
/// The tool that writes a release plan, only while every release check passed on the tree.
pub const WRITE_RELEASE_PLAN: &str = "write_release_plan";

/// The name the developer's build gives, as `.mcp.json` names it.
pub(crate) const NAME: &str = "clarity-cdev";

/// What the agent is told of these tools, beside the server's own lines.
pub(crate) const LINE: &str = "This server is clarity-cdev, the developer's build of Clarity from its repository (a debug build, started by \
./mcp.sh); an installed Clarity serves as clarity, without the tools below. Working on Clarity itself, build a piece of Clarity (its library, browser files, runtimes) with build_piece, \
which presses its Build in Clarity settings > DevTools, never with cargo or a script of your own. Read how its releases stand with \
release_state before writing a release plan, and write it only with write_release_plan, which refuses while a release check \
has not passed on the tree as it is. Read disk_state before a build or the release checks; when room is short, ask the person \
with remove_folder, never remove build folders yourself.";

fn failed(text: impl Into<String>) -> ToolResult {
    ToolResult { text: text.into(), is_error: true }
}

/// The DevTools tools, as the server offers them.
pub(crate) fn tools() -> Vec<ToolDef> {
    let pieces: Vec<&str> = swap::PIECES.iter().map(|p| p.key).collect();
    let folders: Vec<&str> = disk::places(&PathBuf::new()).iter().map(|p| p.key).collect();
    vec![
        ToolDef {
            name: BUILD_PIECE.into(),
            description: format!(
                "For working on Clarity itself: presses the Build button of a piece's tab in Clarity settings > DevTools, in a Clarity run from \
its repository (./dev.sh), which builds the cDdev's own build of that piece and shows it in the tab. The person allows it as they chose for \
agents. Waits for the build and gives how it ended with its last lines. Pieces: {}.",
                pieces.join(", ")
            ),
            input_schema: json!({ "type": "object", "properties": { "piece": { "type": "string", "enum": pieces } }, "required": ["piece"] }),
        },
        ToolDef {
            name: RELEASE_STATE.into(),
            description: "For working on Clarity itself: how each of Clarity's releases stands, read from its repository (the newest tag, \
the version in source, the files changed since the tag sorted into code, manifest, tests and docs), the chain of steps releasing every \
release whose code changed would start (the checks on main, each tag with its workflow and where it shows, what it waits for), and how \
the release checks last ended, which the person runs in Clarity settings > DevTools > Release planning. Read it to write the Release \
Helper's plan; it changes nothing."
                .into(),
            input_schema: json!({ "type": "object", "properties": {} }),
        },
        ToolDef {
            name: DISK_STATE.into(),
            description: "For working on Clarity itself: the free space of the disk Clarity's repository is on, whether it is below the \
40 GB builds and checks need, and each build folder of Clarity settings > DevTools > Disk with its size and what removing it costs. \
It changes nothing."
                .into(),
            input_schema: json!({ "type": "object", "properties": {} }),
        },
        ToolDef {
            name: REMOVE_FOLDER.into(),
            description: format!(
                "For working on Clarity itself: asks the person, in Clarity settings > DevTools, to remove a build folder of the Disk tab \
(see disk_state for what each costs). Clarity always asks the person first, as removing cannot be undone, and refuses while a build runs. \
Gives how it ended and what it freed. Folders: {}.",
                folders.join(", ")
            ),
            input_schema: json!({ "type": "object", "properties": { "folder": { "type": "string", "enum": folders } }, "required": ["folder"] }),
        },
        ToolDef {
            name: WRITE_RELEASE_PLAN.into(),
            description: "For working on Clarity itself: writes the Release Helper's plan (its format, ~/Projects/ReleaseHelper/README.md) as \
releases/<stamp>.json, `stamp` as YYYY-MM-DD-HHMM; its steps begin with the checks on main (a watch step of checks.yml) and an existing \
plan is never overwritten. After a failed release, `fix` (a plan's stamp) with `commit_message` only changes that plan's commit message. \
Either is refused while a release check (release_state) has failed, not run, or ran on another tree than the one now: the person runs \
them in Clarity settings > DevTools > Release planning (Run release checks). Only the fix routine's one line, `Fix the failed Clarity \
<version> release`, is written all the same. It is the only way a plan is written."
                .into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "stamp": { "type": "string", "description": "The new plan's name: YYYY-MM-DD-HHMM" },
                    "plan": { "type": "object", "description": "The plan: title, commit_message, steps, as the Release Helper reads it" },
                    "fix": { "type": "string", "description": "A plan's stamp, to change only its commit message after a failed release" },
                    "commit_message": { "type": "string", "description": "With fix: the new commit message" }
                }
            }),
        },
    ]
}

impl ClarityServer {
    /// The same server, waiting `take` for the IDE to take an ask and `build` for what it asked.
    pub fn with_waits(self, take: std::time::Duration, build: std::time::Duration) -> ClarityServer {
        ClarityServer { take_waits: take, build_waits: build, ..self }
    }

    /// The folder of the asks to the IDE's DevTools.
    pub fn asks(&self) -> PathBuf {
        self.asks.clone()
    }

    /// Calls the DevTools tool `name` for `client`; none when `name` is no DevTools tool.
    pub(crate) fn devtools_call(&self, client: &str, name: &str, input: &Value) -> Option<ToolResult> {
        if ![BUILD_PIECE, RELEASE_STATE, DISK_STATE, REMOVE_FOLDER, WRITE_RELEASE_PLAN].contains(&name) {
            return None;
        }
        if !clarity_project::ide::outside_agents_in(&self.ide_file) {
            return Some(failed(NOT_ALLOWED));
        }
        Some(match name {
            BUILD_PIECE => self.build_piece(client, input),
            RELEASE_STATE => json_of(repo().and_then(|repo| release_state(&repo))),
            DISK_STATE => json_of(repo().map(|repo| disk_state(&repo))),
            WRITE_RELEASE_PLAN => write_release_plan(input),
            _ => self.remove_folder(client, input),
        })
    }

    /// Asks the IDE to press the Build of the piece `input.piece`, and waits for the answer.
    fn build_piece(&self, client: &str, input: &Value) -> ToolResult {
        let key = input["piece"].as_str().unwrap_or_default();
        if swap::piece(key).is_none() {
            let keys: Vec<&str> = swap::PIECES.iter().map(|p| p.key).collect();
            return failed(format!("there is no piece '{key}': one of {}", keys.join(", ")));
        }
        self.ask_and_wait(Action::Build, key, client, &format!("The build of {key}"), "press Build in Clarity settings > DevTools")
    }

    /// Asks the IDE to remove the build folder `input.folder`, and waits for the person's answer.
    fn remove_folder(&self, client: &str, input: &Value) -> ToolResult {
        let key = input["folder"].as_str().unwrap_or_default();
        let keys: Vec<&str> = disk::places(&PathBuf::new()).iter().map(|p| p.key).collect();
        if !keys.contains(&key) {
            return failed(format!("there is no build folder '{key}': one of {}", keys.join(", ")));
        }
        self.ask_and_wait(Action::Remove, key, client, &format!("Removing {key}"), "remove it in Clarity settings > DevTools > Disk")
    }

    /// Writes an ask, waits for a Clarity run from its repository to take it, then for its answer.
    fn ask_and_wait(&self, action: Action, key: &str, client: &str, what: &str, by_hand: &str) -> ToolResult {
        let id = match dev_asks::ask_to(&self.asks, action, key, client) {
            Ok(id) => id,
            Err(why) => return failed(why),
        };
        let start = std::time::Instant::now();
        while !dev_asks::taken(&self.asks, &id) {
            if start.elapsed() >= self.take_waits {
                dev_asks::forget(&self.asks, &id);
                return failed(format!("No Clarity run from its repository took the ask: ask the person to start Clarity with ./dev.sh, or to {by_hand}."));
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        loop {
            if let Some(answer) = dev_asks::answer_of(&self.asks, &id) {
                dev_asks::forget(&self.asks, &id);
                return ToolResult { text: answer.text, is_error: !answer.ok };
            }
            if start.elapsed() >= self.build_waits {
                return failed(format!("{what} goes on after {} minutes; Clarity settings > DevTools shows it.", self.build_waits.as_secs() / 60));
            }
            std::thread::sleep(std::time::Duration::from_millis(250));
        }
    }
}

/// Clarity's repository: the one this server was built in.
fn repo() -> Result<PathBuf, String> {
    let from = std::env::current_exe().unwrap_or_default();
    clarity_gen::repository_of(&from)
        .or_else(|| clarity_gen::repository_of(&std::env::current_dir().unwrap_or_default()))
        .ok_or_else(|| "this server was not built in Clarity's repository".to_string())
}

fn json_of(state: Result<Value, String>) -> ToolResult {
    match state {
        Ok(v) => ToolResult { text: serde_json::to_string_pretty(&v).unwrap_or_default(), is_error: false },
        Err(why) => failed(why),
    }
}

/// How the releases and their checks stand.
fn release_state(repo: &std::path::Path) -> Result<Value, String> {
    let standing = release::standing(repo)?;
    let chain = release::chain(repo, &standing);
    let releases: Vec<Value> = standing
        .iter()
        .map(|s| {
            let unit = release::unit(&s.key);
            json!({
                "key": s.key, "name": s.name, "tag": s.tag, "released": s.released, "source": s.source,
                "bumped": s.bumped, "next": s.next, "workflow": unit.and_then(|u| u.workflow),
                "changes": s.changes,
            })
        })
        .collect();
    let (files, since) = release::since_ide_release(repo)?;
    let records = release::check_records(repo);
    let state = release::tree_state(repo)?;
    let (mut passed, mut failed, mut not_run) = (Vec::new(), Vec::new(), Vec::new());
    let checks = release::checks(repo, &files);
    for check in &checks {
        match records.get(&check.key) {
            // A result of another tree (a file the check reads changed since) counts as not run.
            Some(r) if r.tree != state.fingerprint(check) => not_run.push(json!(format!("{} (ran on another tree)", r.title))),
            Some(r) if r.passed => passed.push(json!({ "title": r.title, "ended": r.ended, "head": r.head })),
            Some(r) => failed.push(json!({ "title": r.title, "why": r.why, "lines": r.lines, "ended": r.ended, "head": r.head })),
            None => not_run.push(json!(check.title)),
        }
    }
    Ok(json!({
        "head": release::head(repo)?,
        "since": since,
        "releases": releases,
        "chain": chain,
        "checks": { "total": checks.len(), "passed": passed, "failed": failed, "notRun": not_run },
        "planAllowed": failed.is_empty() && not_run.is_empty(),
    }))
}

/// Writes a release plan, or a fixed one's commit message, while no release check holds it back
/// (the fix routine's one line is never held back).
fn write_release_plan(input: &Value) -> ToolResult {
    let repo = match repo() {
        Ok(repo) => repo,
        Err(why) => return failed(why),
    };
    let fix = input.get("fix").and_then(Value::as_str);
    let new = input.get("stamp").and_then(Value::as_str).zip(input.get("plan"));
    if fix.is_none() && new.is_none() {
        return failed("give a new plan (stamp and plan), or fix (a plan's stamp) with commit_message");
    }
    // The fix routine's one line changes nothing the checks look at, so it is never held back.
    let routine_fix = fix.is_some() && input.get("commit_message").and_then(Value::as_str).is_some_and(release::is_fix_line);
    match release::held_back(&repo) {
        Ok(_) if routine_fix => {}
        Ok(held) if !held.is_empty() => {
            let shown: Vec<String> = held.iter().take(15).cloned().collect();
            let more = if held.len() > shown.len() { format!(" and {} more", held.len() - shown.len()) } else { String::new() };
            return failed(format!(
                "The plan is held back: {} release checks have not passed on the tree as it is ({}{more}). Ask the person to press Run release checks in Clarity settings > DevTools > Release planning, then try again.",
                held.len(),
                shown.join("; ")
            ));
        }
        Ok(_) => {}
        Err(why) => return failed(why),
    }
    let written = match (fix, new) {
        (Some(stamp), _) => match input.get("commit_message").and_then(Value::as_str) {
            Some(message) => release::set_commit_message(&repo, stamp, message),
            None => Err("a fix gives the new commit_message".to_string()),
        },
        (None, Some((stamp, plan))) => release::write_plan(&repo, stamp, plan),
        (None, None) => unreachable!(),
    };
    match written {
        Ok(file) if routine_fix => ToolResult { text: format!("Written: {}.", file.display()), is_error: false },
        Ok(file) => ToolResult { text: format!("Written: {}. Every release check passed on this tree.", file.display()), is_error: false },
        Err(why) => failed(why),
    }
}

/// How big the build folders are and how much room is left.
fn disk_state(repo: &std::path::Path) -> Value {
    let free = disk::free_space(repo);
    let folders: Vec<Value> = disk::places(repo)
        .iter()
        .map(|p| {
            let bytes = disk::size(p);
            json!({ "folder": p.key, "name": p.name, "bytes": bytes, "size": disk::bytes_text(bytes), "cost": p.cost })
        })
        .collect();
    json!({
        "free": free,
        "freeText": free.map(disk::bytes_text),
        "low": free.is_some_and(|f| f < disk::ROOM),
        "room": disk::bytes_text(disk::ROOM),
        "folders": folders,
    })
}
