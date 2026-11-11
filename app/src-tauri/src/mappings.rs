//! The mapping page (docs/FORMATS-PLAN.md, step 4): the project's mappings
//! as forms, saving one back into its module through the printer as one
//! designer edit (checked, written, one Undo), deleting one that nothing
//! uses, and a sample file laid over one, read through Clarity.Formats as a
//! program reads it.

use std::path::Path;

use clarity_project::mappings::{MappingEntry, MappingPreview};
use clarity_spec::code::mapping::{MappingForm, text};
use clarity_spec::edit::Edit;
use tauri::State;

use super::{EditApplied, HistoryState, WatchState, apply_edit, open_dir_of};

/// The rows a preview shows of each table.
const PREVIEW_ROWS: usize = 50;

/// Every mapping of the open project, in file order.
#[tauri::command]
pub fn list_mappings(state: State<WatchState>) -> Result<Vec<MappingEntry>, String> {
    clarity_project::mappings::mappings(&open_dir_of(&state)?)
}

/// The mapping called `name`: where it is declared and its form.
#[tauri::command]
pub fn read_mapping(name: String, state: State<WatchState>) -> Result<MappingEntry, String> {
    clarity_project::mappings::mapping(&open_dir_of(&state)?, &name)
}

/// Writes `form` into module file `path` (at `version`, as the page saw it):
/// in place of the declaration at `item`, or at the end of the file for a
/// new mapping.
#[tauri::command]
pub fn save_mapping(path: String, version: String, item: Option<u32>, form: MappingForm, state: State<WatchState>, history: State<HistoryState>) -> Result<EditApplied, String> {
    let written = text(&form);
    let edit = match item {
        Some(item) => Edit::ReplaceDecl { item, text: written },
        None => Edit::AppendDecl { text: written },
    };
    apply_edit(path, version, edit, state, history)
}

/// The first records of `sample` (a file picked in Clarity's file browser)
/// laid over mapping `name`, or why the file does not fit it.
#[tauri::command]
pub fn preview_mapping(name: String, sample: String, state: State<WatchState>) -> Result<MappingPreview, String> {
    clarity_project::mappings::preview_mapping(&open_dir_of(&state)?, &name, Path::new(&sample), PREVIEW_ROWS)
}

/// Where mapping `name` is used: data sources laid over it and code reading
/// or writing through it, each as a person reads it.
#[tauri::command]
pub fn mapping_uses(name: String, state: State<WatchState>) -> Result<Vec<String>, String> {
    clarity_project::mappings::mapping_uses(&open_dir_of(&state)?, &name)
}

/// Deletes mapping `name` from its module as one edit (one Undo); refused,
/// naming where, while a data source or code still uses it.
#[tauri::command]
pub fn delete_mapping(name: String, state: State<WatchState>, history: State<HistoryState>) -> Result<EditApplied, String> {
    let dir = open_dir_of(&state)?;
    let uses = clarity_project::mappings::mapping_uses(&dir, &name)?;
    if !uses.is_empty() {
        return Err(format!("mapping {name} is used by {}; change those first", uses.join(", ")));
    }
    let entry = clarity_project::mappings::mapping(&dir, &name)?;
    let source = std::fs::read_to_string(&entry.path).map_err(|err| format!("cannot read {}: {err}", entry.path))?;
    let version = clarity_spec::content_version(&source);
    apply_edit(entry.path, version, Edit::Remove { item: entry.item }, state, history)
}
