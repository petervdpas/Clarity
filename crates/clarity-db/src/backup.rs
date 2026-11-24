//! Backups of the SQLite databases a project owns (docs/ledger/backups.md).
//! A backup is a copy in the folder the caller gives (the project decides
//! it: `clarity_project::backups`, a folder of the data source's name in the
//! project's backups folder), made with
//! SQLite's `VACUUM INTO` so it is consistent while the database is open,
//! and named for when (UTC) and why: `shop-20261105T213012Z-manual.db`.
//! The automatic backups (any reason but `manual`) are pruned to the newest
//! [`KEPT_AUTOMATIC`]; a restore first backs up what it replaces.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, OpenFlags};

/// How many automatic backups each database keeps.
pub const KEPT_AUTOMATIC: usize = clarity_runtime::backup::KEPT_AUTOMATIC;

/// The reason of a backup the person asked for.
pub const MANUAL: &str = "manual";

/// A backup of a database: its file, when it was taken and why.
#[derive(Debug, Clone, PartialEq)]
pub struct Backup {
    pub path: PathBuf,
    /// Its file name: `shop-20261105T213012Z-manual.db`.
    pub file: String,
    /// When, in UTC: `20261105T213012Z`.
    pub when: String,
    /// Why: `manual`, `before-migration-0006`, `before-restore`.
    pub why: String,
    pub bytes: u64,
}

fn stem(database: &Path) -> String {
    database.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "database".into())
}

/// A moment as a backup names it: `20261105T213012Z` (the runtime's naming, the one every backup has).
pub fn stamp(at: SystemTime) -> String {
    clarity_runtime::backup::stamp(at.duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0))
}


/// Takes a backup of `database` into `folder` for `why`, making the folder
/// when it is not there; an automatic one prunes the oldest beyond the ten kept.
pub fn take(database: &Path, folder: &Path, why: &str) -> Result<Backup, String> {
    if !clarity_runtime::backup::reasonable_filename(why) {
        return Err(format!("'{why}' is no reason for a backup: lower case words joined by hyphens"));
    }
    if !database.is_file() {
        return Err(format!("there is no database at {} yet, so nothing to back up", database.display()));
    }
    let dir = folder;
    std::fs::create_dir_all(dir).map_err(|e| format!("cannot make {}: {e}", dir.display()))?;
    let when = stamp(SystemTime::now());
    let base = format!("{}-{when}-{why}", stem(database));
    let mut path = dir.join(format!("{base}.db"));
    let mut n = 2;
    while path.exists() {
        // Reasons hold no `_`, so the count of a second backup of the same second reads apart from its reason.
        path = dir.join(format!("{base}_{n}.db"));
        n += 1;
    }
    let db = Connection::open_with_flags(database, OpenFlags::SQLITE_OPEN_READ_WRITE).map_err(|e| format!("cannot open {}: {e}", database.display()))?;
    db.execute("VACUUM INTO ?1", [path.to_string_lossy().as_ref()]).map_err(|e| format!("cannot back up {}: {e}", database.display()))?;
    drop(db);
    if why != MANUAL {
        prune_automatic(database, folder);
    }
    read(database, &path).ok_or_else(|| format!("the backup {} was not written", path.display()))
}

/// A backup file of `database` as a [`Backup`]; none for a file that is not one (the runtime's reading of backup names).
fn read(database: &Path, path: &Path) -> Option<Backup> {
    let file = path.file_name()?.to_string_lossy().to_string();
    let named = clarity_runtime::backup::parse(&stem(database), &file)?;
    let bytes = std::fs::metadata(path).ok()?.len();
    Some(Backup { path: path.to_path_buf(), file, when: named.when, why: named.why, bytes })
}

/// The backups of `database` in `folder`, the newest first.
pub fn list(database: &Path, folder: &Path) -> Vec<Backup> {
    let Ok(entries) = std::fs::read_dir(folder) else { return Vec::new() };
    let mut all: Vec<Backup> = entries.flatten().filter_map(|e| read(database, &e.path())).collect();
    // The newest first: by the second it names, then its count within that second (conformance/backups.json), then when the file was written.
    let written = |b: &Backup| std::fs::metadata(&b.path).and_then(|m| m.modified()).ok();
    all.sort_by(|a, b| b.when.cmp(&a.when).then_with(|| count(database, &b.file).cmp(&count(database, &a.file))).then_with(|| written(b).cmp(&written(a))));
    all
}

/// The count of a backup within its second: 1, or the n of `_n` before `.db`.
fn count(database: &Path, file: &str) -> u32 {
    clarity_runtime::backup::parse(&stem(database), file).map_or(1, |n| n.count)
}

/// Removes the automatic backups of `database` in `folder` beyond the newest [`KEPT_AUTOMATIC`]; manual ones stay (the runtime's pruning, which clarity-migrate uses too).
pub fn prune_automatic(database: &Path, folder: &Path) {
    clarity_runtime::backup::prune(folder, &stem(database));
}

/// `path` as a backup of `database`: in `folder`, named as its backups are.
fn own_backup(database: &Path, folder: &Path, path: &Path) -> Result<Backup, String> {
    let inside = path.parent().and_then(|p| std::path::absolute(p).ok()) == std::path::absolute(folder).ok();
    match read(database, path) {
        Some(b) if inside => Ok(b),
        _ => Err(format!("{} is not a backup of {}", path.display(), database.display())),
    }
}

/// Puts the backup at `path` in place of `database`, after backing up what
/// is there now into `folder`. Refuses a file that is not a backup of this
/// database in `folder`, or no database, and a database something has open
/// for writing.
pub fn restore(database: &Path, folder: &Path, path: &Path) -> Result<Backup, String> {
    own_backup(database, folder, path)?;
    let check = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(|e| format!("{} is not a database: {e}", path.display()))?;
    let fine: String = check.query_row("PRAGMA quick_check", [], |r| r.get(0)).map_err(|e| format!("{} is not a database: {e}", path.display()))?;
    if fine != "ok" {
        return Err(format!("{} is damaged: {fine}", path.display()));
    }
    drop(check);
    // Nothing may be writing to it while it is replaced: an application running on it is closed first.
    if database.is_file() {
        let db = Connection::open(database).map_err(|e| format!("cannot open {}: {e}", database.display()))?;
        db.busy_timeout(std::time::Duration::from_millis(200)).ok();
        if db.execute_batch("BEGIN EXCLUSIVE; COMMIT;").is_err() {
            return Err(format!("{} is in use; close the application that has it open, then restore", database.display()));
        }
    }
    let before = if database.is_file() { Some(take(database, folder, "before-restore")?) } else { None };
    let staging = database.with_extension("db.restoring");
    std::fs::copy(path, &staging).map_err(|e| format!("cannot copy {}: {e}", path.display()))?;
    for side in ["db-wal", "db-shm"] {
        let _ = std::fs::remove_file(database.with_extension(side));
    }
    std::fs::rename(&staging, database).map_err(|e| format!("cannot put the backup in place of {}: {e}", database.display()))?;
    match before {
        Some(b) => Ok(b),
        None => own_backup(database, folder, path),
    }
}

/// Deletes the backup at `path` of `database` in `folder`; nothing else.
pub fn delete(database: &Path, folder: &Path, path: &Path) -> Result<(), String> {
    own_backup(database, folder, path)?;
    std::fs::remove_file(path).map_err(|e| format!("cannot delete {}: {e}", path.display()))
}
