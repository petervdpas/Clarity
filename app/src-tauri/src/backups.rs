//! Backups of the project's databases in the Database section of the
//! Project tab and for the agent (docs/BACKUP-PLAN.md): take one, list
//! them, restore one, delete one. The work is `clarity_db::backup`'s; which
//! database a data source has, and whether Clarity backs it up, is
//! `clarity_project::database_to_back_up`'s.

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
    /// The newest first.
    pub backups: Vec<BackupEntry>,
    /// Why there is no backing up here: an external source, a server database.
    pub unavailable: Option<String>,
    /// What the last action did, to show.
    pub done: Option<String>,
}

fn entries(database: &Path) -> Vec<BackupEntry> {
    clarity_db::backup::list(database).into_iter().map(|b| BackupEntry { file: b.file, when: b.when, why: b.why, bytes: b.bytes }).collect()
}

/// The backups of data source `source` of the project in `dir`.
pub(crate) fn list(dir: &Path, source: &str) -> BackupList {
    match clarity_project::database_to_back_up(dir, Some(source)) {
        Ok((name, database)) => BackupList { source: name, database: Some(database.display().to_string()), backups: entries(&database), unavailable: None, done: None },
        Err(why) => BackupList { source: source.to_string(), database: None, backups: Vec::new(), unavailable: Some(why), done: None },
    }
}

/// Takes a backup of data source `source`'s database.
pub(crate) fn take(dir: &Path, source: &str) -> Result<BackupList, String> {
    let (_, database) = clarity_project::database_to_back_up(dir, Some(source))?;
    let taken = clarity_db::backup::take(&database, clarity_db::backup::MANUAL)?;
    Ok(BackupList { done: Some(format!("Backed up {source} to {}", taken.file)), ..list(dir, source) })
}

/// The backup named `file` of `database`, in its folder.
fn backup_file(database: &Path, file: &str) -> Result<std::path::PathBuf, String> {
    if file.contains('/') || file.contains('\\') {
        return Err(format!("'{file}' is a backup's file name, not a path"));
    }
    Ok(clarity_db::backup::folder(database).join(file))
}

/// Restores the backup named `file` of data source `source`, after backing up what it replaces.
pub(crate) fn restore(dir: &Path, source: &str, file: &str) -> Result<BackupList, String> {
    let (_, database) = clarity_project::database_to_back_up(dir, Some(source))?;
    let before = clarity_db::backup::restore(&database, &backup_file(&database, file)?)?;
    let kept = if before.file == file { String::new() } else { format!("; what it replaced is kept as {}", before.file) };
    Ok(BackupList { done: Some(format!("Restored {source} from {file}{kept}")), ..list(dir, source) })
}

/// Deletes the backup named `file` of data source `source`.
pub(crate) fn delete(dir: &Path, source: &str, file: &str) -> Result<BackupList, String> {
    let (_, database) = clarity_project::database_to_back_up(dir, Some(source))?;
    clarity_db::backup::delete(&database, &backup_file(&database, file)?)?;
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
        std::fs::write(root.join("shop.clarity"), "project Shop\n    stack rust desktop\n    datasource main sqlite \"data/shop.db\"\n    datasource orders postgres \"env:ORDERS_DB\"\n").unwrap();
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
        let _ = std::fs::remove_dir_all(dir.parent().unwrap());
    }
}
