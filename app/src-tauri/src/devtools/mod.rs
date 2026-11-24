//! DevTools (Clarity settings, a Clarity run from its repository only): the
//! cDdev's tools for working on Clarity itself. A tab per piece of Clarity
//! an app uses: the release or the cDdev's own build, swapped
//! (clarity_devtools::swap, kept in environment.json), and a
//! Build button that makes the cDdev's own build, shown in the tab while it
//! builds (jobs.rs, one build at a time). An agent presses Build through
//! Clarity's MCP (build_piece): its ask comes through a file
//! (clarity_project::dev_asks), and the page allows it as the person chose
//! for agents. Every build's state is written to its file beside the asks
//! as it changes (`dev_jobs`), which the agent follows with build_state. The release planning is release.rs, the Disk tab disk.rs. An installed Clarity has
//! no DevTools and always uses the release.

pub mod disk;
pub mod release;

use std::path::{Path, PathBuf};

use clarity_gen::clarity_ui;
use clarity_devtools::jobs::{Job, JobState, Jobs, OnChange};
use clarity_devtools::swap::{self, Kind, Piece, Swaps};
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

/// DevTools' builds; in a Clarity run from its repository each writes its
/// state to its file beside the agents' asks as it changes, for build_state.
pub fn dev_jobs() -> DevJobs {
    let jobs = DevJobs::default();
    if crate::repository().is_some() {
        jobs.0.record_in(&dev_asks::jobs(&dev_asks::folder()));
    }
    jobs
}

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
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PieceTab {
    pub key: String,
    pub name: String,
    /// What uses it, as people read it.
    pub uses: String,
    /// Its tab is always there.
    pub core: bool,
    /// Shown by itself because the cDdev's own build is behind its source,
    /// not added: it goes again once that build is as new as its source.
    pub auto: bool,
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
    /// What its Build debug runs, for a native library; none for the others.
    pub debug_build: Option<String>,
    /// Its debug build's key among the builds (`piece:clarityUi:debug`).
    pub debug_job: String,
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
    /// `build` a piece, `useOwn` or `useRelease` for its Swap, or `remove` a build folder of the Disk tab.
    pub action: String,
    pub key: String,
    pub name: String,
    pub by: String,
}

fn release_of(piece: &Piece) -> String {
    match piece.key {
        "rustCrates" => clarity_gen::rust_runtime_version().to_string(),
        "nugetPackages" => clarity_gen::csharp_runtime_version().to_string(),
        "goModule" => clarity_gen::go_runtime_version().to_string(),
        _ => clarity_ui::release_version().to_string(),
    }
}

fn own_of(repo: &Path, piece: &Piece, os: &str) -> String {
    let path = match piece.kind {
        Kind::Runtime { folder } => repo.join(folder),
        _ => swap::built_file(repo, piece, os).unwrap_or_else(|| repo.to_path_buf()),
    };
    clarity_project::slashed(&path.canonicalize().unwrap_or(path))
}

/// The tabs shown by themselves that the cDdev closed: away while Clarity runs, back at its next start.
fn hidden() -> std::sync::MutexGuard<'static, std::collections::BTreeSet<String>> {
    static HIDDEN: std::sync::Mutex<std::collections::BTreeSet<String>> = std::sync::Mutex::new(std::collections::BTreeSet::new());
    HIDDEN.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn view(repo: &Path, jobs: &Jobs) -> DevToolsView {
    let os = std::env::consts::OS;
    let env = Environment::current();
    let swaps = env.swaps();
    let tabs = swaps
        .shown(|p| swap::built_at(repo, p, os).is_some() && swap::behind(repo, p, os).is_some())
        .into_iter()
        .filter(|(piece, auto)| !auto || !hidden().contains(piece.key))
        .map(|(piece, auto)| PieceTab {
            key: piece.key.to_string(),
            name: piece.name.to_string(),
            uses: piece.uses.to_string(),
            core: piece.core,
            auto,
            mine: swaps.mine(piece.key),
            release: release_of(piece),
            own: own_of(repo, piece, os),
            built: swap::built_at(repo, piece, os).and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_millis() as u64),
            signal: swap::behind(repo, piece, os),
            build: swap::build(repo, piece).shown(),
            job: swap::job_key(piece.key, false),
            debug_build: swap::build_debug(repo, piece).map(|b| b.shown()),
            debug_job: swap::job_key(piece.key, true),
        })
        .collect::<Vec<_>>();
    let addable = swap::PIECES.iter().filter(|p| !tabs.iter().any(|t| t.key == p.key)).map(|p| PieceChoice { key: p.key.to_string(), name: p.name.to_string() }).collect();
    DevToolsView { tabs, addable, file: environment::file().display().to_string(), jobs: jobs.all().iter().map(JobView::from).collect() }
}

