//! DevTools' release planning tab: how each of Clarity's releases stands
//! (read from the repository, clarity_gen::tools::devtools::release), its
//! Bump, the checks a release needs first (run as builds, one at a time,
//! beside the pieces'), and Write plan, which writes the Release Helper's
//! plan into releases/. A failing or unrun check holds the plan until the
//! person decides to write it anyway.

use clarity_gen::tools::devtools::jobs::JobState;
use clarity_gen::tools::devtools::release::{self, Out, Planned};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};
use ts_rs::TS;

use super::{DevJobs, on_change, repo};

/// A release, as the tab shows it.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct ReleaseRow {
    pub key: String,
    pub name: String,
    /// Its newest tag, and the version that released.
    pub tag: Option<String>,
    pub released: Option<String>,
    /// The version in source; none when the tag alone names it.
    pub source: Option<String>,
    /// The files changed since its tag.
    pub changed: Vec<String>,
    /// The version it would release.
    pub next: String,
    /// The version in source is past the released one.
    pub bumped: bool,
    /// It has a version in source to bump.
    pub bumpable: bool,
}

/// A check a release needs first.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct CheckRow {
    /// Its build's key among the builds (`check:guard:test:...`).
    pub job: String,
    pub title: String,
    pub command: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct ReleaseView {
    pub rows: Vec<ReleaseRow>,
    pub checks: Vec<CheckRow>,
    /// The IDE's last release, which the checks count changes from.
    pub since: Option<String>,
}

/// A release the person put in the plan.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
pub struct PlanRelease {
    pub key: String,
    pub version: String,
    pub note: String,
}

fn job_key(check: &str) -> String {
    format!("check:{check}")
}

/// The files changed since the IDE's last release, and that release.
fn changed(repo: &std::path::Path) -> Result<(Vec<String>, Option<String>), String> {
    let since = release::newest(&release::tags(repo)?, "v");
    let files = match &since {
        Some(tag) => release::changed_since(repo, tag, &["."])?,
        None => Vec::new(),
    };
    Ok((files, since))
}

fn view() -> Result<ReleaseView, String> {
    let repo = repo()?;
    let rows = release::standing(&repo)?
        .into_iter()
        .map(|s| ReleaseRow { key: s.key, name: s.name, tag: s.tag, released: s.released, source: s.source, changed: s.changed, next: s.next, bumped: s.bumped, bumpable: s.bumpable })
        .collect();
    let (files, since) = changed(&repo)?;
    let checks = release::checks(&repo, &files).into_iter().map(|c| CheckRow { job: job_key(&c.key), title: c.title, command: c.run.shown() }).collect();
    Ok(ReleaseView { rows, checks, since })
}

/// The release planning tab.
#[tauri::command(async)]
pub fn release_view() -> Result<ReleaseView, String> {
    view()
}

/// Bumps a release to its next minor version, and the crates naming it.
#[tauri::command(async)]
pub fn release_bump(key: String) -> Result<ReleaseView, String> {
    let repo = repo()?;
    let unit = release::unit(&key).ok_or_else(|| format!("there is no release {key}"))?;
    release::bump(&repo, unit)?;
    view()
}

/// Runs the checks `jobs` (their builds' keys), one after the other.
#[tauri::command(async)]
pub fn release_run(jobs: Vec<String>, app: AppHandle, builds: State<DevJobs>) -> Result<(), String> {
    let repo = repo()?;
    let (files, _) = changed(&repo)?;
    let checks = release::checks(&repo, &files);
    let mut refused = Vec::new();
    for key in jobs {
        let Some(check) = checks.iter().find(|c| job_key(&c.key) == key) else {
            refused.push(format!("there is no check {key}"));
            continue;
        };
        if let Err(why) = builds.0.start(&key, &check.title, check.run.clone(), Some(on_change(&app))) {
            refused.push(why);
        }
    }
    if refused.is_empty() { Ok(()) } else { Err(refused.join("; ")) }
}

/// Writes the plan of `releases` into releases/<stamp>.json; the file. Held
/// while a check has not passed, unless `anyway` (the person decided).
#[tauri::command(async)]
pub fn release_write_plan(title: String, message: String, stamp: String, releases: Vec<PlanRelease>, anyway: bool, builds: State<DevJobs>) -> Result<String, String> {
    let repo = repo()?;
    let (files, _) = changed(&repo)?;
    if !anyway {
        let open: Vec<String> = release::checks(&repo, &files)
            .into_iter()
            .filter(|c| builds.0.job(&job_key(&c.key)).is_none_or(|j| j.state != JobState::Passed))
            .map(|c| c.title)
            .collect();
        if !open.is_empty() {
            return Err(format!("{} checks have not passed: {}", open.len(), open.join(", ")));
        }
    }
    let standing = release::standing(&repo)?;
    let mut planned = Vec::new();
    for r in releases {
        let unit = release::unit(&r.key).ok_or_else(|| format!("there is no release {}", r.key))?;
        let row = standing.iter().find(|s| s.key == r.key);
        let from = row.and_then(|s| s.tag.clone());
        let assets = match (unit.out, &from, row.and_then(|s| s.released.as_deref())) {
            (Out::GithubRelease, Some(tag), Some(released)) => release::assets_for(&release::release_assets(&repo, tag)?, released, &r.version),
            _ => Vec::new(),
        };
        let packages = if unit.out == Out::Nuget { release::nuget_packages(&repo) } else { Vec::new() };
        planned.push(Planned { key: r.key, version: r.version, from, note: r.note, assets, packages });
    }
    let plan = release::plan(&title, &message, &planned)?;
    let file = release::write_plan(&repo, &stamp, &plan)?;
    Ok(file.display().to_string())
}
