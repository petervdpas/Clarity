//! The CSS of the project's themes for the designer: the same files the
//! generator writes into the application (`ui/themes/*.css`), so the canvas
//! shows a theme as the application will.

use tauri::State;

use super::{WatchState, open_project_of};

/// Clarity's own theme and every theme of the open project as CSS.
#[tauri::command]
pub fn theme_styles(state: State<WatchState>) -> Result<String, String> {
    let (_, dir) = open_project_of(&state)?;
    let project = clarity_project::load_modules(&dir).map_err(|err| format!("cannot read {}: {err}", dir.display()))?;
    let themes = project.files.iter().flat_map(|f| &f.themes).map(clarity_gen::theme_css);
    // Clarity's own theme first: the base the others change.
    Ok(std::iter::once(clarity_gen::base_css()).chain(themes).collect::<Vec<_>>().join("\n"))
}

/// The text of a new theme called `name`, starting from one of Clarity's own
/// looks (light and dark as the system is, when none is given), so every
/// value a theme can set is there. With `Clarity` it is Clarity's own theme.
#[tauri::command]
pub fn theme_starter(name: String, start: Option<clarity_spec::themes::ThemeStart>) -> String {
    clarity_spec::themes::starter_from(name.trim(), start.unwrap_or(clarity_spec::themes::ThemeStart::System))
}
