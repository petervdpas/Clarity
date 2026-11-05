//! Backups of the SQLite databases a project owns (docs/BACKUP-PLAN.md).
//! A backup is a copy beside the database, in `backups/`, made with
//! SQLite's `VACUUM INTO` so it is consistent while the database is open,
//! and named for when (UTC) and why: `shop-20261105T213012Z-manual.db`.
//! The automatic backups (any reason but `manual`) are pruned to the newest
//! [`KEPT_AUTOMATIC`]; a restore first backs up what it replaces.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, OpenFlags};

/// How many automatic backups each database keeps.
pub const KEPT_AUTOMATIC: usize = 10;

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

/// The folder a database's backups are in: `backups/` beside it.
pub fn folder(database: &Path) -> PathBuf {
    database.parent().unwrap_or(Path::new(".")).join("backups")
}

fn stem(database: &Path) -> String {
    database.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "database".into())
}

/// A moment as a backup names it: `20261105T213012Z`.
pub fn stamp(at: SystemTime) -> String {
    let secs = at.duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    let (days, rest) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    // The civil date of a day count (Howard Hinnant's days_from_civil, turned around).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}{month:02}{day:02}T{:02}{:02}{:02}Z", rest / 3600, rest % 3600 / 60, rest % 60)
}

/// Whether a reason is one Clarity writes into a file name: lower case words joined by hyphens, digits allowed.
fn fair_reason(why: &str) -> bool {
    !why.is_empty() && why.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') && !why.starts_with('-')
}

/// Takes a backup of `database` for `why`; an automatic one prunes the oldest beyond the ten kept.
pub fn take(database: &Path, why: &str) -> Result<Backup, String> {
    if !fair_reason(why) {
        return Err(format!("'{why}' is no reason for a backup: lower case words joined by hyphens"));
    }
    if !database.is_file() {
        return Err(format!("there is no database at {} yet, so nothing to back up", database.display()));
    }
    let dir = folder(database);
    std::fs::create_dir_all(&dir).map_err(|e| format!("cannot make {}: {e}", dir.display()))?;
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
        prune_automatic(database);
    }
    read(database, &path).ok_or_else(|| format!("the backup {} was not written", path.display()))
}

/// A backup file of `database` as a [`Backup`]; none for a file that is not one.
fn read(database: &Path, path: &Path) -> Option<Backup> {
    let file = path.file_name()?.to_string_lossy().to_string();
    let rest = file.strip_prefix(&format!("{}-", stem(database)))?.strip_suffix(".db")?;
    let (when, why) = rest.split_once('-')?;
    let well_formed = when.len() == 16 && when.as_bytes()[8] == b'T' && when.ends_with('Z') && when[..8].bytes().all(|b| b.is_ascii_digit()) && when[9..15].bytes().all(|b| b.is_ascii_digit());
    if !well_formed {
        return None;
    }
    // A second backup of the same second carries `_2`: its reason is the same.
    let why = match why.split_once('_') {
        Some((head, n)) if n.parse::<u32>().is_ok() => head,
        Some(_) => return None,
        None => why,
    };
    if !fair_reason(why) {
        return None;
    }
    let bytes = std::fs::metadata(path).ok()?.len();
    let (when, why) = (when.to_string(), why.to_string());
    Some(Backup { path: path.to_path_buf(), file, when, why, bytes })
}

/// The backups of `database`, the newest first.
pub fn list(database: &Path) -> Vec<Backup> {
    let Ok(entries) = std::fs::read_dir(folder(database)) else { return Vec::new() };
    let mut all: Vec<Backup> = entries.flatten().filter_map(|e| read(database, &e.path())).collect();
    // The newest first: by the second it names, then its count within that second (conformance/backups.json), then when the file was written.
    let written = |b: &Backup| std::fs::metadata(&b.path).and_then(|m| m.modified()).ok();
    all.sort_by(|a, b| b.when.cmp(&a.when).then_with(|| count(&b.file).cmp(&count(&a.file))).then_with(|| written(b).cmp(&written(a))));
    all
}

/// The count of a backup within its second: 1, or the n of `_n` before `.db`.
fn count(file: &str) -> u32 {
    file.strip_suffix(".db").and_then(|f| f.rsplit_once('_')).and_then(|(_, n)| n.parse().ok()).unwrap_or(1)
}

/// Removes the automatic backups of `database` beyond the newest [`KEPT_AUTOMATIC`]; manual ones stay.
pub fn prune_automatic(database: &Path) {
    for old in list(database).into_iter().filter(|b| b.why != MANUAL).skip(KEPT_AUTOMATIC) {
        let _ = std::fs::remove_file(&old.path);
    }
}

/// `path` as a backup of `database`: in its folder, named as its backups are.
fn own_backup(database: &Path, path: &Path) -> Result<Backup, String> {
    let inside = path.parent().and_then(|p| std::path::absolute(p).ok()) == std::path::absolute(folder(database)).ok();
    match read(database, path) {
        Some(b) if inside => Ok(b),
        _ => Err(format!("{} is not a backup of {}", path.display(), database.display())),
    }
}

/// Puts the backup at `path` in place of `database`, after backing up what
/// is there now. Refuses a file that is not a backup of this database, or
/// no database, and a database something has open for writing.
pub fn restore(database: &Path, path: &Path) -> Result<Backup, String> {
    own_backup(database, path)?;
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
    let before = if database.is_file() { Some(take(database, "before-restore")?) } else { None };
    let staging = database.with_extension("db.restoring");
    std::fs::copy(path, &staging).map_err(|e| format!("cannot copy {}: {e}", path.display()))?;
    for side in ["db-wal", "db-shm"] {
        let _ = std::fs::remove_file(database.with_extension(side));
    }
    std::fs::rename(&staging, database).map_err(|e| format!("cannot put the backup in place of {}: {e}", database.display()))?;
    match before {
        Some(b) => Ok(b),
        None => own_backup(database, path),
    }
}

/// Deletes the backup at `path` of `database`; nothing else.
pub fn delete(database: &Path, path: &Path) -> Result<(), String> {
    own_backup(database, path)?;
    std::fs::remove_file(path).map_err(|e| format!("cannot delete {}: {e}", path.display()))
}
