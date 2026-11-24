//! The names and pruning of SQLite backups (docs/ledger/backups.md): a copy
//! in the backups folder the project names (a folder of the data source's
//! name in it), named `<database>-<when>-<why>.db`
//! with the time in UTC, as Clarity and every runtime name it
//! (conformance/backups.json). The newest ten automatic backups are kept;
//! a manual one is the person's and stays. clarity-migrate takes the one
//! before migration steps; the program does not migrate.

use std::path::{Path, PathBuf};

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

/// A backup's file name read back: when (UTC), its count within that second (1, or the n of `_n`), and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Named {
    pub when: String,
    pub count: u32,
    pub why: String,
}

impl Named {
    /// Whether it was taken automatically (any reason but `manual`), so pruning may remove it.
    pub fn automatic(&self) -> bool {
        self.why != "manual"
    }
}

/// Whether a reason is one a backup's name holds: lower case words joined by hyphens, digits allowed.
pub fn reasonable_filename(why: &str) -> bool {
    !why.is_empty() && why.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') && !why.starts_with('-')
}

/// A backup file of the database named `database` (its file's name without
/// `.db`), as every backup is named; none for any other file.
pub fn parse(database: &str, file: &str) -> Option<Named> {
    let rest = file.strip_prefix(database)?.strip_prefix('-')?.strip_suffix(".db")?;
    let (when, why) = rest.split_once('-')?;
    let b = when.as_bytes();
    let well_formed = b.len() == 16 && b[8] == b'T' && b[15] == b'Z' && b[..8].iter().chain(&b[9..15]).all(u8::is_ascii_digit);
    if !well_formed {
        return None;
    }
    // A second backup of the same second carries `_2`: its reason is the same.
    let (why, count) = match why.split_once('_') {
        Some((why, n)) => (why, n.parse().ok()?),
        None => (why, 1),
    };
    reasonable_filename(why).then(|| Named { when: when.to_string(), count, why: why.to_string() })
}

/// Removes the automatic backups in `folder` of the database named `database`
/// (its file's name without `.db`) beyond the newest ten; manual ones, other
/// databases' and other files stay.
pub fn prune(folder: &Path, database: &str) {
    let name = database;
    let Ok(entries) = std::fs::read_dir(folder) else { return };
    let mut automatic: Vec<(String, u32, PathBuf)> = entries
        .flatten()
        .filter_map(|e| {
            let file = e.file_name().to_string_lossy().to_string();
            parse(name, &file).filter(Named::automatic).map(|n| (n.when, n.count, e.path()))
        })
        .collect();
    automatic.sort_by(|a, b| (&b.0, b.1).cmp(&(&a.0, a.1)));
    for (_, _, old) in automatic.into_iter().skip(KEPT_AUTOMATIC) {
        let _ = std::fs::remove_file(old);
    }
}

