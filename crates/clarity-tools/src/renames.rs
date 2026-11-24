//! Renaming an entity, an enum, a field, a value of an enum, a window or a
//! view, one way for every front end (the IDE's rename and its editors'
//! name cells, the agents' rename tool and change_enum_value): every place
//! that names it follows, a window's or view's file takes the new name, and
//! the migration step records a data rename (crate::steps::follow), so a
//! stored table, column or value keeps its data.

use std::path::{Path, PathBuf};

use clarity_spec::migrate::Rename;
use clarity_spec::rename::RenameKind;

use crate::files::{FileChange, write_atomically};

/// What is renamed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Renaming {
    Entity,
    Enum,
    /// A field of the entity or record `entity`.
    Field { entity: String },
    /// A value of the enum `enumeration`.
    Value { enumeration: String },
    /// A window (dialogs and splash pages too): refused for a view.
    Window,
    /// A view: refused for a window.
    View,
    /// A window or a view, whichever it is (the designer names a page by its class).
    Page,
}

impl Renaming {
    /// As a sentence names it: `field Customer.name`.
    fn what(&self, from: &str) -> String {
        match self {
            Renaming::Entity => format!("entity {from}"),
            Renaming::Enum => format!("enum {from}"),
            Renaming::Field { entity } => format!("field {entity}.{from}"),
            Renaming::Value { enumeration } => format!("value {enumeration}.{from}"),
            Renaming::Window => format!("window {from}"),
            Renaming::View => format!("view {from}"),
            Renaming::Page => format!("page {from}"),
        }
    }
}

/// What a rename did: the files, for Undo (a moved file's old path with no
/// text after), their paths from the project's folder, and what the
/// migration step said, if anything.
#[derive(Debug, Default)]
pub struct Renamed {
    pub written: Vec<FileChange>,
    pub paths: Vec<String>,
    pub step: String,
    /// `field Customer.name`, as a sentence names what was renamed.
    pub what: String,
}

/// Renames `from` to `to` in the project (`file`, its folder `dir`).
/// Refused while the project has errors (a rename follows the checked
/// model), when the rename would add an error, and when a moved file's new
/// path is taken. A data rename goes into the migration step.
pub fn rename(file: &Path, dir: &Path, renaming: &Renaming, from: &str, to: &str) -> Result<Renamed, String> {
    let (from, to) = (from.trim(), to.trim());
    let project = clarity_project::load_modules(dir).map_err(|err| format!("cannot read the project: {err}"))?;
    if crate::steps::blocked(&project).is_some() {
        return Err("fix the errors first: a rename follows the checked model".into());
    }
    let at = |path: &str| if Path::new(path).is_absolute() { PathBuf::from(path) } else { dir.join(path) };
    let source = |path: &str| std::fs::read_to_string(at(path)).ok();
    let what = renaming.what(from);
    let page_kind = |want_view: bool| -> Result<(), String> {
        let page = project.pages().find(|p| p.name.name == from).ok_or_else(|| format!("there is no window or view {from}"))?;
        let is_view = page.kind == clarity_spec::model::PageKind::View;
        if want_view != is_view {
            let (is, not) = if is_view { ("view", "window") } else { ("window", "view") };
            return Err(format!("{from} is a {is}, not a {not}: give kind {is}"));
        }
        Ok(())
    };
    let (changed, recorded) = match renaming {
        Renaming::Entity => (clarity_spec::rename::rename(&project, source, RenameKind::Entity, from, to)?, Some(Rename { entity: None, from: from.into(), to: to.into(), value: false })),
        Renaming::Enum => (clarity_spec::rename::rename(&project, source, RenameKind::Enum, from, to)?, None),
        Renaming::Field { entity } => {
            if entity.trim().is_empty() {
                return Err("a field is renamed in its entity: give entity, like Customer".into());
            }
            let recorded = Rename { entity: Some(entity.trim().into()), from: from.into(), to: to.into(), value: false };
            (clarity_spec::rename::rename_field(&project, source, entity.trim(), from, to)?, Some(recorded))
        }
        Renaming::Value { enumeration } => {
            if enumeration.trim().is_empty() {
                return Err("a value is renamed in its enum: give entity, like ItemKind".into());
            }
            let recorded = Rename::value(enumeration.trim(), from, to);
            (clarity_spec::value_rename::rename_value(&project, source, enumeration.trim(), from, to)?, Some(recorded))
        }
        Renaming::Window | Renaming::View | Renaming::Page => {
            if *renaming != Renaming::Page {
                page_kind(*renaming == Renaming::View)?;
            }
            (clarity_spec::rename::rename(&project, source, RenameKind::Page, from, to)?, None)
        }
    };
    if changed.is_empty() {
        return Ok(Renamed { what, ..Renamed::default() });
    }
    refuse_new_error(dir, &changed)?;
    for r in changed.iter().filter(|r| r.moved_from.is_some()) {
        let target = at(&r.path);
        if target.exists() {
            return Err(format!("{} is there already", target.display()));
        }
    }
    let mut out = Renamed { what, ..Renamed::default() };
    for r in changed {
        let path = at(&r.path);
        let before = if r.moved_from.is_some() { None } else { std::fs::read_to_string(&path).ok() };
        write_atomically(&path, &r.text)?;
        out.paths.push(clarity_project::slashed(path.strip_prefix(dir).unwrap_or(&path)));
        out.written.push(FileChange { file: path, before, after: Some(r.text) });
        // A window's file takes its new name: the old one goes, kept for Undo.
        if let Some(was) = r.moved_from {
            let was = at(&was);
            let before = std::fs::read_to_string(&was).ok();
            std::fs::remove_file(&was).map_err(|err| format!("cannot remove {}: {err}", was.display()))?;
            out.written.push(FileChange { file: was, before, after: None });
        }
    }
    let followed = crate::steps::follow(file, dir, &recorded.into_iter().collect::<Vec<_>>())?;
    out.written.extend(followed.written.into_iter().map(FileChange::from));
    out.step = followed.said;
    Ok(out)
}

