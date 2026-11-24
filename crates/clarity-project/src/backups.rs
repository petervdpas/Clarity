//! Backups of a project's databases (docs/ledger/backups.md): the one version
//! of taking, listing, restoring and deleting them, which the IDE, its agent
//! and the clarity command all call. The project decides: its `backups` line
//! names a folder under the project's folder, and each data source's backups
//! go in a folder of its name inside it. Without the line the project takes
//! no backups. The work on the files is `clarity_db::backup`'s.

use std::path::{Path, PathBuf};

use clarity_db::backup::{self, Backup};

use crate::{DataSource, Settings, SourceKind, load, source_word};

/// Why a project without a backups folder takes no backups, and where to set it.
pub const NO_BACKUPS: &str =
    "the project takes no backups: set its backups folder, under the project's folder, with a line like backups \"data/backups\" in the project file, or in Project settings in the IDE";

/// A database Clarity backs up: its data source, its file and its backups folder.
#[derive(Debug, Clone, PartialEq)]
pub struct BackedUp {
    pub source: String,
    pub database: PathBuf,
    pub folder: PathBuf,
}

/// The backups folder of data source `source` of the project in folder `dir`:
/// a folder of its name in the folder the project's `backups` line names;
/// none without the line.
pub fn folder_of(dir: &Path, settings: &Settings, source: &str) -> Option<PathBuf> {
    let joined = dir.join(settings.backups.as_deref()?).join(source);
    Some(std::path::absolute(&joined).unwrap_or(joined))
}

/// The file of an SQLite database where Clarity's Run opens it: in the
/// project's data folder under its own file name when the project keeps one
/// (`clarity_migrate::in_data_folder`), else where the project file puts it.
fn database_of(dir: &Path, settings: &Settings, source: &DataSource) -> PathBuf {
    let file = crate::sqlite_path(dir, source);
    match (crate::data_folder(dir, settings), file.file_name()) {
        (Some(folder), Some(name)) => folder.join(name),
        _ => file,
    }
}

/// The database Clarity backs up for data source `source` of the project in
/// folder `dir` (the first SQLite one it owns when none is named) and its
/// backups folder. An external source is someone else's, a server database
/// its administrator's for now, and a project without a backups folder takes
/// no backups: each says so.
pub fn backed_up(dir: &Path, source: Option<&str>) -> Result<BackedUp, String> {
    let file = dir.with_extension(clarity_spec::SPEC_EXTENSION);
    let settings = load(&file).settings.ok_or_else(|| format!("{} cannot be read", file.display()))?;
    let found = match source {
        Some(name) => settings.datasources.iter().find(|s| s.name == name).ok_or_else(|| format!("the project has no data source {name}"))?,
        None => settings
            .datasources
            .iter()
            .find(|s| s.kind == SourceKind::Sqlite && !s.external)
            .ok_or_else(|| "the project owns no SQLite database to back up".to_string())?,
    };
    if found.external {
        return Err(format!("data source {} is external: its database is someone else's, and Clarity neither backs it up nor restores it", found.name));
    }
    match found.kind {
        SourceKind::Sqlite if found.url.starts_with("env:") || found.url.starts_with("setting:") => {
            return Err(format!("data source {} is found while the application runs ({}); Clarity backs up a database file the project names", found.name, found.url));
        }
        SourceKind::Sqlite => {}
        SourceKind::Rest => return Err(format!("data source {} is a web API; its data is kept by the service behind it", found.name)),
        other => return Err(format!("data source {} is {}; its administrator backs it up until Clarity backs up server databases itself", found.name, source_word(other))),
    }
    let folder = folder_of(dir, &settings, &found.name).ok_or_else(|| NO_BACKUPS.to_string())?;
    Ok(BackedUp { source: found.name.clone(), database: database_of(dir, &settings, found), folder })
}

/// Takes a backup the person asked for of data source `source`'s database.
pub fn take(dir: &Path, source: Option<&str>) -> Result<(BackedUp, Backup), String> {
    let found = backed_up(dir, source)?;
    let taken = backup::take(&found.database, &found.folder, backup::MANUAL)?;
    Ok((found, taken))
}

/// The backups of data source `source`'s database, the newest first.
pub fn list(dir: &Path, source: Option<&str>) -> Result<(BackedUp, Vec<Backup>), String> {
    let found = backed_up(dir, source)?;
    let all = backup::list(&found.database, &found.folder);
    Ok((found, all))
}

/// The backup named `file` in the backups folder of `found`; a path is refused.
fn named(found: &BackedUp, file: &str) -> Result<PathBuf, String> {
    if file.is_empty() || file.contains('/') || file.contains('\\') || file == "." || file == ".." {
        return Err(format!("'{file}' is not a backup's file name: name one as the list of backups gives it, not a path"));
    }
    Ok(found.folder.join(file))
}

/// Puts the backup named `file` in place of data source `source`'s database,
/// after backing up what it replaces; gives that backup (the restored one
/// when there was no database). Whatever has the database open is closed first.
pub fn restore(dir: &Path, source: Option<&str>, file: &str) -> Result<(BackedUp, Backup), String> {
    let found = backed_up(dir, source)?;
    let before = backup::restore(&found.database, &found.folder, &named(&found, file)?)?;
    Ok((found, before))
}

/// Deletes the backup named `file` of data source `source`'s database.
pub fn delete(dir: &Path, source: Option<&str>, file: &str) -> Result<BackedUp, String> {
    let found = backed_up(dir, source)?;
    backup::delete(&found.database, &found.folder, &named(&found, file)?)?;
    Ok(found)
}
