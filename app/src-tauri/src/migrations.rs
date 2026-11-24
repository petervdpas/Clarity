//! The Database section of the Project tab: the changes in the data
//! dictionary that no migration step covers yet, the renames Clarity
//! proposes for the person to pick, and writing them as the next step
//! through clarity_tools::steps, as the agents and `clarity migrate` do.

use crate::fail_safe::Held;
use clarity_spec::Preview;
use clarity_spec::migrate::{self, Rename};
use clarity_tools::steps;
use serde::Serialize;
use tauri::{AppHandle, State};
use ts_rs::TS;

use super::{HistoryState, WatchState, load, open_project_of, record, watch};

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
    /// Renames Clarity proposes for the person to pick: a name that went and one that came.
    pub proposed: Vec<Rename>,
    /// How many steps the project has.
    pub steps: usize,
    /// Why no step can be written now, such as errors in the project.
    pub blocked: Option<String>,
}

/// The pending changes of data source `source`, with `renames` (the ones
/// the person picked) taken as renames; the renames Clarity proposes are
/// listed for the person to pick, never taken unasked.
#[tauri::command]
pub fn pending_migration(source: String, renames: Option<Vec<Rename>>, state: State<WatchState>) -> Result<PendingMigration, String> {
    let (_, dir) = open_project_of(&state)?;
    let project = clarity_project::load_modules(&dir).map_err(|err| format!("cannot read {}: {err}", dir.display()))?;
    let renames = renames.unwrap_or_default();
    let pending = steps::pending(&project, Some(&source), &renames)?.pop().ok_or_else(|| format!("data source {source} has no migration steps"))?;
    Ok(PendingMigration {
        changes: pending
            .ops
            .iter()
            .map(|op| PendingChange {
                text: migrate::op_text(op),
                loses_data: op.loses_data(),
            })
            .collect(),
        proposed: pending.proposed,
        steps: pending.steps,
        source,
        blocked: steps::blocked(&project),
    })
}

/// Writes the pending changes of `source` as its next step, the one way
/// (clarity_tools::steps::write_steps): `renames` are the ones the person
/// picked, and changes that can lose data are written once the person
/// checked them (`allow_data_loss`). Undo removes the step again.
#[tauri::command]
pub fn create_migration(
    source: String,
    title: String,
    renames: Vec<Rename>,
    allow_data_loss: Option<bool>,
    app: AppHandle,
    state: State<WatchState>,
    history: State<HistoryState>,
) -> Result<Preview, String> {
    let (file, dir) = open_project_of(&state)?;
    let had_folder = dir.join(clarity_spec::MIGRATIONS_DIR).join(&source).is_dir();
    let request = steps::StepRequest { source: Some(&source), title: Some(&title), renames: &renames, allow_data_loss: allow_data_loss.unwrap_or(false) };
    let migrated = steps::write_steps(&dir, &request)?;
    if migrated.written.is_empty() {
        return Err(migrated.said);
    }
    record(&history, migrated.written)?;
    // The folder is new: watch it from now on, like the modules.
    if !had_folder {
        let watcher = watch(&file, &dir, app)?;
        if let Some(open) = state.open.held().map_err(|_| "watch state is poisoned")?.as_mut() {
            open.watcher = watcher;
        }
    }
    load(&dir)
}
