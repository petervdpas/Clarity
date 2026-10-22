//! The languages of a project, under i18n in the project tree: every key
//! with its value per language, written to `translations/<language>.clarity`.
//! Every language file has the same keys; the default language gives each
//! a value.

use std::path::{Path, PathBuf};

use clarity_spec::Preview;
use clarity_spec::translations::{self, TRANSLATIONS_DIR, TranslationTable};
use tauri::State;

use super::{FileChange, HistoryState, WatchState, load, open_project_of, record};

/// The translation file of `language` in the module folder `dir`.
fn file_of(dir: &Path, language: &str) -> Result<PathBuf, String> {
    if let Some(problem) = clarity_spec::formats::language_problem(language) {
        return Err(problem);
    }
    Ok(dir.join(TRANSLATIONS_DIR).join(format!("{language}.{}", clarity_spec::SPEC_EXTENSION)))
}

/// Writes `after` as the translation file `path`, so Undo puts back what
/// was there.
fn write(path: PathBuf, before: Option<String>, after: String, history: &State<HistoryState>) -> Result<(), String> {
    if let Some(folder) = path.parent() {
        std::fs::create_dir_all(folder).map_err(|err| format!("cannot make {}: {err}", folder.display()))?;
    }
    std::fs::write(&path, &after).map_err(|err| format!("cannot write {}: {err}", path.display()))?;
    record(history, vec![FileChange { file: path, before, after }])
}

#[tauri::command]
pub fn translation_table(state: State<WatchState>) -> Result<TranslationTable, String> {
    let (_, dir) = open_project_of(&state)?;
    let project = clarity_project::load_modules(&dir).map_err(|err| format!("cannot read {}: {err}", dir.display()))?;
    Ok(translations::table(&project))
}

/// Gives `key` the value `value` in `language`. The default language gives
/// every key a value; another may leave one empty, which shows the default's.
#[tauri::command]
pub fn set_translation(language: String, key: String, value: String, state: State<WatchState>, history: State<HistoryState>) -> Result<Preview, String> {
    let (_, dir) = open_project_of(&state)?;
    let project = clarity_project::load_modules(&dir).map_err(|err| format!("cannot read {}: {err}", dir.display()))?;
    let value = value.trim();
    let default = translations::default_language(&project);
    if language == default && value.is_empty() {
        return Err(format!("{default} is the default language, so {key} needs a value in it"));
    }
    let path = file_of(&dir, &language)?;
    let before = std::fs::read_to_string(&path).ok();
    let after = translations::set(before.as_deref(), &language, &key, value);
    write(path, before, after, &history)?;
    load(&dir)
}

/// Points `key` in `language` at `file` of the resources: what a markdown
/// or an image shows by its key in that language.
#[tauri::command]
pub fn set_translation_resource(language: String, key: String, file: String, state: State<WatchState>, history: State<HistoryState>) -> Result<Preview, String> {
    let (_, dir) = open_project_of(&state)?;
    let path = file_of(&dir, &language)?;
    let before = std::fs::read_to_string(&path).ok();
    let after = translations::set_value(before.as_deref(), &language, &key, &translations::Value::Resource(file.trim().to_string()));
    write(path, before, after, &history)?;
    load(&dir)
}

/// Starts the file of `language` with every key the project has, each
/// empty until it is translated.
#[tauri::command]
pub fn add_language(language: String, state: State<WatchState>, history: State<HistoryState>) -> Result<Preview, String> {
    let (_, dir) = open_project_of(&state)?;
    let language = language.trim().to_string();
    let path = file_of(&dir, &language)?;
    if path.exists() {
        return Err(format!("the project has the language {language} already"));
    }
    let project = clarity_project::load_modules(&dir).map_err(|err| format!("cannot read {}: {err}", dir.display()))?;
    let default = translations::default_language(&project);
    let keys: Vec<String> = match project.translations.get(&default) {
        Some(values) => values.keys().cloned().collect(),
        None => translations::keys(&project).into_iter().map(|k| k.key).collect(),
    };
    let text = new_language_file(&language, &keys, project.resource_keys.get(&default));
    write(path, None, text, &history)?;
    load(&dir)
}

/// The file of a new language: every text key empty until it is translated,
/// and every key that points at a resource pointing at the default
/// language's file until it is pointed at its own.
fn new_language_file(language: &str, keys: &[String], resources: Option<&std::collections::BTreeMap<String, String>>) -> String {
    let mut text = format!("translation {language}\n");
    for key in keys {
        text.push_str(&format!("    {key} \"\"\n"));
    }
    for (key, file) in resources.into_iter().flatten() {
        text = translations::set_value(Some(&text), language, key, &translations::Value::Resource(file.clone()));
    }
    text
}

