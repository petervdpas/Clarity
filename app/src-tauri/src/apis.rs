//! The web APIs of the open project (DESIGN 5.23): what a description holds
//! and what the project picks of it, and a description brought into the
//! project's apis folder. Clarity writes each API's module from them when
//! the project loads and whenever its file or a description changes.

use std::path::PathBuf;

use tauri::State;

use super::{WatchState, project_file_of};

/// Every operation and schema of the description data source `source`
/// names, with what the project picks.
#[tauri::command]
pub fn api_listing(source: String, state: State<WatchState>) -> Result<clarity_project::ApiListing, String> {
    clarity_project::api_listing(&project_file_of(&state)?, &source)
}

/// Copies the description at `from` into the project's apis folder; gives
/// its name there, which the data source's `openapi` line then names.
#[tauri::command]
pub fn add_api_file(from: String, state: State<WatchState>) -> Result<String, String> {
    clarity_project::add_api_file(&project_file_of(&state)?, &PathBuf::from(from))
}
