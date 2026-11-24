//! DevTools' release planning tab, for reviewing: how each of Clarity's
//! releases stands (read from the repository, clarity_devtools::release),
//! its changes sorted by kind, the chain of steps releasing the ones whose
//! code changed would start, and the checks a release needs first (run as
//! builds, one at a time, beside the pieces'). How each check ends is kept
//! (target/release-checks.json), where the agent reads it through MCP
//! (`release_state`) to write the Release Helper's plan. The tab writes no plan.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use clarity_devtools::jobs::{Job, JobState, OnChange};
use clarity_devtools::release::{self, Check, CheckRecord};
use serde::Serialize;
use tauri::{AppHandle, State};
use ts_rs::TS;

use super::{DevJobs, on_change, repo};

/// A release, for review: its tag, its version in source and what changed since, by kind.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct ReleaseRow {
    pub key: String,
    pub name: String,
    /// Its newest tag; none when it was never released.
    pub tag: Option<String>,
    /// The version in source; none when the tag alone names it (the IDE).
    pub source: Option<String>,
    pub code: Vec<String>,
    pub manifest: Vec<String>,
    pub tests: Vec<String>,
    pub docs: Vec<String>,
}

/// A step a release would start: the checks on main, a tag with its workflow, or a step by hand.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ChainStep {
    pub title: String,
    pub tag: Option<String>,
    pub workflow: Option<String>,
    /// Where it shows once its workflow passed.
    pub shows: Option<String>,
    /// The steps before it that it needs.
    pub waits_for: Vec<String>,
    /// The next step may go once its workflow passed.
    pub next_after_workflow: bool,
}

/// How a check ended the last time it ran, kept across starts.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct CheckLast {
    pub passed: bool,
    pub why: Option<String>,
    /// When it ended, in milliseconds since 1970.
    #[ts(type = "number")]
    pub ended: u64,
    /// It ran on the tree as it is now; a result of another tree counts as not run.
    pub current: bool,
}

/// A check a release needs first.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct CheckRow {
    /// Its build's key among the builds (`check:guard:test:...`).
    pub job: String,
    pub title: String,
    pub command: String,
    pub last: Option<CheckLast>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct ReleaseView {
    pub releases: Vec<ReleaseRow>,
    pub chain: Vec<ChainStep>,
    pub checks: Vec<CheckRow>,
    /// The IDE's last release, which the checks count changes from.
    pub since: Option<String>,
}

const CHECK: &str = "check:";

fn job_key(check: &str) -> String {
    format!("{CHECK}{check}")
}

fn view() -> Result<ReleaseView, String> {
    let repo = repo()?;
    let standing = release::standing(&repo)?;
    let chain = release::chain(&repo, &standing)
        .into_iter()
        .map(|e| ChainStep { title: e.title, tag: e.tag, workflow: e.workflow, shows: e.shows, waits_for: e.waits_for, next_after_workflow: e.next_after_workflow })
        .collect();
    let releases = standing
        .into_iter()
        .map(|s| ReleaseRow { key: s.key, name: s.name, tag: s.tag, source: s.source, code: s.changes.code, manifest: s.changes.manifest, tests: s.changes.tests, docs: s.changes.docs })
        .collect();
    let (files, since) = release::since_ide_release(&repo)?;
    let checks = release::check_standings(&repo, &files)
        .into_iter()
        .map(|s| {
            let last = s.record.map(|r| CheckLast { passed: r.passed, why: r.why, ended: r.ended, current: s.counted.current() });
            CheckRow { job: job_key(&s.check.key), title: s.check.title, command: s.check.run.shown(), last }
        })
        .collect();
    Ok(ReleaseView { releases, chain, checks, since })
}

/// The release planning tab.
#[tauri::command(async)]
pub fn release_view() -> Result<ReleaseView, String> {
    view()
}

/// What a check's build says as it goes, and when it ends, how it ended kept
/// for the agent, with the fingerprint of what it read: of the tree as it was
/// when the check started, or for a check in the fresh checkout, of the
/// tree the checkout was made from.
fn recording(app: &AppHandle, repo: PathBuf, check: Check) -> OnChange {
    let notify = on_change(app);
    let ran_on: Mutex<Option<(String, String)>> = Mutex::new(None);
    Arc::new(move |job: &Job| {
        notify(job);
        let Ok(mut ran_on) = ran_on.lock() else { return };
        if job.state == JobState::Running && ran_on.is_none() {
            let checkout = release::fresh_checkout_dir(&repo.canonicalize().unwrap_or_else(|_| repo.clone()));
            let (state, head) = if check.fresh { (release::checkout_state(&checkout), release::head(&checkout)) } else { (release::tree_state(&repo), release::head(&repo)) };
            *ran_on = Some((head.unwrap_or_default(), state.map(|s| s.fingerprint(&check)).unwrap_or_default()));
        }
        if let Some(record) = record_of(job, ran_on.as_ref()) {
            let _ = release::record_check(&repo, &record);
        }
    })
}

/// How a check's build ended, once it has, on the tree (`head`, `tree`) it
/// started on. A check stopped before it started is not kept.
fn record_of(job: &Job, ran_on: Option<&(String, String)>) -> Option<CheckRecord> {
    let key = job.key.strip_prefix(CHECK)?;
    let (passed, why) = match &job.state {
        JobState::Passed => (true, None),
        JobState::Failed(why) => (false, Some(why.clone())),
        JobState::Waiting | JobState::Running => return None,
    };
    let (head, tree) = ran_on?.clone();
    let lines = if passed { Vec::new() } else { job.lines.iter().skip(job.lines.len().saturating_sub(30)).cloned().collect() };
    Some(CheckRecord { key: key.to_string(), title: job.title.clone(), passed, why, ended: job.ended.unwrap_or_default(), head, tree, lines })
}

/// Runs the checks `jobs` (their builds' keys) that do not count yet (failed,
/// not run, or what they read changed since), one after the other.
#[tauri::command(async)]
pub fn release_run(jobs: Vec<String>, app: AppHandle, builds: State<DevJobs>) -> Result<(), String> {
    let repo = repo()?;
    let (files, _) = release::since_ide_release(&repo)?;
    let checks = release::check_standings(&repo, &files);
    let mut refused = Vec::new();
    for key in jobs {
        let Some(standing) = checks.iter().find(|s| job_key(&s.check.key) == key) else {
            refused.push(format!("there is no check {key}"));
            continue;
        };
        let check = &standing.check;
        // A check that passed on what it reads now still counts: it is not run again.
        // The checkout, the one build and the clean tree always run: they are cheap when nothing changed.
        if check.inputs != release::Inputs::Whole && standing.counted == release::Counted::Passed {
            continue;
        }
        if let Err(why) = builds.0.start(&key, &check.title, check.run.clone(), Some(recording(&app, repo.clone(), check.clone()))) {
            refused.push(why);
        }
    }
    if refused.is_empty() { Ok(()) } else { Err(refused.join("; ")) }
}

/// Stops the release checks: the one running ends now, the waiting ones never start.
#[tauri::command(async)]
pub fn release_stop(builds: State<DevJobs>) -> usize {
    let keys: Vec<String> = builds.0.all().into_iter().map(|job| job.key).filter(|key| key.starts_with(CHECK)).collect();
    builds.0.stop(&keys)
}
