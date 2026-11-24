//! Naming parts and making names camelCase: the one way for the agents'
//! name_parts and camel_names and for `clarity spec names`. Both work on
//! every window, view and app, in a module or in its own `.cxaml`, or on
//! one of them; camelCase renames a handler in the page's code block and a
//! slot in the translations with it. Refused, writing nothing, when the
//! project would gain an error in a file they write.

use std::path::{Path, PathBuf};

use clarity_spec::edit::Edit;
use clarity_spec::translations::TRANSLATIONS_DIR;

use crate::edits::{code_file_of, is_page_file};
use crate::files::{FileChange, write_atomically};

/// Gives every part without a name the first free one of its kind, in the
/// file `only` or in every file holding a window, view or app of the project
/// in `dir`. Gives each file written, with its text before and after.
pub fn name_parts(dir: &Path, only: Option<&Path>) -> Result<Vec<FileChange>, String> {
    let before = load(dir)?;
    let mut texts = Texts::default();
    for file in owners(dir, &before, only) {
        let text = texts.text(&file)?;
        let shown = shown(dir, &file);
        let after = if is_page_file(&file) {
            clarity_spec::cxaml::apply(Some(&before), &text, &Edit::NameParts).map_err(|err| format!("{shown}: {err}"))?.text
        } else {
            clarity_spec::edit::apply_in(&before, &text, &Edit::NameParts).map_err(|err| format!("{shown}: {err}"))?.text
        };
        texts.set(&file, after);
    }
    texts.write_checked(dir, &before)
}

/// Renames every part and handler that is not camelCase to camelCase, in
/// the file `only` (a module with the windows and views whose code it
/// holds) or in every file holding a window, view or app of the project in
/// `dir`: a `.cxaml` with its page's code block in the module, and the
/// slots of the translations that name them. Gives each file
/// written, with its text before and after.
pub fn camel_names(dir: &Path, only: Option<&Path>) -> Result<Vec<FileChange>, String> {
    let before = load(dir)?;
    let mut texts = Texts::default();
    let mut renames = Vec::new();
    let mut files = owners(dir, &before, only);
    // A module holds the code of the windows and views in their own files: those are its too.
    if let Some(module) = only.filter(|f| !is_page_file(f)) {
        let pages = owners(dir, &before, None).into_iter().filter(|f| is_page_file(f) && code_file_of(f).is_ok_and(|m| m == module));
        files.extend(pages);
    }
    for file in files {
        if is_page_file(&file) {
            let module = code_file_of(&file)?;
            let (markup, code) = (texts.text(&file)?, texts.text(&module)?);
            let (markup, code, renamed) = clarity_spec::names::camel_names_of_page(&markup, &code);
            texts.set(&file, markup);
            texts.set(&module, code);
            renames.extend(renamed);
        } else {
            let (text, renamed) = clarity_spec::names::camel_names_with(&texts.text(&file)?);
            texts.set(&file, text);
            renames.extend(renamed);
        }
    }
    // The translations' texts follow the renamed slots of the texts.
    let folder = dir.join(TRANSLATIONS_DIR);
    if !renames.is_empty() && folder.is_dir() {
        for path in clarity_spec::spec_files(&folder).map_err(|err| format!("cannot read {}: {err}", folder.display()))? {
            let text = texts.text(&path)?;
            texts.set(&path, clarity_spec::names::rename_slots(&text, &renames));
        }
    }
    texts.write_checked(dir, &before)
}

fn load(dir: &Path) -> Result<clarity_spec::Project, String> {
    clarity_project::load_modules(dir).map_err(|err| format!("cannot read {}: {err}", dir.display()))
}

fn shown(dir: &Path, file: &Path) -> String {
    clarity_project::slashed(file.strip_prefix(dir).unwrap_or(file))
}

