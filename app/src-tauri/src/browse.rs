//! Clarity's own file browser over the backend's listing (docs/PROJECT-BROWSER-PLAN.md):
//! the window asks what a folder holds, where a use starts, and the projects
//! opened last; the backend lists and tells Clarity's projects apart.

use std::path::{Path, PathBuf};

use clarity_project::browse::{Listing, ProjectInfo, Show};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// What a use of the browser can pick, as the window asks for it.
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase", tag = "kind")]
#[ts(export)]
pub enum BrowseShow {
    Everything,
    Folders,
    /// Files of these kinds, by extension without the dot.
    Files { kinds: Vec<String> },
}

/// Where browsing starts, and the places beside the list.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BrowsePlaces {
    /// The folder the use was left in, else the one asked for, else Home.
    pub start: String,
    pub home: Option<String>,
    /// The projects folder of Clarity settings.
    pub projects: Option<String>,
    /// The apps folder of Clarity settings.
    pub apps: Option<String>,
    /// `/`, or the drives on Windows.
    pub roots: Vec<String>,
}

/// A folder as the browser lists it.
#[tauri::command]
pub fn browse_list(folder: String, show: BrowseShow, hidden: bool) -> Listing {
    let show = match show {
        BrowseShow::Everything => Show::Everything,
        BrowseShow::Folders => Show::Folders,
        BrowseShow::Files { kinds } => Show::Files(kinds),
    };
    clarity_project::browse::list(Path::new(&folder), &show, hidden)
}

/// Where the use `purpose` starts: where it was left, else `asked` (the folder the caller shows now), else Home.
#[tauri::command]
pub fn browse_places(purpose: String, asked: Option<String>) -> BrowsePlaces {
    let shown = |p: PathBuf| p.display().to_string();
    let folders = clarity_project::Folders::current();
    let home = clarity_project::browse::home();
    let asked = asked.map(PathBuf::from).filter(|p| p.is_dir());
    let start = clarity_project::recent::last_folder(&purpose).or(asked).or_else(|| home.clone()).map(shown).unwrap_or_default();
    BrowsePlaces {
        start,
        home: home.map(shown),
        projects: folders.projects.map(shown),
        apps: folders.output.map(shown),
        roots: clarity_project::browse::roots().into_iter().map(shown).collect(),
    }
}

/// Remembers the folder the use `purpose` was left in.
#[tauri::command]
pub fn browse_remember(purpose: String, folder: String) -> Result<(), String> {
    clarity_project::recent::remember_folder(&purpose, Path::new(&folder))
}

/// The projects opened last, newest first, as the browser tells them.
#[tauri::command]
pub fn recent_projects() -> Vec<ProjectInfo> {
    clarity_project::recent::recent().iter().filter_map(|p| clarity_project::browse::project_info(p)).collect()
}
