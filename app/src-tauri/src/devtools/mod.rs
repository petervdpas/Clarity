//! DevTools (Clarity settings, a Clarity run from its repository only): the
//! cDdev's tools for working on Clarity itself. A tab per piece of Clarity
//! an app uses: the release or the cDdev's own build, swapped
//! (clarity_gen::tools::devtools::swap, kept in environment.json), and a
//! Build button that makes the cDdev's own build, shown in the tab while it
//! builds (jobs.rs, one build at a time). An agent presses Build through
//! Clarity's MCP (build_piece): its ask comes through a file
//! (clarity_project::dev_asks), and the page allows it as the person chose
//! for agents. The release planning is release.rs. An installed Clarity has
//! no DevTools and always uses the release.

pub mod release;

use std::path::{Path, PathBuf};

use clarity_gen::clarity_ui;
use clarity_gen::tools::devtools::jobs::{Job, JobState, Jobs, OnChange};
use clarity_gen::tools::devtools::swap::{self, Kind, Piece, Swaps};
use clarity_gen::tools::environment::{self, Environment};
use clarity_project::dev_asks;
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use ts_rs::TS;

/// The event with a build each time it changes.
pub const JOB_CHANGED: &str = "devtools-job";
/// The event with an agent's ask to press a Build, for the page to allow.
pub const ASKED: &str = "devtools-asked";

/// The builds, one at a time, of the pieces and the release checks.
#[derive(Clone, Default)]
pub struct DevJobs(pub Jobs);

/// A build, as the page shows it.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct JobView {
    pub key: String,
    pub title: String,
    pub command: String,
    /// `waiting`, `running`, `passed` or `failed`.
    pub state: String,
    /// Why it failed.
    pub why: Option<String>,
    pub lines: Vec<String>,
    /// The agent that pressed it.
    pub by: Option<String>,
    #[ts(type = "number | null")]
    pub started: Option<u64>,
    #[ts(type = "number | null")]
    pub ended: Option<u64>,
}

impl From<&Job> for JobView {
    fn from(job: &Job) -> JobView {
        let (state, why) = match &job.state {
            JobState::Waiting => ("waiting", None),
            JobState::Running => ("running", None),
            JobState::Passed => ("passed", None),
            JobState::Failed(why) => ("failed", Some(why.clone())),
        };
        JobView { key: job.key.clone(), title: job.title.clone(), command: job.command.clone(), state: state.into(), why, lines: job.lines.clone(), by: job.by.clone(), started: job.started, ended: job.ended }
    }
}

/// A piece's tab, as the page shows it.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct PieceTab {
    pub key: String,
    pub name: String,
    /// What uses it, as people read it.
    pub uses: String,
    /// Its tab is always there.
    pub core: bool,
    /// The cDdev's own build is in use, not the release.
    pub mine: bool,
    /// The release this Clarity names: `0.27.0`.
    pub release: String,
    /// Where the cDdev's own build is.
    pub own: String,
    /// When the cDdev's build was made, in milliseconds since 1970; none
    /// when it is not built or is used from its source as it is.
    #[ts(type = "number | null")]
    pub built: Option<u64>,
    /// The signal: the cDdev's build is not there or is behind its source.
    pub signal: Option<String>,
    /// What its Build runs.
    pub build: String,
    /// Its build's key among the builds (`piece:clarityUi`).
    pub job: String,
}

/// A piece the cDdev can add a tab for.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct PieceChoice {
    pub key: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct DevToolsView {
    pub tabs: Vec<PieceTab>,
    pub addable: Vec<PieceChoice>,
    /// Where the swaps are kept.
    pub file: String,
    /// Every build so far, of the pieces and the release checks.
    pub jobs: Vec<JobView>,
}

/// An agent's ask to press a Build, for the page to allow.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct BuildAsk {
    pub id: String,
    pub key: String,
    pub name: String,
    pub by: String,
}

fn release_of(piece: &Piece) -> String {
    match piece.key {
        "rustCrates" => clarity_gen::rust_runtime_version().to_string(),
        "nugetPackages" => clarity_gen::csharp_runtime_version().to_string(),
        "goModule" => clarity_gen::go_runtime_version().to_string(),
        _ => clarity_ui::version().to_string(),
    }
}