/// `only`, or every file of `project` that holds a window, view or app.
fn owners(dir: &Path, project: &clarity_spec::Project, only: Option<&Path>) -> Vec<PathBuf> {
    if let Some(only) = only {
        return vec![only.to_path_buf()];
    }
    let mut files: Vec<PathBuf> = Vec::new();
    for file in project.files.iter().filter(|f| !f.pages.is_empty() || !f.apps.is_empty()) {
        let path = Path::new(&file.path);
        let path = if path.is_absolute() { path.to_path_buf() } else { dir.join(path) };
        if !files.contains(&path) {
            files.push(path);
        }
    }
    files
}

/// The files being changed: each with its text before and its text now.
#[derive(Default)]
struct Texts(Vec<(PathBuf, String, String)>);

impl Texts {
    fn text(&mut self, path: &Path) -> Result<String, String> {
        if let Some((_, _, now)) = self.0.iter().find(|(p, _, _)| p == path) {
            return Ok(now.clone());
        }
        let text = std::fs::read_to_string(path).map_err(|err| format!("cannot read {}: {err}", path.display()))?;
        self.0.push((path.to_path_buf(), text.clone(), text.clone()));
        Ok(text)
    }

    fn set(&mut self, path: &Path, text: String) {
        match self.0.iter_mut().find(|(p, _, _)| p == path) {
            Some((_, _, now)) => *now = text,
            None => self.0.push((path.to_path_buf(), String::new(), text)),
        }
    }

