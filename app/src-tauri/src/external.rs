//! Importing tables of an external database on the Project tab: the
//! tables a source has, and writing the chosen ones as a module.

use clarity_project::SourceTable;
use clarity_spec::Preview;
use serde::Serialize;
use tauri::State;
use ts_rs::TS;

use super::{FileChange, HistoryState, WatchState, load, open_project_of, record};

/// The tables of external data source `source`; for an XML file source,
/// those of its XSD `schema` when one is given (relative to the project's folder).
#[tauri::command]
pub fn source_tables(source: String, schema: Option<String>, state: State<WatchState>) -> Result<Vec<SourceTable>, String> {
    let (_, dir) = open_project_of(&state)?;
    clarity_project::source_tables(&dir, &source, schema.as_deref().map(std::path::Path::new))
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

/// Writes `tables` of `source` as module `module`. Undo removes the file,
/// and puts back the project file where the import set a file source's `at`.
#[tauri::command]
pub fn import_tables(
    source: String,
    module: String,
    tables: Vec<String>,
    schema: Option<String>,
    state: State<WatchState>,
    history: State<HistoryState>,
) -> Result<Imported, String> {
    let (_, dir) = open_project_of(&state)?;
    let done = clarity_project::import_tables(&dir, &source, module.trim(), &tables, schema.as_deref().map(std::path::Path::new))?;
    let mut changes = vec![FileChange { file: done.path.clone(), before: None, after: Some(done.text) }];
    if let Some((file, before, after)) = done.project_file {
        changes.push(FileChange { file, before: Some(before), after: Some(after) });
    }
    record(&history, changes)?;
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
            FileChange { file: owned.step, before: None, after: Some(owned.step_text) },
            FileChange { file: owned.project_file, before: Some(owned.before), after: Some(owned.after) },
        ],
    )?;
    load(&dir)
}
