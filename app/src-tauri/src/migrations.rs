//! The Database section of the Project tab: the changes in the data
//! dictionary that no migration step covers yet, the renames Clarity
//! proposes, and writing them as the next step.

use crate::fail_safe::Held;
use clarity_spec::migrate::{self, Rename};
use clarity_spec::{Preview, Severity};
use serde::Serialize;
use tauri::{AppHandle, State};
use ts_rs::TS;

use super::{FileChange, HistoryState, WatchState, load, open_project_of, record, watch};

/// One change that is not in a step yet.
#[derive(Serialize, TS)]
#[ts(export)]
pub struct PendingChange {
    /// As the step will say it: `add field Customer.phone: text(20)`.
    pub text: String,
    /// Removing a field or an entity, or changing a type, can lose data.
    #[serde(rename = "losesData")]
    #[ts(rename = "losesData")]
    pub loses_data: bool,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct PendingMigration {
    /// The data source whose steps these are.
    pub source: String,
    /// The changes, with the renames that were given taken as renames.
    pub changes: Vec<PendingChange>,
    /// Renames Clarity proposes: a name that went and one that came.
    pub proposed: Vec<Rename>,
    /// How many steps the project has.
    pub steps: usize,
    /// Why no step can be written now, such as errors in the project.
    pub blocked: Option<String>,
}

fn blocked(project: &clarity_spec::Project) -> Option<String> {
    project
        .diagnostics
        .iter()
        .find(|d| d.severity == Severity::Error)
        .map(|d| format!("fix the errors first: {}", d.message))
}

/// The pending changes of data source `source`, with `renames` taken as
/// renames; the proposed renames when `renames` is not given.
#[tauri::command]
pub fn pending_migration(
    source: String,
    renames: Option<Vec<Rename>>,
    state: State<WatchState>,
) -> Result<PendingMigration, String> {
    let (_, dir) = open_project_of(&state)?;
    let project = clarity_project::load_modules(&dir).map_err(|err| format!("cannot read {}: {err}", dir.display()))?;
    let proposed = migrate::proposed_renames(&project, &source);
    let renames = renames.unwrap_or_else(|| proposed.clone());
    let changes = migrate::pending(&project, &source, &renames)
        .iter()
        .map(|op| PendingChange {
            text: migrate::op_text(op),
            loses_data: op.loses_data(),
        })
        .collect();
    Ok(PendingMigration {
        changes,
        proposed,
        steps: project.migrations.as_ref().and_then(|m| m.get(&source)).map_or(0, Vec::len),
        source,
        blocked: blocked(&project),
    })
}

/// Writes the pending changes of `source` as its next step. Undo removes
/// it again.
#[tauri::command]
pub fn create_migration(
    source: String,
    title: String,
    renames: Vec<Rename>,
    app: AppHandle,
    state: State<WatchState>,
    history: State<HistoryState>,
) -> Result<Preview, String> {
    let (file, dir) = open_project_of(&state)?;
    let project = clarity_project::load_modules(&dir).map_err(|err| format!("cannot read {}: {err}", dir.display()))?;
    if let Some(reason) = blocked(&project) {
        return Err(reason);
    }
    let title = title.trim();
    if title.is_empty() {
        return Err("give the step a title".into());
    }
    let ops = migrate::pending(&project, &source, &renames);
    let had_folder = dir.join(clarity_spec::MIGRATIONS_DIR).join(&source).is_dir();
    let path = clarity_spec::write_step(&dir, &source, title, &ops).map_err(|err| format!("cannot write the step: {err}"))?;
    let text = std::fs::read_to_string(&path).map_err(|err| format!("cannot read {}: {err}", path.display()))?;
    record(&history, vec![FileChange { file: path, before: None, after: Some(text) }])?;
    // The folder is new: watch it from now on, like the modules.
    if !had_folder {
        let watcher = watch(&file, &dir, app)?;
        if let Some(open) = state.open.held().map_err(|_| "watch state is poisoned")?.as_mut() {
            open.watcher = watcher;
        }
    }
    load(&dir)
}
