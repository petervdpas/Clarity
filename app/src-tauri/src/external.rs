//! Importing tables of an external database on the Project tab: the
//! tables a source has, and writing the chosen ones as a module.

use clarity_project::SourceTable;
use clarity_spec::Preview;
use serde::Serialize;
use tauri::State;
use ts_rs::TS;

use super::{FileChange, HistoryState, WatchState, load, open_project_of, record};

/// The tables of external data source `source`.
#[tauri::command]
pub fn source_tables(source: String, state: State<WatchState>) -> Result<Vec<SourceTable>, String> {
    let (_, dir) = open_project_of(&state)?;
    clarity_project::source_tables(&dir, &source)
}

/// A table that was left out of an import, and why.
#[derive(Serialize, TS)]
#[ts(export)]
pub struct Skipped {
    pub table: String,
    pub reason: String,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct Imported {
    pub preview: Preview,
    /// The module file that was written.
    pub file: String,
    pub skipped: Vec<Skipped>,
}

/// Writes `tables` of `source` as module `module`. Undo removes the file.
#[tauri::command]
pub fn import_tables(
    source: String,
    module: String,
    tables: Vec<String>,
    state: State<WatchState>,
    history: State<HistoryState>,
) -> Result<Imported, String> {
    let (_, dir) = open_project_of(&state)?;
    let done = clarity_project::import_tables(&dir, &source, module.trim(), &tables)?;
    record(&history, vec![FileChange { file: done.path.clone(), before: None, after: done.text }])?;
    Ok(Imported {
        preview: load(&dir)?,
        file: done.path.display().to_string(),
        skipped: done.skipped.into_iter().map(|(table, reason)| Skipped { table, reason }).collect(),
    })
}

/// Takes ownership of external data source `source`. Undo puts the
/// project file back and removes the baseline step.
#[tauri::command]
pub fn take_ownership(source: String, state: State<WatchState>, history: State<HistoryState>) -> Result<Preview, String> {
    let (_, dir) = open_project_of(&state)?;
    let owned = clarity_project::take_ownership(&dir, &source)?;
    record(
        &history,
        vec![
            FileChange { file: owned.step, before: None, after: owned.step_text },
            FileChange { file: owned.project_file, before: Some(owned.before), after: owned.after },
        ],
    )?;
    load(&dir)
}
