//! The cDdev's DevTools tools, for working on Clarity itself: press a
//! piece's Build (build_piece, which returns once the build waits or runs
//! in its tab), follow it and every other build (build_state), read how Clarity's releases and their checks
//! stand (release_state), read how big the build folders are and how much
//! room is left (disk_state), and ask to remove a build folder
//! (remove_folder), and pick a piece's own build or the release
//! (swap_piece). Compiled only into a server built from Clarity's
//! repository (lib.rs, `#[cfg(debug_assertions)]`): a release has none, as
//! an installed Clarity has no DevTools. A build and a removal go to the IDE
//! as asks (clarity_project::dev_asks), which the person allows.

use std::path::PathBuf;

use clarity_agent::{ToolDef, ToolResult};
use clarity_devtools::jobs::{self, RecordState};
use clarity_devtools::{disk, release, swap};
use clarity_project::dev_asks::{self, Action};
use serde_json::{Value, json};

use crate::ClarityServer;

/// The tool that presses a DevTools tab's Build.
pub const BUILD_PIECE: &str = "build_piece";
/// The tool that says how a DevTools build stands: its state, times and last lines.
pub const BUILD_STATE: &str = "build_state";
/// The tool that picks a DevTools tab's own build or the release, as its Swap does.
pub const SWAP_PIECE: &str = "swap_piece";
/// The tool that says how Clarity's releases and their checks stand.
pub const RELEASE_STATE: &str = "release_state";
/// The tool that bumps every release in the chain to the version it would release.
pub const BUMP_VERSIONS: &str = "bump_versions";
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
./mcp.sh); an installed Clarity serves as clarity, without the tools below. Working on Clarity itself, build every piece of Clarity (its library, browser files, runtimes) with build_piece, \
which presses its Build in Clarity settings > DevTools, the one runner, where the person sees it live in its tab; it returns once the \
build waits or runs, and build_state follows it (its state, how long, its last lines) until it ended. Build a piece never with cargo \
or a script of your own from the terminal, which nobody sees. When a build or a test needs \
a piece the release lacks or your own build is newer, pick its own build with swap_piece (use own), and the release again once a \
release carries it. Read how its releases stand with \
release_state before writing a release plan, bump the versions with bump_versions before the release checks, and write it only with write_release_plan, which refuses while a release check \
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
its repository (./dev.sh), which builds the cDdev's own build of that piece and shows it live in the tab; one build runs at a time, the others wait their \
turn. The person allows it as they chose for agents. Returns as soon as the build waits or runs, never waiting for its end: follow \
it with build_state until it succeeded or failed. Pieces: {}.",
                pieces.join(", ")
            ),
            input_schema: json!({ "type": "object", "properties": { "piece": { "type": "string", "enum": pieces }, "debug": { "type": "boolean", "description": "Build debug instead: a native library's debug build (target/debug), which C# programs built in Debug and the runtimes' tests take" } }, "required": ["piece"] }),
        },
        ToolDef {
            name: BUILD_STATE.into(),
            description: format!(
                "For working on Clarity itself: how a build of Clarity settings > DevTools stands, as its tab shows it live, in a \
Clarity run from its repository (./dev.sh): waiting, running, succeeded, failed or stopped, since when and how long, who started it, \
what it runs and its last lines. With `piece` (and `debug` for its debug build) that piece's build, as build_piece started it; \
without, every build known (the pieces' and the release checks'). It changes nothing. Pieces: {}.",
                pieces.join(", ")
            ),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "piece": { "type": "string", "enum": pieces },
                    "debug": { "type": "boolean", "description": "The piece's debug build instead" },
                    "lines": { "type": "integer", "minimum": 0, "description": "How many of its last lines: 40 by default for a piece, 10 for each build without one" }
                }
            }),
        },
        ToolDef {
            name: SWAP_PIECE.into(),
            description: format!(
                "For working on Clarity itself: picks which build of a piece the programs Clarity builds use, as the Swap of its tab in \
Clarity settings > DevTools does, in a Clarity run from its repository (./dev.sh): `own` for the cDdev's own build (build it first with \
build_piece), `release` for the release. The person allows it as they chose for agents. Pieces: {}.",
                pieces.join(", ")
            ),
            input_schema: json!({
                "type": "object",
                "properties": { "piece": { "type": "string", "enum": pieces }, "use": { "type": "string", "enum": ["own", "release"] } },
                "required": ["piece", "use"]
            }),
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
            name: BUMP_VERSIONS.into(),
            description: "For working on Clarity itself, before the release checks: bumps every release in release_state's chain to the \
version it would release (its `next`), in its manifest (Cargo.toml, the C# props, the VS Code package.json), and every Cargo.toml that \
names its crate with a version (the carried crates' pin too); the Go runtime is left for its sums step, after its tag. Gives each \
change; a second call changes nothing. A manifest change restarts a Clarity run from the repository; run the generated-file guards and \
the carried guard after it."
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
plan is never overwritten; a field the Release Helper does not read (at the top, on a step of its kind, in a check) is refused. `fix` (a plan's stamp) with `commit_message` only changes that plan's commit message, at any time (after a \
failed release, for the Go sums). A new plan is refused while a release check (release_state) has failed, not run, or ran on another \
tree than the one now: the person runs them in Clarity settings > DevTools > Release planning (Run release checks). It is the only way \
a plan is written."
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
        if ![BUILD_PIECE, BUILD_STATE, SWAP_PIECE, RELEASE_STATE, BUMP_VERSIONS, DISK_STATE, REMOVE_FOLDER, WRITE_RELEASE_PLAN].contains(&name) {
            return None;
        }
        Some(match name {
            BUILD_PIECE => self.build_piece(client, input),
            BUILD_STATE => self.build_state(input),
            SWAP_PIECE => self.swap_piece(client, input),
            RELEASE_STATE => json_of(repo().and_then(|repo| release_state(&repo))),
            BUMP_VERSIONS => json_of(repo().and_then(|repo| clarity_devtools::bump::bump(&repo)).map(|done| serde_json::to_value(done).unwrap_or_default())),
            DISK_STATE => json_of(repo().map(|repo| disk_state(&repo))),
            WRITE_RELEASE_PLAN => write_release_plan(input),
            _ => self.remove_folder(client, input),
        })
    }

    /// Asks the IDE to press the Build of the piece `input.piece`, and
    /// returns once its build waits or runs (its record says so) or the IDE refused.
    fn build_piece(&self, client: &str, input: &Value) -> ToolResult {
        let key = input["piece"].as_str().unwrap_or_default();
        if swap::piece(key).is_none() {
            let keys: Vec<&str> = swap::PIECES.iter().map(|p| p.key).collect();
            return failed(format!("there is no piece '{key}': one of {}", keys.join(", ")));
        }
        let debug = input["debug"].as_bool() == Some(true);
        if debug && swap::piece(key).and_then(|p| swap::build_debug(std::path::Path::new("."), p)).is_none() {
            return failed(format!("{key} has no debug build: only a native library has one"));
        }
        let (action, by_hand) = if debug { (Action::BuildDebug, "press Build debug in Clarity settings > DevTools") } else { (Action::Build, "press Build in Clarity settings > DevTools") };
        let asked = now_ms();
        let id = match dev_asks::ask_to(&self.asks, action, key, client) {
            Ok(id) => id,
            Err(why) => return failed(why),
        };
        if let Err(refused) = self.wait_taken(&id, by_hand) {
            return refused;
        }
        let job = swap::job_key(key, debug);
        let records = dev_asks::jobs(&self.asks);
        let follow = if debug { format!("build_state with piece {key} and debug") } else { format!("build_state with piece {key}") };
        let start = std::time::Instant::now();
        loop {
            // Refused: the person said no, agents may not build, or it builds already.
            if let Some(answer) = dev_asks::answer_of(&self.asks, &id) {
                dev_asks::forget(&self.asks, &id);
                return ToolResult { text: answer.text, is_error: !answer.ok };
            }
            if let Some(record) = jobs::read_record(&records, &job)
                && record.queued.is_some_and(|q| q >= asked)
            {
                dev_asks::forget(&self.asks, &id);
                return ToolResult {
                    text: format!(
                        "The build is in its tab in Clarity settings > DevTools, where the person sees it live. Follow it with {follow} until it succeeded or failed; never build it with cargo.\n\n{}",
                        record.told(now_ms(), 10)
                    ),
                    is_error: record.state == RecordState::Failed,
                };
            }
            if start.elapsed() >= self.build_waits {
                dev_asks::forget(&self.asks, &id);
                return failed(format!("The build of {key} did not start within {} minutes: the person has not allowed it; Clarity settings > DevTools shows the ask.", self.build_waits.as_secs() / 60));
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }

    /// How the build of `input.piece` stands, or every build without one, as the IDE's records say.
    fn build_state(&self, input: &Value) -> ToolResult {
        let records = dev_asks::jobs(&self.asks);
        let now = now_ms();
        let Some(key) = input["piece"].as_str() else {
            let last = input["lines"].as_u64().map(|n| n as usize).unwrap_or(10);
            let all = jobs::read_records(&records);
            if all.is_empty() {
                return ToolResult { text: "No build is known: start one with build_piece.".into(), is_error: false };
            }
            return ToolResult { text: all.iter().map(|r| r.told(now, last)).collect::<Vec<_>>().join("\n\n"), is_error: false };
        };
        if swap::piece(key).is_none() {
            let keys: Vec<&str> = swap::PIECES.iter().map(|p| p.key).collect();
            return failed(format!("there is no piece '{key}': one of {}", keys.join(", ")));
        }
        let debug = input["debug"].as_bool() == Some(true);
        let last = input["lines"].as_u64().map(|n| n as usize).unwrap_or(40);
        match jobs::read_record(&records, &swap::job_key(key, debug)) {
            Some(record) => ToolResult { text: record.told(now, last), is_error: false },
            None => ToolResult {
                text: format!("No {}build of {key} is known since Clarity started from the repository: start it with build_piece.", if debug { "debug " } else { "" }),
                is_error: false,
            },
        }
    }

    /// Asks the IDE to swap the piece `input.piece` to its own build or the release (`input.use`), and waits for the answer.
    fn swap_piece(&self, client: &str, input: &Value) -> ToolResult {
        let key = input["piece"].as_str().unwrap_or_default();
        if swap::piece(key).is_none() {
            let keys: Vec<&str> = swap::PIECES.iter().map(|p| p.key).collect();
            return failed(format!("there is no piece '{key}': one of {}", keys.join(", ")));
        }
        let mine = match input["use"].as_str().unwrap_or_default() {
            "own" => true,
            "release" => false,
            other => return failed(format!("'use' is own (the cDdev's own build) or release, not '{other}'")),
        };
        let id = match dev_asks::ask_swap(&self.asks, key, mine, client) {
            Ok(id) => id,
            Err(why) => return failed(why),
        };
        self.wait_for(&id, &format!("Swapping {key}"), "pick it in its tab's Swap in Clarity settings > DevTools")
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
        self.wait_for(&id, what, by_hand)
    }

    /// Waits for a Clarity run from its repository to take the ask `id`; the refusal when none does.
    fn wait_taken(&self, id: &str, by_hand: &str) -> Result<(), ToolResult> {
        let start = std::time::Instant::now();
        while !dev_asks::taken(&self.asks, id) {
            if start.elapsed() >= self.take_waits {
                dev_asks::forget(&self.asks, id);
                return Err(failed(format!("No Clarity run from its repository took the ask: ask the person to start Clarity with ./dev.sh, or to {by_hand}.")));
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        Ok(())
    }

    /// Waits for a Clarity run from its repository to take the ask `id`, then for its answer.
    fn wait_for(&self, id: &str, what: &str, by_hand: &str) -> ToolResult {
        let id = id.to_string();
        let start = std::time::Instant::now();
        if let Err(refused) = self.wait_taken(&id, by_hand) {
            return refused;
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

fn now_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
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
    let (mut passed, mut failed, mut not_run) = (Vec::new(), Vec::new(), Vec::new());
    let checks = release::check_standings(repo, &files);
    for standing in &checks {
        match (standing.counted, &standing.record) {
            (release::Counted::Passed, Some(r)) => passed.push(json!({ "title": r.title, "ended": r.ended, "head": r.head })),
            (release::Counted::Failed, Some(r)) => failed.push(json!({ "title": r.title, "why": r.why, "lines": r.lines, "ended": r.ended, "head": r.head })),
            // A result of another tree (a file the check reads changed since) counts as not run.
            (release::Counted::OtherTree, Some(r)) => not_run.push(json!(format!("{} (ran on another tree)", r.title))),
            _ => not_run.push(json!(standing.check.title)),
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

/// Writes a release plan while no release check holds it back, or a plan's commit message at any time.
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
    // A commit message changes nothing the checks look at, so it is never held back.
    let message_only = fix.is_some();
    match release::held_back(&repo) {
        Ok(_) if message_only => {}
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
        Ok(file) if message_only => ToolResult { text: format!("Written: {}.", file.display()), is_error: false },
        Ok(file) => ToolResult { text: format!("Written: {}. Every release check passed on this tree.", file.display()), is_error: false },
        Err(why) => failed(why),
    }
}

/// How big the build folders are and how much room is left: DevTools' Disk tab (clarity_devtools::disk::view).
fn disk_state(repo: &std::path::Path) -> Value {
    let view = disk::view(repo);
    let folders: Vec<Value> = view
        .places
        .iter()
        .map(|p| json!({ "folder": p.key, "name": p.name, "bytes": p.bytes, "size": disk::bytes_text(p.bytes), "cost": p.cost, "marked": p.marked }))
        .collect();
    json!({
        "free": view.free,
        "freeText": view.free.map(disk::bytes_text),
        "low": view.low,
        "room": disk::bytes_text(disk::ROOM),
        "folders": folders,
    })
}
