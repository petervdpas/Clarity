//! Move pages into design: every window, view and app still written in a
//! module's file moves into its own `.cxaml` in design/. The one way for
//! the IDE (Move pages into design, under Problems) and `clarity
//! convert-pages`. Refused, writing nothing, when a `.cxaml` it would make is
//! there already or the project would gain an error.

use std::path::{Path, PathBuf};

use crate::files::{FileChange, write_atomically};

/// Moves the pages and the app of the project in `dir` into their own
/// files; gives each file written or made, with its text before and after.
pub fn move_pages_into_design(dir: &Path) -> Result<Vec<FileChange>, String> {
    let read_err = |err: std::io::Error| format!("cannot read {}: {err}", dir.display());
    let mut files = Vec::new();
    for path in clarity_spec::spec_files(dir).map_err(read_err)? {
        let text = std::fs::read_to_string(&path).map_err(|err| format!("cannot read {}: {err}", path.display()))?;
        files.push((path.display().to_string(), text));
    }
    let moved = clarity_spec::cxaml::moved_out(&files)?;
    let mut changes = Vec::new();
    for (path, after) in moved {
        let file = PathBuf::from(&path);
        let before = files.iter().find(|(p, _)| *p == path).map(|(_, t)| t.clone());
        if before.is_none() && file.exists() {
            return Err(format!("{} is there already; nothing was moved", file.display()));
        }
        if before.as_deref() != Some(after.as_str()) {
            changes.push(FileChange { file, before, after: Some(after) });
        }
    }
    if changes.is_empty() {
        return Ok(changes);
    }
    // The project as it would load: the same errors as before, no new ones. A
    // moved page's error is in its new file, so errors are told apart by what they say.
    let errors = |project: &clarity_spec::Project| -> Vec<String> {
        project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.clone()).collect()
    };
    let before = errors(&clarity_project::load_modules(dir).map_err(read_err)?);
    let replaced: Vec<(&Path, &str)> = changes.iter().map(|c| (c.file.as_path(), c.after.as_deref().unwrap_or(""))).collect();
    let after = errors(&clarity_project::load_modules_with_files(dir, &replaced).map_err(read_err)?);
    if let Some(new) = after.iter().find(|e| !before.contains(e)) {
        return Err(format!("moving the pages would add an error: {new}; nothing was moved"));
    }
    for change in &changes {
        if let Some(folder) = change.file.parent() {
            std::fs::create_dir_all(folder).map_err(|err| format!("cannot make {}: {err}", folder.display()))?;
        }
        write_atomically(&change.file, change.after.as_deref().unwrap_or(""))?;
    }
    Ok(changes)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAGE: &str = "\n// A second window.\npage Second: window\n    title @secondTitle\n";

    fn project(name: &str) -> (PathBuf, PathBuf, PathBuf) {
        let root = std::env::temp_dir().join(format!("clarity-pages-out-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let file = crate::new_project::write_new_project(&root.join("todo"), clarity_project::starter("Todo", "todo").stack).unwrap();
        let dir = clarity_project::module_dir(&file);
        let module = dir.join("todo.clarity");
        let text = std::fs::read_to_string(&module).unwrap();
        std::fs::write(&module, format!("{text}{PAGE}")).unwrap();
        let en = dir.join("translations/en.clarity");
        let text = std::fs::read_to_string(&en).unwrap();
        std::fs::write(&en, format!("{text}    secondTitle \"Second\"\n")).unwrap();
        (root, dir, module)
    }

    /// A window written in a module moves into design/, with what each file
    /// held before for Undo; the project checks no worse.
    #[test]
    fn a_window_in_a_module_moves_into_its_own_file() {
        let (root, dir, module) = project("moved");
        let changes = move_pages_into_design(&dir).unwrap();
        let second = clarity_spec::cxaml::design_folder(&dir).join("Second.cxaml");
        assert_eq!(changes.len(), 2, "{:?}", changes.iter().map(|c| &c.file).collect::<Vec<_>>());
        assert!(changes.iter().any(|c| c.file == second && c.before.is_none()));
        assert!(changes.iter().any(|c| c.file == module && c.before.as_deref().is_some_and(|t| t.contains("page Second"))));
        assert!(std::fs::read_to_string(&second).unwrap().contains("<Window class=\"todo.Second\""));
        assert!(!std::fs::read_to_string(&module).unwrap().contains("page Second"));
        assert!(move_pages_into_design(&dir).unwrap().is_empty(), "nothing is left to move");
        std::fs::remove_dir_all(&root).unwrap();
    }

    /// A `.cxaml` there already is never written over: nothing moves.
    #[test]
    fn a_file_there_already_stops_the_move() {
        let (root, dir, module) = project("there");
        let second = clarity_spec::cxaml::design_folder(&dir).join("Second.cxaml");
        std::fs::write(&second, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<View class=\"todo.Second\" />\n").unwrap();
        let before = std::fs::read_to_string(&module).unwrap();
        let refused = move_pages_into_design(&dir).unwrap_err();
        assert!(refused.contains("Second.cxaml is there already; nothing was moved"), "{refused}");
        assert_eq!(std::fs::read_to_string(&module).unwrap(), before);
        std::fs::remove_dir_all(&root).unwrap();
    }
}
