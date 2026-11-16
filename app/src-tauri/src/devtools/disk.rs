//! DevTools' Disk tab: the folders Clarity's builds and checks fill in its
//! repository (clarity_gen::tools::devtools::disk), how big each is, what
//! removing it costs, and the free space of the disk the repository is on,
//! with a warning below 40 GB. Removing waits until no build runs.

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
    let places = disk::places(&repo).into_iter().map(|p| DiskPlace { key: p.key.into(), name: p.name.into(), cost: p.cost.into(), bytes: disk::size(&p) }).collect();
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
        disk::remove(&repo()?, &key)
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
