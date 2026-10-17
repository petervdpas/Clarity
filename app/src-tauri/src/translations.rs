//! The Translations table on the Project tab: every text a person sees,
//! with a column per language, written to `translations/<language>.clarity`.

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

/// Translates `text` as `translation` in `language`; an empty translation
/// removes it.
#[tauri::command]
pub fn set_translation(
    language: String,
    text: String,
    translation: String,
    state: State<WatchState>,
    history: State<HistoryState>,
) -> Result<Preview, String> {
    let (_, dir) = open_project_of(&state)?;
    let path = file_of(&dir, &language)?;
    let before = std::fs::read_to_string(&path).ok();
    let after = translations::set(before.as_deref(), &language, &text, translation.trim());
    write(path, before, after, &history)?;
    load(&dir)
}

/// Starts the translation file of `language`.
#[tauri::command]
pub fn add_language(language: String, state: State<WatchState>, history: State<HistoryState>) -> Result<Preview, String> {
    let (_, dir) = open_project_of(&state)?;
    let language = language.trim().to_string();
    let path = file_of(&dir, &language)?;
    if path.exists() {
        return Err(format!("the project has the language {language} already"));
    }
    write(path, None, translations::set(None, &language, "", ""), &history)?;
    load(&dir)
}
