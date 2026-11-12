//! The file browser's memory: the projects
//! opened last, newest first, and the folder each use of the browser was
//! left in. The user: "10 recent, remember last folder". Kept in
//! `recent.json` in Clarity's own folder, beside `ide.json`, and changed key
//! by key: what else the file holds stays, and a file that is not JSON is
//! refused, never overwritten.

use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

/// How many projects the list keeps.
pub const KEPT: usize = 10;

const PROJECTS: &str = "projects";
const FOLDERS: &str = "lastFolders";

/// Where the memory is kept.
pub fn file() -> PathBuf {
    crate::clarity_home().join("recent.json")
}

/// The projects opened last, newest first, those still there.
pub fn recent() -> Vec<PathBuf> {
    recent_in(&file())
}

pub fn recent_in(file: &Path) -> Vec<PathBuf> {
    let Ok(Value::Object(settings)) = read(file) else { return Vec::new() };
    settings.get(PROJECTS).and_then(Value::as_array).map(|list| list.iter().filter_map(Value::as_str).map(PathBuf::from).filter(|p| p.is_file()).collect()).unwrap_or_default()
}

/// Puts `project` at the top of the list, once, keeping the last 10.
pub fn remember_project(project: &Path) -> Result<(), String> {
    remember_project_in(&file(), project)
}

pub fn remember_project_in(file: &Path, project: &Path) -> Result<(), String> {
    change(file, |settings| {
        let shown = project.display().to_string();
        let mut list: Vec<Value> = settings.get(PROJECTS).and_then(Value::as_array).cloned().unwrap_or_default();
        list.retain(|p| p.as_str() != Some(shown.as_str()));
        list.insert(0, Value::String(shown));
        list.truncate(KEPT);
        settings.insert(PROJECTS.into(), Value::Array(list));
    })
}

/// The folder `use_` (open-project, folder, a file kind) was left in, while it is there.
pub fn last_folder(use_: &str) -> Option<PathBuf> {
    last_folder_in(&file(), use_)
}

pub fn last_folder_in(file: &Path, use_: &str) -> Option<PathBuf> {
    let Ok(Value::Object(settings)) = read(file) else { return None };
    settings.get(FOLDERS)?.get(use_)?.as_str().map(PathBuf::from).filter(|p| p.is_dir())
}

/// Remembers the folder `use_` was left in.
pub fn remember_folder(use_: &str, folder: &Path) -> Result<(), String> {
    remember_folder_in(&file(), use_, folder)
}

pub fn remember_folder_in(file: &Path, use_: &str, folder: &Path) -> Result<(), String> {
    change(file, |settings| {
        let folders = settings.entry(FOLDERS).or_insert_with(|| Value::Object(Map::new()));
        if !folders.is_object() {
            *folders = Value::Object(Map::new());
        }
        if let Some(folders) = folders.as_object_mut() {
            folders.insert(use_.to_string(), Value::String(folder.display().to_string()));
        }
    })
}

/// The file's JSON: an empty object when it is not there, an error when it is not JSON.
fn read(file: &Path) -> Result<Value, String> {
    match std::fs::read_to_string(file) {
        Ok(text) => serde_json::from_str(&text).map_err(|err| format!("{} is not JSON: {err}", file.display())),
        Err(_) => Ok(Value::Object(Map::new())),
    }
}

/// Reads the file, changes it, and writes it beside and into place, so a reader never sees half a file.
fn change(file: &Path, edit: impl FnOnce(&mut Map<String, Value>)) -> Result<(), String> {
    let Value::Object(mut settings) = read(file)? else {
        return Err(format!("{} is not a JSON object; it is left as it is", file.display()));
    };
    edit(&mut settings);
    if let Some(folder) = file.parent() {
        std::fs::create_dir_all(folder).map_err(|err| format!("cannot make {}: {err}", folder.display()))?;
    }
    let text = serde_json::to_string_pretty(&Value::Object(settings)).map_err(|err| err.to_string())?;
    let beside = file.with_extension("json.new");
    std::fs::write(&beside, text).map_err(|err| format!("cannot write {}: {err}", beside.display()))?;
    std::fs::rename(&beside, file).map_err(|err| format!("cannot write {}: {err}", file.display()))
}