fn own_of(repo: &Path, piece: &Piece, os: &str) -> String {
    let path = match piece.kind {
        Kind::Runtime { folder } => repo.join(folder),
        _ => swap::built_file(repo, piece, os).unwrap_or_else(|| repo.to_path_buf()),
    };
    clarity_project::slashed(&path.canonicalize().unwrap_or(path))
}

fn job_key(piece: &str) -> String {
    format!("piece:{piece}")
}

fn view(repo: &Path, jobs: &Jobs) -> DevToolsView {
    let os = std::env::consts::OS;
    let env = Environment::current();
    let swaps = env.swaps();
    let tabs = swaps
        .tabs()
        .into_iter()
        .filter_map(swap::piece)
        .map(|piece| PieceTab {
            key: piece.key.to_string(),
            name: piece.name.to_string(),
            uses: piece.uses.to_string(),
            core: piece.core,
            mine: swaps.mine(piece.key),
            release: release_of(piece),
            own: own_of(repo, piece, os),
            built: swap::built_at(repo, piece, os).and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_millis() as u64),
            signal: swap::behind(repo, piece, os),
            build: swap::build(repo, piece).shown(),
            job: job_key(piece.key),
        })
        .collect::<Vec<_>>();
    let addable = swap::PIECES.iter().filter(|p| !p.core && !tabs.iter().any(|t| t.key == p.key)).map(|p| PieceChoice { key: p.key.to_string(), name: p.name.to_string() }).collect();
    DevToolsView { tabs, addable, file: environment::file().display().to_string(), jobs: jobs.all().iter().map(JobView::from).collect() }
}

/// The release's folder of native libraries, fetched now when it is not on this machine yet.
fn release_libraries() -> Result<PathBuf, String> {
    let os = std::env::consts::OS;
    let cache = clarity_ui::cache_dir_in(os, |name| std::env::var(name).ok());
    let release = clarity_ui::library_in(None, None, cache.as_deref(), clarity_ui::rid(os, std::env::consts::ARCH).as_deref(), clarity_ui::version(), os);
    clarity_ui::provide(release, clarity_ui::RELEASES, &crate::build::download, &|line| eprintln!("{line}"))
}

/// Takes `swaps`: the library choices they give are kept and used from now on.
fn apply(repo: &Path, swaps: Swaps) -> Result<(), String> {
    let os = std::env::consts::OS;
    let natives = swaps.tabs().into_iter().filter(|k| swaps.mine(k)).filter_map(swap::piece).any(|p| matches!(p.kind, Kind::Native { .. }));
    let release = if natives { Some(release_libraries()?) } else { None };
    let choices = swap::choices(repo, &swaps, release.as_deref(), os)?;
    let file = environment::file();
    let mut env = Environment::load(&file);
    env.set_swaps(swaps, choices);
    env.save(&file).map_err(|e| format!("cannot keep the swaps in {}: {e}", file.display()))?;
    environment::use_libraries(&env);
    Ok(())
}

pub(crate) fn repo() -> Result<PathBuf, String> {
    crate::repository().ok_or_else(|| "only a Clarity run from its repository has DevTools".to_string())
}

/// What a build tells the page each time it changes.
pub(crate) fn on_change(app: &AppHandle) -> OnChange {
    let app = app.clone();
    std::sync::Arc::new(move |job: &Job| {
        let _ = app.emit(JOB_CHANGED, JobView::from(job));
    })
}

/// DevTools; none in an installed Clarity, which has none.
#[tauri::command]
pub fn devtools_view(jobs: State<DevJobs>) -> Option<DevToolsView> {
    crate::repository().map(|repo| view(&repo, &jobs.0))
}

/// Swaps a piece to the cDdev's own build (`mine`) or back to the release.
#[tauri::command(async)]
pub fn devtools_swap(key: String, mine: bool, jobs: State<DevJobs>) -> Result<DevToolsView, String> {
    let repo = repo()?;
    let mut swaps = Environment::current().swaps().clone();
    swaps.swap(&key, mine)?;
    apply(&repo, swaps)?;
    Ok(view(&repo, &jobs.0))
}

