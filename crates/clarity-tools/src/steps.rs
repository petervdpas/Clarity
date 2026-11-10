//! The agent's migration steps (docs/MCP-PLAN.md): when a tool changes the
//! project's data model, the change goes into a step at once, so the model
//! and the steps are never out of step. While no database has applied the
//! agent's latest step, a change folds into it, so a task leaves one step.
//! A rename reaches a step only as the agent asked for it with the rename
//! tool, never guessed: a field that went and one that came are a drop and
//! an add.

use std::path::{Path, PathBuf};

use clarity_project::SourceKind;
use clarity_spec::migrate::{Migration, Op, Rename};
use clarity_spec::Severity;

use crate::Written;

/// The title of the steps the agent writes itself; the mark of a step it may fold.
pub const AGENT_STEP: &str = "Changes by the agent";

/// What following a change wrote: the step files, and a line for the agent.
#[derive(Debug, Default)]
pub struct Followed {
    pub written: Vec<Written>,
    pub said: String,
}

/// Puts what the data model of the project (`file`, its folder `dir`)
/// changed into the agent's step of each data source Clarity keeps the
/// tables of: folded into its latest step while no database has applied
/// it, a new step otherwise. `renames` are the ones the agent asked for.
/// A project with errors gets no step yet: it follows once the project checks clean.
pub fn follow(file: &Path, dir: &Path, renames: &[Rename]) -> Result<Followed, String> {
    let project = clarity_project::load_modules(dir).map_err(|err| format!("cannot read the project: {err}"))?;
    if project.diagnostics.iter().any(|d| d.severity == Severity::Error) {
        return Ok(Followed::default());
    }
    let settings = clarity_project::load(file).settings.ok_or("the project file cannot be read")?;
    let mut out = Followed::default();
    let mut lines = Vec::new();
    for source in settings.datasources.iter().filter(|s| s.kind != SourceKind::Rest && !s.external) {
        let steps: Vec<Migration> = project.migrations.as_ref().and_then(|m| m.get(&source.name)).cloned().unwrap_or_default();
        let applied = match source.kind {
            SourceKind::Sqlite => Some(clarity_db::applied_steps(&clarity_project::sqlite_path(dir, source))?),
            // A server database cannot be seen from here: every change gets a step of its own.
            _ => None,
        };
        let foldable = steps.last().filter(|last| last.title == AGENT_STEP && !last.baseline && applied.is_some_and(|a| a < steps.len()));
        match foldable {
            Some(last) => {
                let before: Vec<Op> = last.ops.iter().map(|(op, _)| op.clone()).collect();
                let mut kept = renames_in(&before);
                for rename in renames {
                    kept = composed(kept, rename);
                }
                let mut without = project.clone();
                if let Some(m) = without.migrations.as_mut().and_then(|m| m.get_mut(&source.name)) {
                    m.pop();
                }
                let ops = clarity_spec::migrate::pending(&without, &source.name, &kept);
                if ops == before {
                    continue;
                }
                let path = PathBuf::from(&last.path);
                let was = std::fs::read_to_string(&path).ok();
                if ops.is_empty() {
                    // The changes undid each other: the step has nothing left to do.
                    std::fs::remove_file(&path).map_err(|err| format!("cannot remove {}: {err}", path.display()))?;
                    lines.push(format!("The step {} has nothing left to do and is gone.", shown(dir, &path)));
                    continue;
                }
                let text = clarity_spec::migrate::step_text(AGENT_STEP, &ops);
                std::fs::write(&path, &text).map_err(|err| format!("cannot write {}: {err}", path.display()))?;
                lines.push(format!("Folded into the step {}:\n{}", shown(dir, &path), text.trim_end()));
                out.written.push(Written { file: path, before: was, after: Some(text) });
            }
            None => {
                let ops = clarity_spec::migrate::pending(&project, &source.name, renames);
                if ops.is_empty() {
                    continue;
                }
                let path = clarity_spec::write_step(dir, &source.name, AGENT_STEP, &ops).map_err(|err| format!("cannot write the step: {err}"))?;
                let text = std::fs::read_to_string(&path).map_err(|err| format!("cannot read {}: {err}", path.display()))?;
                lines.push(format!("Wrote the step {}:\n{}", shown(dir, &path), text.trim_end()));
                out.written.push(Written { file: path, before: None, after: Some(text) });
            }
        }
    }
    out.said = lines.join("\n");
    Ok(out)
}

/// A step's path as the agent reads it: from the project's folder, with slashes.
fn shown(dir: &Path, path: &Path) -> String {
    clarity_project::slashed(path.strip_prefix(dir).unwrap_or(path))
}

/// The renames a step records, which folding keeps.
fn renames_in(ops: &[Op]) -> Vec<Rename> {
    ops.iter()
        .filter_map(|op| match op {
            Op::RenameEntity { from, to } => Some(Rename { entity: None, from: from.clone(), to: to.clone() }),
            Op::RenameField { entity, from, to } => Some(Rename { entity: Some(entity.clone()), from: from.clone(), to: to.clone() }),
            _ => None,
        })
        .collect()
}

/// The renames with `next` added: a name renamed again keeps one rename,
/// from its first name to its last; one renamed back keeps none.
fn composed(mut renames: Vec<Rename>, next: &Rename) -> Vec<Rename> {
    match renames.iter().position(|r| r.entity == next.entity && r.to == next.from) {
        Some(i) if renames[i].from == next.to => {
            renames.remove(i);
        }
        Some(i) => renames[i].to = next.to.clone(),
        None => renames.push(next.clone()),
    }
    renames
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rename(entity: Option<&str>, from: &str, to: &str) -> Rename {
        Rename { entity: entity.map(String::from), from: from.into(), to: to.into() }
    }

    #[test]
    fn renames_compose_from_the_first_name_to_the_last() {
        let once = composed(Vec::new(), &rename(Some("Customer"), "name", "fullName"));
        assert_eq!(composed(once.clone(), &rename(Some("Customer"), "fullName", "displayName")), [rename(Some("Customer"), "name", "displayName")]);
        assert_eq!(composed(once.clone(), &rename(Some("Customer"), "fullName", "name")), []);
        assert_eq!(composed(once, &rename(Some("Order"), "fullName", "x")).len(), 2, "another entity's field");
    }
}