/// Refuses a rename that would add an error anywhere in the project: the
/// whole project checked with every renamed file in place, against the
/// same files as they are.
fn refuse_new_error(dir: &Path, renamed: &[clarity_spec::rename::Renamed]) -> Result<(), String> {
    let read_err = |err: std::io::Error| format!("cannot read {}: {err}", dir.display());
    let at = |path: &str| if Path::new(path).is_absolute() { PathBuf::from(path) } else { dir.join(path) };
    let mut paths = clarity_spec::spec_files(dir).map_err(read_err)?;
    paths.extend(clarity_spec::cxaml::page_files(dir, &paths.clone(), &[]).map_err(read_err)?);
    let mut now = Vec::new();
    let mut after = Vec::new();
    for path in paths {
        let shown = path.display().to_string();
        let text = std::fs::read_to_string(&path).map_err(read_err)?;
        // A moved file (a renamed window's) is read under its new path only.
        if !renamed.iter().any(|r| r.moved_from.as_deref().is_some_and(|was| at(was) == path)) {
            let new = renamed.iter().find(|r| at(&r.path) == path).map_or(text.clone(), |r| r.text.clone());
            after.push((shown.clone(), new));
        }
        now.push((shown, text));
    }
    for r in renamed.iter().filter(|r| r.moved_from.is_some()) {
        after.push((at(&r.path).display().to_string(), r.text.clone()));
    }
    let before = clarity_spec::load_sources(now.iter().map(|(p, t)| (p.as_str(), t.as_str())));
    let after = clarity_spec::load_sources(after.iter().map(|(p, t)| (p.as_str(), t.as_str())));
    match clarity_spec::new_error(&before, &after) {
        Some(error) => Err(error.message.clone()),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project(name: &str) -> (PathBuf, PathBuf, PathBuf) {
        let root = std::env::temp_dir().join(format!("clarity-renames-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let file = crate::new_project::write_new_project(&root.join("todo"), clarity_project::starter("Todo", "todo").stack).unwrap();
        let dir = clarity_project::module_dir(&file);
        let en = dir.join("translations/en.clarity");
        let keys = format!("{}\n    taskTitle \"Title\"\n    taskKind \"Kind\"\n    kindPrivate \"Private\"\n    kindBusiness \"Business\"\n", std::fs::read_to_string(&en).unwrap().trim_end());
        std::fs::write(&en, keys).unwrap();
        let tasks = "module tasks\n\nenum Kind\n    Private label @kindPrivate\n    Business label @kindBusiness\n\nentity Task\n    title: text(120), label @taskTitle\n    kind: Kind, default Private, label @taskKind\n";
        std::fs::write(dir.join("tasks.clarity"), tasks).unwrap();
        (root, file, dir)
    }

    fn latest(dir: &Path) -> String {
        let mut names: Vec<PathBuf> = std::fs::read_dir(dir.join("migrations/main")).unwrap().flatten().map(|e| e.path()).collect();
        names.sort();
        std::fs::read_to_string(names.last().unwrap()).unwrap()
    }

    /// Marks every step as run on the project's database, so the next
    /// change is a step of its own.
    fn applied(file: &Path, dir: &Path) {
        let count = std::fs::read_dir(dir.join("migrations/main")).unwrap().count();
        let settings = clarity_project::load(file).settings.unwrap();
        let db = clarity_project::sqlite_path(dir, &settings.datasources[0]);
        std::fs::create_dir_all(db.parent().unwrap()).unwrap();
        let sql = format!("CREATE TABLE IF NOT EXISTS clarity_schema (version INTEGER NOT NULL); INSERT INTO clarity_schema VALUES ({count});");
        rusqlite::Connection::open(&db).unwrap().execute_batch(&sql).unwrap();
    }

    /// A field's and a value's rename go into the migration step, for every
    /// front end alike, given back with the renamed files for one Undo.
    #[test]
    fn a_data_rename_goes_into_the_step() {
        let (root, file, dir) = project("step");
        crate::steps::write_steps(&dir, &crate::steps::StepRequest::default()).unwrap();
        applied(&file, &dir);
        let field = rename(&file, &dir, &Renaming::Field { entity: "Task".into() }, "title", "name").unwrap();
        assert!(latest(&dir).contains("rename field Task.title to name"), "{}", latest(&dir));
        assert!(field.written.iter().any(|w| w.file.ends_with("tasks.clarity")) && field.written.iter().any(|w| w.file.starts_with(dir.join("migrations"))), "{:?}", field.written);
        assert!(field.step.contains("Wrote the step"), "{}", field.step);
        applied(&file, &dir);
        rename(&file, &dir, &Renaming::Value { enumeration: "Kind".into() }, "Private", "Personal").unwrap();
        assert!(latest(&dir).contains("rename value Kind.Private to Personal"), "{}", latest(&dir));
        assert!(std::fs::read_to_string(dir.join("tasks.clarity")).unwrap().contains("default Personal"));
        let _ = std::fs::remove_dir_all(root);
    }

    /// Every guard any front end had: errors before, a window named as a
    /// view, a moved file's taken path; a page by either kind is fine.
    #[test]
    fn a_rename_takes_every_guard() {
        let (root, file, dir) = project("guards");
        let kind = rename(&file, &dir, &Renaming::View, "Main", "Home").unwrap_err();
        assert!(kind.contains("Main is a window, not a view"), "{kind}");
        // A file in the window's new place: never written over.
        let stray = std::fs::read_to_string(dir.join("design/MainView.cxaml")).unwrap().replace("todo.MainView", "todo.Other");
        std::fs::write(dir.join("design/Home.cxaml"), &stray).unwrap();
        assert!(rename(&file, &dir, &Renaming::Page, "Main", "Home").is_err(), "the new file's place is taken");
        assert_eq!(std::fs::read_to_string(dir.join("design/Home.cxaml")).unwrap(), stray);
        std::fs::remove_file(dir.join("design/Home.cxaml")).unwrap();
        let page = rename(&file, &dir, &Renaming::Page, "Main", "Home").unwrap();
        assert!(page.written.iter().any(|w| w.file == dir.join("design/Main.cxaml") && w.after.is_none()));
        std::fs::write(dir.join("broken.clarity"), "module broken\n\nentity Broken\n    x: nonsense\n").unwrap();
        let broken = rename(&file, &dir, &Renaming::Entity, "Task", "Job").unwrap_err();
        assert!(broken.starts_with("fix the errors first"), "{broken}");
        let _ = std::fs::remove_dir_all(root);
    }
}
