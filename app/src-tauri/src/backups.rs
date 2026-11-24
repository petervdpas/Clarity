//! Backups of the project's databases in the Database section of the
//! Project tab and for the agent (docs/ledger/backups.md): take one, list
//! them, restore one, delete one, each with the one version the clarity
//! command uses too (`clarity_project::backups`), in the project's backups folder.

use std::path::Path;

use serde::Serialize;
use tauri::{AppHandle, State};
use ts_rs::TS;

use super::{WatchState, open_project_of};

/// A backup of a database, as the Database section lists it.
#[derive(Serialize, TS, Clone, Debug, PartialEq)]
#[ts(export)]
pub struct BackupEntry {
    /// Its file name, which restore and delete name it by.
    pub file: String,
    /// When it was taken, in UTC: `20261105T213012Z`.
    pub when: String,
    /// Why: `manual`, `before-migration-0006`, `before-restore`.
    pub why: String,
    #[ts(type = "number")]
    pub bytes: u64,
}

/// A data source's backups, or why Clarity does not back it up.
#[derive(Serialize, TS, Clone, Debug, PartialEq)]
#[ts(export)]
pub struct BackupList {
    pub source: String,
    /// The database's file, when Clarity backs it up.
    pub database: Option<String>,
    /// Its backups folder: a folder of the source's name in the project's backups folder.
    pub folder: Option<String>,
    /// The newest first.
    pub backups: Vec<BackupEntry>,
    /// Why there is no backing up here: no backups folder, an external source, a server database.
    pub unavailable: Option<String>,
    /// What the last action did, to show.
    pub done: Option<String>,
}

fn entries(all: Vec<clarity_db::backup::Backup>) -> Vec<BackupEntry> {
    all.into_iter().map(|b| BackupEntry { file: b.file, when: b.when, why: b.why, bytes: b.bytes }).collect()
}

/// The backups of data source `source` of the project in `dir`.
pub(crate) fn list(dir: &Path, source: &str) -> BackupList {
    match clarity_project::backups::list(dir, Some(source)) {
        Ok((found, all)) => BackupList {
            source: found.source,
            database: Some(found.database.display().to_string()),
            folder: Some(found.folder.display().to_string()),
            backups: entries(all),
            unavailable: None,
            done: None,
        },
        Err(why) => BackupList { source: source.to_string(), database: None, folder: None, backups: Vec::new(), unavailable: Some(why), done: None },
    }
}

/// Takes a backup of data source `source`'s database.
pub(crate) fn take(dir: &Path, source: &str) -> Result<BackupList, String> {
    let (_, taken) = clarity_project::backups::take(dir, Some(source))?;
    Ok(BackupList { done: Some(format!("Backed up {source} to {}", taken.file)), ..list(dir, source) })
}

/// Restores the backup named `file` of data source `source`, after backing up what it replaces.
pub(crate) fn restore(dir: &Path, source: &str, file: &str) -> Result<BackupList, String> {
    let (_, before) = clarity_project::backups::restore(dir, Some(source), file)?;
    let kept = if before.file == file { String::new() } else { format!("; what it replaced is kept as {}", before.file) };
    Ok(BackupList { done: Some(format!("Restored {source} from {file}{kept}")), ..list(dir, source) })
}

/// Deletes the backup named `file` of data source `source`.
pub(crate) fn delete(dir: &Path, source: &str, file: &str) -> Result<BackupList, String> {
    clarity_project::backups::delete(dir, Some(source), file)?;
    Ok(BackupList { done: Some(format!("Deleted {file}")), ..list(dir, source) })
}

#[tauri::command]
pub fn list_backups(source: String, state: State<WatchState>) -> Result<BackupList, String> {
    let (_, dir) = open_project_of(&state)?;
    Ok(list(&dir, &source))
}

#[tauri::command]
pub fn take_backup(source: String, state: State<WatchState>) -> Result<BackupList, String> {
    let (_, dir) = open_project_of(&state)?;
    take(&dir, &source)
}

/// Restores a backup; the application Run started is stopped first, since it has the database open.
#[tauri::command]
pub fn restore_backup(source: String, file: String, app: AppHandle, state: State<WatchState>) -> Result<BackupList, String> {
    let (_, dir) = open_project_of(&state)?;
    crate::build::stop(&app);
    restore(&dir, &source, &file)
}

#[tauri::command]
pub fn delete_backup(source: String, file: String, state: State<WatchState>) -> Result<BackupList, String> {
    let (_, dir) = open_project_of(&state)?;
    delete(&dir, &source, &file)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A project in a folder of its own with one SQLite database holding a table.
    fn project(name: &str) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!("clarity-ide-backups-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("shop/data")).unwrap();
        std::fs::write(root.join("shop.clarity"), "project Shop\n    stack rust desktop\n    backups \"data/backups\"\n    datasource main sqlite \"data/shop.db\"\n    datasource orders postgres \"env:ORDERS_DB\"\n").unwrap();
        let db = rusqlite::Connection::open(root.join("shop/data/shop.db")).unwrap();
        db.execute_batch("CREATE TABLE item (id INTEGER PRIMARY KEY); INSERT INTO item DEFAULT VALUES;").unwrap();
        root.join("shop")
    }

    #[test]
    fn the_database_section_takes_lists_restores_and_deletes_backups() {
        let dir = project("all");
        assert!(list(&dir, "main").backups.is_empty());
        let taken = take(&dir, "main").unwrap();
        assert_eq!(1, taken.backups.len());
        assert!(taken.done.unwrap().starts_with("Backed up main"));
        let file = taken.backups[0].file.clone();
        let restored = restore(&dir, "main", &file).unwrap();
        assert!(restored.done.unwrap().contains("what it replaced is kept"));
        assert_eq!(2, restored.backups.len());
        assert!(restore(&dir, "main", "../shop.db").is_err(), "a file name, never a path");
        let deleted = delete(&dir, "main", &file).unwrap();
        assert_eq!(1, deleted.backups.len());
        let server = list(&dir, "orders");
        assert!(server.unavailable.unwrap().contains("administrator"));
        assert!(dir.join("data/backups/main").join(&deleted.backups[0].file).is_file(), "in the project's backups folder");
        let _ = std::fs::remove_dir_all(dir.parent().unwrap());
    }

    /// A project without a backups folder takes no backups; the section says where to set it.
    #[test]
    fn without_a_backups_folder_the_section_says_where_to_set_it() {
        let dir = project("off");
        let file = dir.with_extension("clarity");
        let text = std::fs::read_to_string(&file).unwrap().replace("    backups \"data/backups\"\n", "");
        std::fs::write(&file, text).unwrap();
        let listed = list(&dir, "main");
        assert!(listed.unavailable.unwrap().contains("Project settings"));
        assert!(take(&dir, "main").is_err());
        let _ = std::fs::remove_dir_all(dir.parent().unwrap());
    }
}