    /// Writes the files that changed, unless the project would gain an error
    /// in one of them; gives what it wrote.
    fn write_checked(self, dir: &Path, before: &clarity_spec::Project) -> Result<Vec<FileChange>, String> {
        let changed: Vec<(PathBuf, String, String)> = self.0.into_iter().filter(|(_, was, now)| was != now).collect();
        if changed.is_empty() {
            return Ok(Vec::new());
        }
        let replaced: Vec<(&Path, &str)> = changed.iter().map(|(path, _, now)| (path.as_path(), now.as_str())).collect();
        let after = clarity_project::load_modules_with_files(dir, &replaced).map_err(|err| format!("cannot read {}: {err}", dir.display()))?;
        let written = |error: &clarity_spec::Diagnostic| error.path.is_empty() || replaced.iter().any(|(path, _)| *path == dir.join(&error.path).as_path());
        if let Some(error) = crate::edits::new_errors(before, &after).into_iter().find(|e| written(e)) {
            return Err(format!("{}; nothing was written", crate::edits::refusal(dir, &replaced, error)));
        }
        let mut changes = Vec::new();
        for (file, was, now) in changed {
            write_atomically(&file, &now)?;
            changes.push(FileChange { file, before: Some(was), after: Some(now) });
        }
        Ok(changes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A new project in a scratch folder, with the view Extra in its own
    /// `.cxaml` and its code block in the module.
    fn project(name: &str, markup: &str, code: &str) -> (PathBuf, PathBuf, PathBuf) {
        let root = std::env::temp_dir().join(format!("clarity-naming-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let file = crate::new_project::write_new_project(&root.join("todo"), clarity_project::starter("Todo", "todo").stack).unwrap();
        let dir = clarity_project::module_dir(&file);
        std::fs::write(clarity_spec::cxaml::design_folder(&dir).join("Extra.cxaml"), markup).unwrap();
        let module = dir.join("todo.clarity");
        let text = std::fs::read_to_string(&module).unwrap();
        std::fs::write(&module, format!("{text}\n{code}")).unwrap();
        (root, dir, module)
    }

    /// camelCase reaches a view's own file: its parts and handlers in the
    /// `.cxaml`, the code block in the module and the slots of the
    /// translations, each file written once with its text before, for Undo.
    #[test]
    fn camel_names_reach_a_view_file_its_code_and_the_translations() {
        let markup = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<View class=\"todo.Extra\">\n  <Button name=\"go_button\" at=\"1,1\" span=\"2\" caption=\"@goCaption\" onclick=\"go_clicked\" />\n</View>\n";
        let code = "code Extra\n    handler go_clicked\n        message @goSaid(go_button = go_button.caption)\n";
        let (root, dir, module) = project("camel", markup, code);
        let en = dir.join("translations/en.clarity");
        let text = std::fs::read_to_string(&en).unwrap();
        std::fs::write(&en, format!("{text}    goCaption \"Go\"\n    goSaid \"Went {{go_button}}\"\n")).unwrap();

        let changes = camel_names(&dir, None).unwrap();
        let mut changed: Vec<String> = changes.iter().map(|c| shown(&dir, &c.file)).collect();
        changed.sort();
        assert_eq!(changed, ["design/Extra.cxaml", "todo.clarity", "translations/en.clarity"]);
        assert!(changes.iter().all(|c| c.before.is_some() && c.after.is_some()));
        let markup = std::fs::read_to_string(clarity_spec::cxaml::design_folder(&dir).join("Extra.cxaml")).unwrap();
        assert!(markup.contains("name=\"goButton\"") && markup.contains("onclick=\"goClicked\""), "{markup}");
        let code = std::fs::read_to_string(&module).unwrap();
        assert!(code.contains("handler goClicked\n        message @goSaid(goButton = goButton.caption)"), "{code}");
        assert!(std::fs::read_to_string(&en).unwrap().contains("goSaid \"Went {goButton}\""));
        assert!(camel_names(&dir, None).unwrap().is_empty(), "a second run writes nothing");
        std::fs::remove_dir_all(&root).unwrap();
    }

    /// The designer's fix in Problems (Make them camelCase, an edit on the
    /// file of the problem) is this one way too: on a module, it reaches the
    /// windows and views whose code is in it.
    #[test]
    fn the_designer_edit_on_a_module_reaches_its_views() {
        let markup = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<View class=\"todo.Extra\">\n  <Button name=\"goButton\" at=\"1,1\" span=\"2\" caption=\"@goCaption\" onclick=\"go_clicked\" />\n</View>\n";
        let (root, dir, module) = project("edit", markup, "code Extra\n    handler go_clicked\n        goButton.enabled = false\n");
        let en = dir.join("translations/en.clarity");
        let text = std::fs::read_to_string(&en).unwrap();
        std::fs::write(&en, format!("{text}    goCaption \"Go\"\n")).unwrap();
        let applied = crate::edits::apply(&dir, &module, &Edit::CamelNames).unwrap();
        assert_eq!(applied.changes.len(), 2);
        assert!(std::fs::read_to_string(&module).unwrap().contains("handler goClicked"));
        assert!(std::fs::read_to_string(clarity_spec::cxaml::design_folder(&dir).join("Extra.cxaml")).unwrap().contains("onclick=\"goClicked\""));
        std::fs::remove_dir_all(&root).unwrap();
    }

    /// A part without a name in a view's own file gets one.
    #[test]
    fn name_parts_names_a_part_in_a_view_file() {
        let markup = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<View class=\"todo.Extra\">\n  <Button at=\"1,1\" span=\"2\" caption=\"@goCaption\" />\n</View>\n";
        let (root, dir, _) = project("parts", markup, "");
        let en = dir.join("translations/en.clarity");
        let text = std::fs::read_to_string(&en).unwrap();
        std::fs::write(&en, format!("{text}    goCaption \"Go\"\n")).unwrap();
        let changes = name_parts(&dir, None).unwrap();
        assert_eq!(changes.iter().map(|c| shown(&dir, &c.file)).collect::<Vec<_>>(), ["design/Extra.cxaml"]);
        let markup = std::fs::read_to_string(clarity_spec::cxaml::design_folder(&dir).join("Extra.cxaml")).unwrap();
        assert!(markup.contains("<Button name=\""), "{markup}");
        assert!(name_parts(&dir, None).unwrap().is_empty());
        std::fs::remove_dir_all(&root).unwrap();
    }
}
