//! DevTools' Disk tab: the folders Clarity's builds and checks fill in its
//! repository (clarity_gen::tools::devtools::disk), how big each is, what
//! removing it costs, and the free space of the disk the repository is on,
//! with a warning below 40 GB. Removing waits until no build runs. A folder
//! the running IDE holds (on Windows its own files) is marked for the next
//! start instead, which dev.sh and dev.bat remove before anything builds.

use clarity_gen::tools::devtools::disk;
use clarity_gen::tools::devtools::jobs::JobState;
use clarity_project::dev_asks;
use serde::Serialize;
use tauri::State;
use ts_rs::TS;

use super::{DevJobs, repo};

/// A folder builds fill, as the tab shows it.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct DiskPlace {
    pub key: String,
    pub name: String,
    pub cost: String,
    #[ts(type = "number")]
    pub bytes: u64,
    /// Marked to go at the next start of the IDE.
    pub marked: bool,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct DiskView {
    pub places: Vec<DiskPlace>,
    /// The free space of the disk the repository is on; none when unknown.
    #[ts(type = "number | null")]
    pub free: Option<u64>,
    /// Free space is below what builds and checks need.
    pub low: bool,
}

fn view() -> Result<DiskView, String> {
    let repo = repo()?;
    let marked = disk::marked(&repo);
    let places = disk::places(&repo)
        .into_iter()
        .map(|p| DiskPlace { key: p.key.into(), name: p.name.into(), cost: p.cost.into(), bytes: disk::size(&p), marked: marked.contains(&p.key) })
        .collect();
    let free = disk::free_space(&repo);
    Ok(DiskView { places, free, low: free.is_some_and(|f| f < disk::ROOM) })
}

/// The Disk tab.
#[tauri::command(async)]
pub fn disk_view() -> Result<DiskView, String> {
    view()
}

/// Removes the place `key`; refused while a build runs or waits, which may
/// need it. For an agent's ask (`ask`), the agent hears how it went.
#[tauri::command(async)]
pub fn disk_remove(key: String, ask: Option<String>, jobs: State<DevJobs>) -> Result<DiskView, String> {
    let done = (|| {
        if jobs.0.all().iter().any(|j| matches!(j.state, JobState::Waiting | JobState::Running)) {
            return Err("a build is running: remove once it ended".to_string());
        }
        disk::remove(&repo()?, &key).map_err(|why| format!("{why}. A folder the running Clarity holds goes with At next start, when dev.sh or dev.bat starts it again."))
    })();
    if let Some(id) = &ask {
        let answer = match &done {
            Ok(freed) => dev_asks::Answer { ok: true, text: format!("removed, {} freed", disk::bytes_text(*freed)) },
            Err(why) => dev_asks::Answer { ok: false, text: why.clone() },
        };
        let _ = dev_asks::answer(&dev_asks::folder(), id, &answer);
    }
    done?;
    view()
}

/// Marks the place `key` to go at the next start, or takes the mark away:
/// dev.sh and dev.bat remove it before anything builds (app/scripts/clean-at-start.mjs).
#[tauri::command(async)]
pub fn disk_mark(key: String, marked: bool) -> Result<DiskView, String> {
    disk::mark(&repo()?, &key, marked)?;
    view()
}
