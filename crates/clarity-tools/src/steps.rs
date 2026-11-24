//! Migration steps, written one way for every front end (the IDE's
//! Database section, the agents' migrate tool, `clarity migrate`):
//! `write_steps` writes what the data dictionary changed as the next step of
//! each data source that has steps (`clarity_spec::migrate::owned_sources`).
//! A rename reaches a step only as it was asked for (the rename tool, the
//! IDE's rename, a rename the person picked or named), never guessed: a
//! field that went and one that came are a drop and an add.
//!
//! The agents' steps (docs/ledger/mcp-tools.md): when a tool changes the
//! project's data model, `follow` puts the change into a step at once, so
//! the model and the steps are never out of step. While no database has
//! applied Clarity's latest step, a change folds into it, so a task leaves
//! one step.

use std::path::{Path, PathBuf};

use clarity_project::SourceKind;
use clarity_spec::migrate::{Migration, Op, Rename};
use clarity_spec::{Project, Severity};

use crate::Written;
use crate::files::FileChange;

/// The title the agent's steps had before they were titled "Step N" as every step is: such a step still folds.
pub const AGENT_STEP: &str = "Changes by the agent";

/// What every front end answers when no step is needed.
pub const NOTHING_TO_MIGRATE: &str = "The migrations cover the data dictionary; nothing to migrate.";

/// Why no step can be written now: the project has errors.
pub fn blocked(project: &Project) -> Option<String> {
    project.diagnostics.iter().find(|d| d.severity == Severity::Error).map(|d| format!("fix the errors first: {}", d.message))
}

/// The changes of one data source that no step covers yet.
#[derive(Debug, Clone)]
pub struct Pending {
    pub source: String,
    /// The changes, with the renames that were given taken as renames.
    pub ops: Vec<Op>,
    /// Renames Clarity can suggest (a name that went and one of the same
    /// kind that came); shown for the person to pick, never applied unasked.
    pub proposed: Vec<Rename>,
    /// How many steps the source has.
    pub steps: usize,
}

/// The data sources a step is written for: `source` when it is given (and
/// has steps), else every source that has steps.
fn sources_of<'a>(project: &'a Project, source: Option<&'a str>) -> Result<Vec<&'a str>, String> {
    let owned = clarity_spec::migrate::owned_sources(project);
    match source.map(str::trim).filter(|s| !s.is_empty()) {
        Some(name) if owned.contains(&name) => Ok(vec![name]),
        Some(name) if project.sources.iter().any(|s| s.name == name) => {
            Err(format!("data source {name} has no migration steps: steps are kept for the SQL databases Clarity owns ({})", listed_or_none(&owned)))
        }
        Some(name) => Err(format!("the project has no data source {name}; the ones with migration steps are {}", listed_or_none(&owned))),
        None => Ok(owned),
    }
}

fn listed_or_none(names: &[&str]) -> String {
    if names.is_empty() { "none".to_string() } else { names.join(", ") }
}

/// The pending changes of `source` (or of every source with steps), with
/// `renames` taken as renames.
pub fn pending(project: &Project, source: Option<&str>, renames: &[Rename]) -> Result<Vec<Pending>, String> {
    Ok(sources_of(project, source)?
        .into_iter()
        .map(|source| Pending {
            source: source.to_string(),
            ops: clarity_spec::migrate::pending(project, source, renames),
            proposed: clarity_spec::migrate::proposed_renames(project, source),
            steps: project.migrations.as_ref().and_then(|m| m.get(source)).map_or(0, Vec::len),
        })
        .collect())
}

/// What writing the next step is asked to do.
#[derive(Debug, Default, Clone)]
pub struct StepRequest<'a> {
    /// One data source; every source with steps when not given.
    pub source: Option<&'a str>,
    /// The step's title; `Step N` when not given.
    pub title: Option<&'a str>,
    /// The renames that were asked for; nothing else is taken as a rename.
    pub renames: &'a [Rename],
    /// The person allowed changes that can lose data (removing a field or
    /// an entity, changing a type).
    pub allow_data_loss: bool,
}