/// Renames key `from` as `to` wherever the project names it: in every module
/// and every language file, one change for Undo.
#[tauri::command]
pub fn rename_key(from: String, to: String, state: State<WatchState>, history: State<HistoryState>) -> Result<Preview, String> {
    let (_, dir) = open_project_of(&state)?;
    let to = to.trim().to_string();
    if !clarity_spec::names::is_camel_case(&to) {
        return Err(clarity_spec::names::camel_case_problem("key", &to));
    }
    let project = clarity_project::load_modules(&dir).map_err(|err| format!("cannot read {}: {err}", dir.display()))?;
    if project.translations.values().any(|values| values.contains_key(&to)) {
        return Err(format!("the project has a key {to} already"));
    }
    let read = |path: &Path| std::fs::read_to_string(path).map_err(|err| format!("cannot read {}: {err}", path.display()));
    let mut changes = Vec::new();
    let modules = clarity_spec::spec_files(&dir).map_err(|err| format!("cannot read {}: {err}", dir.display()))?;
    for path in modules {
        let before = read(&path)?;
        let after = clarity_spec::keyed::rename_key_in_module(&before, &from, &to);
        if after != before {
            changes.push(FileChange { file: path, before: Some(before), after });
        }
    }
    let folder = dir.join(TRANSLATIONS_DIR);
    let languages = if folder.is_dir() { clarity_spec::spec_files(&folder).map_err(|err| format!("cannot read {}: {err}", folder.display()))? } else { Vec::new() };
    for path in languages {
        let before = read(&path)?;
        let after = clarity_spec::keyed::rename_key_in_language(&before, &from, &to);
        if after != before {
            changes.push(FileChange { file: path, before: Some(before), after });
        }
    }
    for change in &changes {
        std::fs::write(&change.file, &change.after).map_err(|err| format!("cannot write {}: {err}", change.file.display()))?;
    }
    record(&history, changes)?;
    load(&dir)
}

/// Makes every text people read a key, in every module, with a file per
/// language giving each key its text (the default language's from the
/// texts as written, the others' from their old translations): Make texts
/// keys, under Problems. One change for Undo.
#[tauri::command]
pub fn make_texts_keys(state: State<WatchState>, history: State<HistoryState>) -> Result<Preview, String> {
    let (_, dir) = open_project_of(&state)?;
    let project = clarity_project::load_modules(&dir).map_err(|err| format!("cannot read {}: {err}", dir.display()))?;
    let default = translations::default_language(&project);
    let read_all = |folder: &Path| -> Result<Vec<(PathBuf, String)>, String> {
        if !folder.is_dir() {
            return Ok(Vec::new());
        }
        clarity_spec::spec_files(folder)
            .map_err(|err| format!("cannot read {}: {err}", folder.display()))?
            .into_iter()
            .map(|p| std::fs::read_to_string(&p).map(|t| (p.clone(), t)).map_err(|err| format!("cannot read {}: {err}", p.display())))
            .collect()
    };
    let modules = read_all(&dir)?;
    let folder = dir.join(TRANSLATIONS_DIR);
    let languages = read_all(&folder)?;
    let names = |files: &[(PathBuf, String)]| files.iter().map(|(p, t)| (p.display().to_string(), t.clone())).collect::<Vec<_>>();
    let (module_names, language_names) = (names(&modules), names(&languages));
    let made = clarity_spec::keyed::make_keys(
        &module_names.iter().map(|(p, t)| (p.as_str(), t.as_str())).collect::<Vec<_>>(),
        &language_names.iter().map(|(p, t)| (p.as_str(), t.as_str())).collect::<Vec<_>>(),
        &default,
    );
    let mut changes = Vec::new();
    for ((path, before), (_, after)) in modules.iter().zip(&made.modules) {
        if before != after {
            changes.push(FileChange { file: path.clone(), before: Some(before.clone()), after: after.clone() });
        }
    }
    for language in made.languages.keys() {
        let path = folder.join(format!("{language}.{}", clarity_spec::SPEC_EXTENSION));
        let before = languages.iter().find(|(p, _)| *p == path).map(|(_, t)| t.clone());
        let after = made.language_file(language);
        if before.as_deref() != Some(after.as_str()) {
            changes.push(FileChange { file: path, before, after });
        }
    }
    std::fs::create_dir_all(&folder).map_err(|err| format!("cannot make {}: {err}", folder.display()))?;
    for change in &changes {
        std::fs::write(&change.file, &change.after).map_err(|err| format!("cannot write {}: {err}", change.file.display()))?;
    }
    record(&history, changes)?;
    load(&dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_language_has_every_key_its_resources_pointing_at_the_defaults() {
        let resources = std::collections::BTreeMap::from([("gameRules".to_string(), "rules-en.md".to_string())]);
        let text = new_language_file("nl", &["saveCaption".into()], Some(&resources));
        assert_eq!(text, "translation nl\n    saveCaption \"\"\n    gameRules resource \"rules-en.md\"\n");
    }
}