/// Adds the tab of a piece the cDdev knows has changes.
#[tauri::command(async)]
pub fn devtools_add_tab(key: String, jobs: State<DevJobs>) -> Result<DevToolsView, String> {
    let repo = repo()?;
    let mut swaps = Environment::current().swaps().clone();
    swaps.add(&key)?;
    apply(&repo, swaps)?;
    Ok(view(&repo, &jobs.0))
}

/// Closes an added tab: its piece is the release again.
#[tauri::command(async)]
pub fn devtools_close_tab(key: String, jobs: State<DevJobs>) -> Result<DevToolsView, String> {
    let repo = repo()?;
    let mut swaps = Environment::current().swaps().clone();
    swaps.close(&key)?;
    apply(&repo, swaps)?;
    Ok(view(&repo, &jobs.0))
}

/// The last lines of a build, for an agent's answer.
fn told(job: &Job) -> String {
    let tail: Vec<&str> = job.lines.iter().rev().take(40).rev().map(String::as_str).collect();
    let ended = match &job.state {
        JobState::Passed => format!("{} built.", job.title),
        JobState::Failed(why) => format!("{} failed: {why}", job.title),
        _ => format!("{} goes on.", job.title),
    };
    format!("{ended}\n\n{}", tail.join("\n"))
}

/// Presses a piece's Build: its build starts, or waits for the one before
/// it. `ask` is the agent's ask it answers, `by` the agent.
#[tauri::command]
pub fn devtools_build(key: String, by: Option<String>, ask: Option<String>, app: AppHandle, jobs: State<DevJobs>) -> Result<(), String> {
    let repo = repo()?;
    let piece = swap::piece(&key).ok_or_else(|| format!("there is no piece {key}"))?;
    let notify = on_change(&app);
    let after_repo = repo.clone();
    let answers = ask.clone();
    let ended: OnChange = std::sync::Arc::new(move |job: &Job| {
        notify(job);
        if !matches!(job.state, JobState::Passed | JobState::Failed(_)) {
            return;
        }
        // A swapped piece built again: the folder builds use is put together again from it.
        let swaps = Environment::current().swaps().clone();
        if job.state == JobState::Passed && swaps.mine(piece.key) {
            if let Err(why) = apply(&after_repo, swaps) {
                eprintln!("clarity: {why}");
            }
        }
        if let Some(id) = &ask {
            let _ = dev_asks::answer(&dev_asks::folder(), id, &dev_asks::Answer { ok: job.state == JobState::Passed, text: told(job) });
        }
    });
    let started = jobs.0.start_by(&job_key(piece.key), piece.name, swap::build(&repo, piece), by, Some(ended));
    // Refused (it builds already): the agent hears why.
    if let (Err(why), Some(id)) = (&started, &answers) {
        let _ = dev_asks::answer(&dev_asks::folder(), id, &dev_asks::Answer { ok: false, text: why.clone() });
    }
    started
}

/// Refuses an agent's ask: the person said no, or agents may not build.
#[tauri::command]
pub fn devtools_refuse(id: String, why: String) -> Result<(), String> {
    dev_asks::answer(&dev_asks::folder(), &id, &dev_asks::Answer { ok: false, text: why })
}

/// In a Clarity run from its repository, takes the agents' asks to press a
/// Build as they come and hands each to the page, which allows it as the
/// person chose for agents (devtoolsStore.svelte.ts).
pub fn watch_asks(app: AppHandle) {
    if crate::repository().is_none() {
        return;
    }
    std::thread::spawn(move || {
        let dir = dev_asks::folder();
        loop {
            for ask in dev_asks::pending(&dir) {
                if !dev_asks::take(&dir, &ask.id) {
                    continue;
                }
                match swap::piece(&ask.key) {
                    Some(piece) => {
                        let _ = app.emit(ASKED, BuildAsk { id: ask.id.clone(), key: ask.key.clone(), name: piece.name.to_string(), by: ask.by.clone() });
                    }
                    None => {
                        let _ = dev_asks::answer(&dir, &ask.id, &dev_asks::Answer { ok: false, text: format!("there is no piece {}", ask.key) });
                    }
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(700));
        }
    });
}