/// What writing the next steps did: the files, for Undo, and a line per step.
#[derive(Debug, Default)]
pub struct Migrated {
    pub written: Vec<FileChange>,
    pub said: String,
}

/// Writes what the data dictionary of the project in `dir` changed since
/// the last step as the next step of each data source asked for. Refused
/// while the project has errors, and, when a change can lose data, unless
/// the person allowed it; nothing is written then. When no step is needed
/// it writes nothing and says NOTHING_TO_MIGRATE.
pub fn write_steps(dir: &Path, request: &StepRequest) -> Result<Migrated, String> {
    let project = clarity_project::load_modules(dir).map_err(|err| format!("cannot read the project: {err}"))?;
    if let Some(reason) = blocked(&project) {
        return Err(reason);
    }
    let pending: Vec<Pending> = pending(&project, request.source, request.renames)?.into_iter().filter(|p| !p.ops.is_empty()).collect();
    if pending.is_empty() {
        return Ok(Migrated { written: Vec::new(), said: NOTHING_TO_MIGRATE.to_string() });
    }
    let losing: Vec<String> = pending.iter().flat_map(|p| &p.ops).filter(|op| op.loses_data()).map(|op| clarity_spec::migrate::op_text(op).lines().next().unwrap_or_default().to_string()).collect();
    if !losing.is_empty() && !request.allow_data_loss {
        return Err(format!("some changes can lose data ({}): check them, and write the step once data loss is allowed", losing.join("; ")));
    }
    let mut out = Migrated::default();
    let mut lines = Vec::new();
    for p in pending {
        let title = request.title.map(str::trim).filter(|t| !t.is_empty()).map(String::from).unwrap_or_else(|| format!("Step {}", p.steps + 1));
        let path = clarity_spec::write_step(dir, &p.source, &title, &p.ops).map_err(|err| format!("cannot write the step: {err}"))?;
        let text = std::fs::read_to_string(&path).map_err(|err| format!("cannot read {}: {err}", path.display()))?;
        lines.push(format!("Wrote the step {}:\n{}", shown(dir, &path), text.trim_end()));
        out.written.push(FileChange { file: path, before: None, after: Some(text) });
    }
    out.said = lines.join("\n");
    Ok(out)
}

/// Whether the step file at `path` is one Clarity wrote from the data
/// dictionary (its first line says so), not one written by hand.
fn written_by_clarity(path: &str) -> bool {
    let first = clarity_spec::migrate::step_text("", &[]);
    let first = first.lines().next().unwrap_or_default();
    std::fs::read_to_string(path).is_ok_and(|text| text.lines().next() == Some(first))
}

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
    if blocked(&project).is_some() {
        return Ok(Followed::default());
    }
    let settings = clarity_project::load(file).settings.ok_or("the project file cannot be read")?;
    let mut out = Followed::default();
    let mut lines = Vec::new();
    // The same sources as every step: the SQL databases Clarity owns.
    let owned = clarity_spec::migrate::owned_sources(&project);
    for source in settings.datasources.iter().filter(|s| owned.contains(&s.name.as_str())) {
        let steps: Vec<Migration> = project.migrations.as_ref().and_then(|m| m.get(&source.name)).cloned().unwrap_or_default();
        let applied = match source.kind {
            SourceKind::Sqlite => Some(clarity_db::applied_steps(&clarity_project::sqlite_path(dir, source))?),
            // A server database cannot be seen from here: every change gets a step of its own.
            _ => None,
        };
        // Clarity's own step (the agent's, or one migrate wrote under a title) that no database has run.
        let foldable = steps.last().filter(|last| (last.title == AGENT_STEP || written_by_clarity(&last.path)) && !last.baseline && applied.is_some_and(|a| a < steps.len()));
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
                let text = clarity_spec::migrate::step_text(&last.title, &ops);
                std::fs::write(&path, &text).map_err(|err| format!("cannot write {}: {err}", path.display()))?;
                lines.push(format!("Folded into the step {}:\n{}", shown(dir, &path), text.trim_end()));
                out.written.push(Written { file: path, before: was, after: Some(text) });
            }
            None => {
                let ops = clarity_spec::migrate::pending(&project, &source.name, renames);
                if ops.is_empty() {
                    continue;
                }
                // Titled as every step is, whoever made the change (the person in the IDE, an agent).
                let path = clarity_spec::write_step(dir, &source.name, &format!("Step {}", steps.len() + 1), &ops).map_err(|err| format!("cannot write the step: {err}"))?;
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
            Op::RenameEntity { from, to } => Some(Rename { entity: None, from: from.clone(), to: to.clone(), value: false }),
            Op::RenameField { entity, from, to } => Some(Rename { entity: Some(entity.clone()), from: from.clone(), to: to.clone(), value: false }),
            Op::RenameValue { enumeration, from, to } => Some(Rename::value(enumeration, from, to)),
            _ => None,
        })
        .collect()
}

