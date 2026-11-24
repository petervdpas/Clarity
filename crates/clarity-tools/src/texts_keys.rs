//! Make texts keys: every text people read in the project becomes a key, in
//! every module and every window's, view's and app's `.cxaml`, and joins the
//! language files (the default language's text from the text as written,
//! another language's from a line that translated the text by what it
//! said, else empty). The keys the project has stay, with their texts. The
//! one way for the IDE (Make texts keys, under Problems) and `clarity spec
//! keys`.

use std::path::{Path, PathBuf};

use clarity_spec::translations::TRANSLATIONS_DIR;

use crate::files::FileChange;

/// Makes the texts of the project in `dir` keys and writes only the files
/// that change: each with its text before and after, for Undo.
pub fn make_texts_keys(dir: &Path) -> Result<Vec<FileChange>, String> {
    let project = clarity_project::load_modules(dir).map_err(|err| format!("cannot read {}: {err}", dir.display()))?;
    let default = clarity_spec::translations::default_language(&project);
    let read = |path: &Path| std::fs::read_to_string(path).map(|t| (path.to_path_buf(), t)).map_err(|err| format!("cannot read {}: {err}", path.display()));
    let read_all = |folder: &Path| -> Result<Vec<(PathBuf, String)>, String> {
        if !folder.is_dir() {
            return Ok(Vec::new());
        }
        clarity_spec::spec_files(folder).map_err(|err| format!("cannot read {}: {err}", folder.display()))?.iter().map(|p| read(p)).collect()
    };
    let modules = read_all(dir)?;
    let pages: Vec<(PathBuf, String)> =
        clarity_spec::cxaml::page_files(dir, &[], &[]).map_err(|err| format!("cannot read {}: {err}", dir.display()))?.iter().map(|p| read(p)).collect::<Result<_, _>>()?;
    let folder = dir.join(TRANSLATIONS_DIR);
    let languages = read_all(&folder)?;
    let named = |files: &[(PathBuf, String)]| files.iter().map(|(p, t)| (p.display().to_string(), t.clone())).collect::<Vec<_>>();
    let (module_names, page_names, language_names) = (named(&modules), named(&pages), named(&languages));
    fn refs(files: &[(String, String)]) -> Vec<(&str, &str)> {
        files.iter().map(|(p, t)| (p.as_str(), t.as_str())).collect()
    }
    // The keys the project has stay as they are: a new text that says the same as one takes its key.
    let existing = project.translations.get(&default).cloned().unwrap_or_default();
    let made = clarity_spec::cxaml::make_keys_with_pages_beside(&refs(&module_names), &refs(&page_names), &refs(&language_names), &default, &existing);
    // The result lists the modules and then the pages, each by its own path.
    let mut changes = Vec::new();
    for (path, after) in &made.modules {
        let before = modules.iter().chain(&pages).find(|(p, _)| p.display().to_string() == *path).map(|(_, t)| t.clone());
        if before.as_deref() != Some(after.as_str()) {
            changes.push(FileChange { file: PathBuf::from(path), before, after: Some(after.clone()) });
        }
    }
    // The checker decides which keys the project uses, Clarity's own words among them: a key it
    // does not use is not written, so none is left that can go.
    let replaced: Vec<(&Path, &str)> = changes.iter().map(|c| (c.file.as_path(), c.after.as_deref().unwrap_or(""))).collect();
    let keyed = clarity_project::load_modules_with_files(dir, &replaced).map_err(|err| format!("cannot read {}: {err}", dir.display()))?;
    let used: Vec<String> = clarity_spec::translations::keys(&keyed).into_iter().map(|k| k.key).collect();
    for (language, values) in &made.languages {
        let path = folder.join(format!("{language}.{}", clarity_spec::SPEC_EXTENSION));
        let before = languages.iter().find(|(p, _)| *p == path).map(|(_, t)| t.clone());
        let added: Vec<(String, String)> = made.order.iter().filter(|key| used.contains(key)).filter_map(|key| values.get(key).map(|value| (key.clone(), value.clone()))).collect();
        let after = clarity_spec::keyed::with_keys(before.as_deref(), language, &added);
        if before.as_deref() != Some(after.as_str()) {
            changes.push(FileChange { file: path, before, after: Some(after) });
        }
    }
    if !changes.is_empty() {
        std::fs::create_dir_all(&folder).map_err(|err| format!("cannot make {}: {err}", folder.display()))?;
    }
    for change in &changes {
        change.make()?;
    }
    Ok(changes)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A text written in a view's `.cxaml` becomes a key with its text in the
    /// default language's file; only the files that change are written, and
    /// a second run changes nothing.
    #[test]
    fn a_text_in_a_view_file_becomes_a_key_and_only_changed_files_are_written() {
        let root = std::env::temp_dir().join(format!("clarity-texts-keys-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let file = crate::new_project::write_new_project(&root.join("todo"), clarity_project::starter("Todo", "todo").stack).unwrap();
        let dir = clarity_project::module_dir(&file);
        let view = clarity_spec::cxaml::design_folder(&dir).join("Extra.cxaml");
        std::fs::write(&view, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<View class=\"todo.Extra\">\n  <Button name=\"goButton\" at=\"1,1\" span=\"2\" caption=\"Go on\" />\n</View>\n").unwrap();
        let changes = make_texts_keys(&dir).unwrap();
        let mut changed: Vec<String> = changes.iter().map(|c| clarity_project::slashed(c.file.strip_prefix(&dir).unwrap())).collect();
        changed.sort();
        assert_eq!(changed, ["design/Extra.cxaml", "translations/en.clarity"]);
        let markup = std::fs::read_to_string(&view).unwrap();
        let key = markup.split("caption=\"@").nth(1).and_then(|rest| rest.split('"').next()).unwrap_or_else(|| panic!("no key in {markup}"));
        assert!(std::fs::read_to_string(dir.join("translations/en.clarity")).unwrap().contains(&format!("{key} \"Go on\"")));
        let en = changes.iter().find(|c| c.file.ends_with("en.clarity")).unwrap();
        assert!(en.before.is_some() && en.after.as_deref() != en.before.as_deref(), "before and after, for Undo");
        // The keys the project has stay, and no key is written that the project does not use.
        let module = dir.join("todo.clarity");
        let text = std::fs::read_to_string(&module).unwrap();
        std::fs::write(&module, format!("{text}\nfunction counted() -> integer\n    steps = 1\n    return steps\n")).unwrap();
        let again = make_texts_keys(&dir).unwrap();
        assert!(again.is_empty(), "a second run writes nothing: {:#?}", again.iter().map(|c| (&c.file, &c.before, &c.after)).collect::<Vec<_>>());
        std::fs::remove_dir_all(&root).unwrap();
    }

    /// A language file that translated a text by what it said gives the key
    /// made of the text its translation, and the old line goes.
    #[test]
    fn an_old_translation_gives_the_new_key_its_text() {
        let root = std::env::temp_dir().join(format!("clarity-texts-keys-old-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let file = crate::new_project::write_new_project(&root.join("todo"), clarity_project::starter("Todo", "todo").stack).unwrap();
        let dir = clarity_project::module_dir(&file);
        std::fs::write(clarity_spec::cxaml::design_folder(&dir).join("Extra.cxaml"), "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<View class=\"todo.Extra\">\n  <Button name=\"goButton\" at=\"1,1\" span=\"2\" caption=\"Go on\" />\n</View>\n").unwrap();
        let nl = dir.join("translations/nl.clarity");
        std::fs::write(&nl, "translation nl\n    yes \"Ja\"\n    \"Go on\" \"Ga door\"\n").unwrap();
        make_texts_keys(&dir).unwrap();
        let text = std::fs::read_to_string(&nl).unwrap();
        assert!(text.starts_with("translation nl\n    yes \"Ja\"\n") && text.contains("    goButtonCaption \"Ga door\"\n") && !text.contains("\"Go on\""), "{text}");
        std::fs::remove_dir_all(&root).unwrap();
    }
}
