//! A backup of an SQLite database before migration steps change it
//! (docs/BACKUP-PLAN.md): a copy beside it in `backups/`, made with
//! `VACUUM INTO`, named `<database>-<when>-before-migration-NNNN.db` with
//! the time in UTC, as Clarity and every runtime name it
//! (conformance/backups.json). The newest ten automatic backups are kept;
//! a manual one is the person's and stays.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::dialect::Dialect;
use crate::sql::SqlValue;
use crate::store::Driver;
use crate::{Error, Result};

/// How many automatic backups each database keeps.
pub const KEPT_AUTOMATIC: usize = 10;

/// A moment, in seconds since 1970 (UTC), as a backup names it: `20261105T205905Z`.
pub fn stamp(seconds: i64) -> String {
    let (days, rest) = (seconds.div_euclid(86_400), seconds.rem_euclid(86_400));
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

/// The file of a backup of the database named `database` (its file's name without `.db`) taken at `seconds` before step `step`.
pub fn file_name(database: &str, seconds: i64, step: usize) -> String {
    format!("{database}-{}-before-migration-{step:04}.db", stamp(seconds))
}

/// A backup file of the database named `database`: its stamp, count within that second, and whether it is automatic.
fn parse(database: &str, file: &str) -> Option<(String, u32, bool)> {
    let rest = file.strip_prefix(database)?.strip_prefix('-')?.strip_suffix(".db")?;
    let (when, why) = rest.split_once('-')?;
    if when.len() != 16 || !when.ends_with('Z') {
        return None;
    }
    let (why, count) = match why.split_once('_') {
        Some((why, n)) => (why, n.parse().ok()?),
        None => (why, 1),
    };
    Some((when.to_string(), count, why != "manual"))
}

/// Removes the automatic backups of `database` beyond the newest ten; manual ones, other databases' and other files stay.
pub fn prune(database: &Path) {
    let name = database.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let dir = database.parent().unwrap_or(Path::new(".")).join("backups");
    let Ok(entries) = std::fs::read_dir(&dir) else { return };
    let mut automatic: Vec<(String, u32, PathBuf)> = entries
        .flatten()
        .filter_map(|e| {
            let file = e.file_name().to_string_lossy().to_string();
            parse(&name, &file).filter(|(_, _, auto)| *auto).map(|(when, count, _)| (when, count, e.path()))
        })
        .collect();
    automatic.sort_by(|a, b| (&b.0, b.1).cmp(&(&a.0, a.1)));
    for (_, _, old) in automatic.into_iter().skip(KEPT_AUTOMATIC) {
        let _ = std::fs::remove_file(old);
    }
}

/// Backs up the SQLite database `conn` has open before step `step` is applied to it; nothing for another kind of database or one in memory. A backup that fails stops the migration.
pub(crate) fn before_migration(conn: &mut dyn Driver, step: usize) -> Result<Option<PathBuf>> {
    if conn.dialect() != Dialect::Sqlite {
        return Ok(None);
    }
    let rows = conn.rows("SELECT file FROM pragma_database_list WHERE name = 'main'", &[])?;
    let file = match rows.first().and_then(|r| r.first()) {
        Some(SqlValue::Text(file)) if !file.is_empty() => PathBuf::from(file),
        _ => return Ok(None),
    };
    let refused = |what: String| Error::Storage(format!("the database was not migrated, since it could not be backed up first: {what}"));
    let dir = file.parent().unwrap_or(Path::new(".")).join("backups");
    std::fs::create_dir_all(&dir).map_err(|e| refused(format!("{}: {e}", dir.display())))?;
    let name = file.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "database".into());
    let seconds = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    let first = dir.join(file_name(&name, seconds, step));
    let mut path = first.clone();
    let mut n = 2;
    while path.exists() {
        path = PathBuf::from(format!("{}_{n}.db", first.with_extension("").display()));
        n += 1;
    }
    conn.execute("VACUUM INTO ?1", &[SqlValue::Text(path.to_string_lossy().to_string())]).map_err(|e| match Error::from(e) {
        Error::Storage(what) => refused(what),
        other => other,
    })?;
    prune(&file);
    Ok(Some(path))
}