/// The renames with `next` added: a name renamed again keeps one rename,
/// from its first name to its last; one renamed back keeps none.
fn composed(mut renames: Vec<Rename>, next: &Rename) -> Vec<Rename> {
    match renames.iter().position(|r| r.entity == next.entity && r.value == next.value && r.to == next.from) {
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
        Rename { entity: entity.map(String::from), from: from.into(), to: to.into(), value: false }
    }

    /// A new project in a scratch folder with entity Task (its label keys
    /// given), written by hand so no step follows it.
    fn project(name: &str) -> (PathBuf, PathBuf, PathBuf) {
        let root = std::env::temp_dir().join(format!("clarity-steps-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let file = crate::new_project::write_new_project(&root.join("todo"), clarity_project::starter("Todo", "todo").stack).unwrap();
        let dir = clarity_project::module_dir(&file);
        let en = dir.join("translations/en.clarity");
        let keys = format!("{}\n    taskTitle \"Title\"\n    taskDone \"Done\"\n", std::fs::read_to_string(&en).unwrap().trim_end());
        std::fs::write(&en, keys).unwrap();
        tasks(&dir, "title: text(120), label @taskTitle\n    done: bool, label @taskDone");
        (root, file, dir)
    }

    fn tasks(dir: &Path, fields: &str) {
        std::fs::write(dir.join("tasks.clarity"), format!("module tasks\n\nentity Task\n    {fields}\n")).unwrap();
    }

    fn latest(dir: &Path) -> String {
        let mut names: Vec<PathBuf> = std::fs::read_dir(dir.join("migrations/main")).unwrap().flatten().map(|e| e.path()).collect();
        names.sort();
        std::fs::read_to_string(names.last().unwrap()).unwrap()
    }

    /// Every front end writes a step this one way: the next step of each
    /// source with steps, titled `Step N` by default, given back for Undo;
    /// with nothing to do, the same answer everywhere.
    #[test]
    fn the_next_step_is_written_one_way() {
        let (root, _, dir) = project("one-way");
        let project = clarity_project::load_modules(&dir).unwrap();
        let pending = pending(&project, None, &[]).unwrap();
        assert_eq!(pending.len(), 1, "{pending:?}");
        assert_eq!(pending[0].source, "main");
        assert!(pending[0].ops.iter().any(|op| clarity_spec::migrate::op_text(op).starts_with("add entity Task")), "{pending:?}");
        let count = pending[0].steps;

        let wrote = write_steps(&dir, &StepRequest::default()).unwrap();
        assert_eq!(wrote.written.len(), 1);
        assert_eq!(wrote.written[0].before, None);
        assert!(wrote.said.starts_with("Wrote the step migrations/main/"), "{}", wrote.said);
        assert!(latest(&dir).contains(&format!("migration \"Step {}\"", count + 1)), "{}", latest(&dir));

        let again = write_steps(&dir, &StepRequest { title: Some("Tasks"), ..Default::default() }).unwrap();
        assert!(again.written.is_empty());
        assert_eq!(again.said, NOTHING_TO_MIGRATE);
        let _ = std::fs::remove_dir_all(root);
    }

    /// A change that can lose data is written only once the person allowed
    /// it; refused, nothing is written.
    #[test]
    fn a_change_that_can_lose_data_waits_for_the_person() {
        let (root, _, dir) = project("data-loss");
        write_steps(&dir, &StepRequest::default()).unwrap();
        tasks(&dir, "title: text(120), label @taskTitle");
        let before = latest(&dir);
        let refused = write_steps(&dir, &StepRequest::default()).unwrap_err();
        assert!(refused.contains("can lose data") && refused.contains("remove field Task.done"), "{refused}");
        assert_eq!(latest(&dir), before, "nothing written");
        let wrote = write_steps(&dir, &StepRequest { allow_data_loss: true, ..Default::default() }).unwrap();
        assert_eq!(wrote.written.len(), 1);
        assert!(latest(&dir).contains("remove field Task.done"), "{}", latest(&dir));
        let _ = std::fs::remove_dir_all(root);
    }

    /// A project with errors gets no step, and a source without steps (a web
    /// API, a name the project does not have) is refused by name.
    #[test]
    fn errors_and_sources_without_steps_are_refused() {
        let (root, file, dir) = project("refused");
        let text = std::fs::read_to_string(&file).unwrap();
        std::fs::write(&file, format!("{text}    datasource forms rest \"https://forms.example.com/api\"\n")).unwrap();
        let rest = write_steps(&dir, &StepRequest { source: Some("forms"), ..Default::default() }).unwrap_err();
        assert!(rest.contains("forms has no migration steps") && rest.contains("main"), "{rest}");
        let none = write_steps(&dir, &StepRequest { source: Some("nope"), ..Default::default() }).unwrap_err();
        assert!(none.contains("no data source nope"), "{none}");
        tasks(&dir, "title: nonsense");
        let broken = write_steps(&dir, &StepRequest::default()).unwrap_err();
        assert!(broken.starts_with("fix the errors first"), "{broken}");
        let _ = std::fs::remove_dir_all(root);
    }

    /// A rename is taken only as it was asked for: a name written over is a
    /// drop and an add, while the rename Clarity could suggest is only listed.
    #[test]
    fn a_rename_is_taken_only_as_asked() {
        let (root, _, dir) = project("renames");
        write_steps(&dir, &StepRequest::default()).unwrap();
        tasks(&dir, "name: text(120), label @taskTitle\n    done: bool, label @taskDone");
        let project = clarity_project::load_modules(&dir).unwrap();
        let suggested = rename(Some("Task"), "title", "name");
        let listed = pending(&project, None, &[]).unwrap();
        assert_eq!(listed[0].proposed, [suggested.clone()], "suggested, not applied");
        assert!(listed[0].ops.iter().any(|op| op.loses_data()), "a drop and an add: {listed:?}");
        let asked = [suggested];
        let wrote = write_steps(&dir, &StepRequest { renames: &asked, ..Default::default() }).unwrap();
        assert_eq!(wrote.written.len(), 1);
        assert!(latest(&dir).contains("rename field Task.title to name"), "{}", latest(&dir));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn renames_compose_from_the_first_name_to_the_last() {
        let once = composed(Vec::new(), &rename(Some("Customer"), "name", "fullName"));
        assert_eq!(composed(once.clone(), &rename(Some("Customer"), "fullName", "displayName")), [rename(Some("Customer"), "name", "displayName")]);
        assert_eq!(composed(once.clone(), &rename(Some("Customer"), "fullName", "name")), []);
        assert_eq!(composed(once, &rename(Some("Order"), "fullName", "x")).len(), 2, "another entity's field");
    }
}