/// The release's folder of native libraries, fetched now when it is not on this machine yet.
fn release_libraries() -> Result<PathBuf, String> {
    let os = std::env::consts::OS;
    let cache = clarity_ui::cache_dir_in(os, |name| std::env::var(name).ok());
    let release = clarity_ui::library_in(None, None, cache.as_deref(), clarity_ui::rid(os, std::env::consts::ARCH).as_deref(), clarity_ui::release_version(), os);
    clarity_ui::provide(release, clarity_ui::RELEASES, &clarity_build::download, &|line| eprintln!("{line}"))
}

/// Takes `swaps`: the library choices they give are kept and used from now on.
/// `why` names what changed them; the IDE's log keeps each change, with the
/// swaps before and after, so a swap that went shows where it went.
fn apply(repo: &Path, swaps: Swaps, why: &str) -> Result<(), String> {
    let before = Environment::current().swaps().clone();
    if before != swaps {
        let own = |s: &Swaps| s.tabs().into_iter().filter(|k| s.mine(k)).collect::<Vec<_>>();
        crate::logging::line(&format!("[devtools] swaps by {why}: open {:?}, own {:?} -> open {:?}, own {:?}", before.tabs(), own(&before), swaps.tabs(), own(&swaps)));
    }
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
/// `ask` is the agent's ask it answers (swap_piece).
#[tauri::command(async)]
pub fn devtools_swap(key: String, mine: bool, ask: Option<String>, jobs: State<DevJobs>) -> Result<DevToolsView, String> {
    let swapped = swap_now(&key, mine, &jobs);
    if let Some(id) = &ask {
        let text = match &swapped {
            Ok(_) => format!("{key} now uses {}.", if mine { "the cDdev's own build" } else { "the release" }),
            Err(why) => why.clone(),
        };
        let _ = dev_asks::answer(&dev_asks::folder(), id, &dev_asks::Answer { ok: swapped.is_ok(), text });
    }
    swapped
}

fn swap_now(key: &str, mine: bool, jobs: &State<DevJobs>) -> Result<DevToolsView, String> {
    let key = key.to_string();
    let repo = repo()?;
    let mut swaps = Environment::current().swaps().clone();
    if !swaps.tabs().contains(&key.as_str()) {
        // A piece without a tab uses the release already; to its own build, its tab opens and stays.
        if !mine {
            return Ok(view(&repo, &jobs.0));
        }
        swaps.add(&key)?;
    }
    swaps.swap(&key, mine)?;
    apply(&repo, swaps, &format!("swap {key} to {}", if mine { "own" } else { "release" }))?;
    Ok(view(&repo, &jobs.0))
}

/// Adds the tab of a piece the cDdev knows has changes.
#[tauri::command(async)]
pub fn devtools_add_tab(key: String, jobs: State<DevJobs>) -> Result<DevToolsView, String> {
    let repo = repo()?;
    let mut swaps = Environment::current().swaps().clone();
    swaps.add(&key)?;
    hidden().remove(&key);
    apply(&repo, swaps, &format!("add tab {key}"))?;
    Ok(view(&repo, &jobs.0))
}

/// Closes a piece's tab: its piece is the release again. A tab shown by itself stays away until Clarity starts again or it is added.
#[tauri::command(async)]
pub fn devtools_close_tab(key: String, jobs: State<DevJobs>) -> Result<DevToolsView, String> {
    let repo = repo()?;
    let mut swaps = Environment::current().swaps().clone();
    swaps.close(&key)?;
    hidden().insert(key.clone());
    apply(&repo, swaps, &format!("close tab {key}"))?;
    Ok(view(&repo, &jobs.0))
}

/// Presses a piece's Build, or with `debug` its Build debug: its build
/// starts, or waits for the one before it. `ask` is the agent's ask: it is
/// answered only when the build is refused; once started, the agent
/// follows the build's record (build_state), never waiting for its end.
#[tauri::command]
pub fn devtools_build(key: String, debug: Option<bool>, by: Option<String>, ask: Option<String>, app: AppHandle, jobs: State<DevJobs>) -> Result<(), String> {
    let started = build_now(&key, debug.unwrap_or(false), by, &app, &jobs.0);
    // Refused (no such piece or debug build, it builds already): the agent hears why.
    if let (Err(why), Some(id)) = (&started, &ask) {
        let _ = dev_asks::answer(&dev_asks::folder(), id, &dev_asks::Answer { ok: false, text: why.clone() });
    }
    started
}

fn build_now(key: &str, debug: bool, by: Option<String>, app: &AppHandle, jobs: &Jobs) -> Result<(), String> {
    let repo = repo()?;
    let piece = swap::piece(key).ok_or_else(|| format!("there is no piece {key}"))?;
    let (job, name, run) = if debug {
        let run = swap::build_debug(&repo, piece).ok_or_else(|| format!("{} has no debug build", piece.name))?;
        (swap::job_key(piece.key, true), format!("{} (debug)", piece.name), run)
    } else {
        (swap::job_key(piece.key, false), piece.name.to_string(), swap::build(&repo, piece))
    };
    let notify = on_change(app);
    let after_repo = repo.clone();
    let ended: OnChange = std::sync::Arc::new(move |job: &Job| {
        notify(job);
        if !matches!(job.state, JobState::Passed | JobState::Failed(_)) {
            return;
        }
        // The library it built is as current as its sources, though Cargo left it untouched (swap::mark_built).
        if job.state == JobState::Passed {
            swap::mark_built(&after_repo, piece, debug, std::env::consts::OS);
        }
        // A swapped piece built again: the folder builds use is put together again from it (the release build only).
        let swaps = Environment::current().swaps().clone();
        if !debug && job.state == JobState::Passed && swaps.mine(piece.key) {
            if let Err(why) = apply(&after_repo, swaps, &format!("build of {} ended", piece.key)) {
                eprintln!("clarity: {why}");
            }
        }
    });
    jobs.start_by(&job, &name, run, by, Some(ended))
}

/// Refuses an agent's ask: the person said no, or agents may not build.
#[tauri::command]
pub fn devtools_refuse(id: String, why: String) -> Result<(), String> {
    dev_asks::answer(&dev_asks::folder(), &id, &dev_asks::Answer { ok: false, text: why })
}

/// In a Clarity run from its repository, takes the agents' asks to press a
/// Build or remove a build folder as they come and hands each to the page,
/// which allows it as the person chose for agents, a removal always asked
/// first (devtoolsStore.svelte.ts).
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
                let named = match ask.action {
                    dev_asks::Action::Build => swap::piece(&ask.key).map(|p| ("build", p.name.to_string())).ok_or_else(|| format!("there is no piece {}", ask.key)),
                    dev_asks::Action::BuildDebug => swap::piece(&ask.key).map(|p| ("buildDebug", format!("{} (debug)", p.name))).ok_or_else(|| format!("there is no piece {}", ask.key)),
                    dev_asks::Action::Swap => swap::piece(&ask.key)
                        .map(|p| (if ask.mine { "useOwn" } else { "useRelease" }, p.name.to_string()))
                        .ok_or_else(|| format!("there is no piece {}", ask.key)),
                    dev_asks::Action::Remove => match repo() {
                        Ok(repo) => clarity_devtools::disk::places(&repo)
                            .into_iter()
                            .find(|p| p.key == ask.key)
                            .map(|p| ("remove", format!("{}, {}. {}", p.name, clarity_devtools::disk::bytes_text(clarity_devtools::disk::size(&p)), p.cost)))
                            .ok_or_else(|| format!("there is no build folder {}", ask.key)),
                        Err(why) => Err(why),
                    },
                };
                match named {
                    Ok((action, name)) => {
                        let _ = app.emit(ASKED, BuildAsk { id: ask.id.clone(), action: action.into(), key: ask.key.clone(), name, by: ask.by.clone() });
                    }
                    Err(why) => {
                        let _ = dev_asks::answer(&dir, &ask.id, &dev_asks::Answer { ok: false, text: why });
                    }
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(700));
        }
    });
}
