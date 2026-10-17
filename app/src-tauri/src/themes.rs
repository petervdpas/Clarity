//! The CSS of the project's themes for the designer: the same files the
//! generator writes into the application (`ui/themes/*.css`), so the canvas
//! shows a theme as the application will.

use tauri::State;

use super::{WatchState, open_project_of};

/// Every theme of the open project as CSS, one file after the other.
#[tauri::command]
pub fn theme_styles(state: State<WatchState>) -> Result<String, String> {
    let (_, dir) = open_project_of(&state)?;
    let project = clarity_project::load_modules(&dir).map_err(|err| format!("cannot read {}: {err}", dir.display()))?;
    Ok(project.files.iter().flat_map(|f| &f.themes).map(clarity_gen::theme_css).collect::<Vec<_>>().join("\n"))
}
