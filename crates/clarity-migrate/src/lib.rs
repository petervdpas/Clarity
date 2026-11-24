//! Clarity's own migration tool (docs/ledger/migrations.md, Design, 2026-11-22):
//! it applies a project's migration steps to a database of every kind
//! Clarity owns, SQLite, PostgreSQL, MySQL and SQL Server, backing up first
//! where it can, and records them in `clarity_schema`. Clarity uses it (the
//! IDE, `clarity data migrate`), and an app made with Clarity can carry it
//! (`clarity-migrate`), so its database is migrated without Clarity. A
//! database that has more steps than the app was made with is refused.

use clarity_runtime::{Db, Dialect, SqlValue, backup};
use serde::{Deserialize, Serialize};

/// A database to migrate: its data source, the SQL it speaks (`sqlite`,
/// `postgres`, `mysql`, `mssql`), where it is as the app finds it (a file,
/// a connection, `env:NAME` or `setting:<program>/<name>`), and every step
/// it should have, each a list of statements in its SQL, and the folder an
/// SQLite database is backed up into before steps change it. The generator
/// writes these (`clarity_gen::migrated_databases`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Database {
    pub source: String,
    pub dialect: String,
    pub location: String,
    pub steps: Vec<Vec<String>>,
    /// The folder its backups go in (docs/ledger/backups.md): the project's
    /// `backups` folder with the data source's name; a relative one is in the
    /// folder of the database's file. None when the project takes no backups:
    /// it is migrated without one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backups: Option<String>,
}

/// The file beside the tool, written with an app, that it reads when it is given none.
pub const MIGRATE_FILE: &str = "clarity-migrate.json";

/// The databases of the file at `path`.
pub fn read(path: &std::path::Path) -> Result<Vec<Database>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{} cannot be read: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("{} does not read: {e}", path.display()))
}

/// The file the tool reads when it is given none: `clarity-migrate.json` beside its own program.
pub fn beside() -> Result<std::path::PathBuf, String> {
    let program = std::env::current_exe().map_err(|e| format!("cannot find where clarity-migrate is: {e}"))?;
    Ok(program.parent().map(|folder| folder.join(MIGRATE_FILE)).unwrap_or_else(|| MIGRATE_FILE.into()))
}

/// `database` with its SQLite file where an app finds it when it runs with
/// CLARITY_DATA naming `folder`, as Clarity's Run starts it (conformance/data-folder.json);
/// a server's database as it is.
pub fn in_data_folder(database: &Database, folder: &std::path::Path) -> Database {
    if database.dialect != "sqlite" {
        return database.clone();
    }
    let named = folder.display().to_string();
    let variable = clarity_runtime::data_folder::VARIABLE;
    let location = clarity_runtime::data_folder::sqlite(&database.location, |name| if name == variable { Some(named.clone()) } else { std::env::var(name).ok() });
    Database { location, ..database.clone() }
}

/// What migrating a database did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// It had every step already.
    UpToDate,
    /// It got this many steps.
    Migrated(usize),
}

/// The SQL a database speaks, by its word.
pub fn dialect_of(word: &str) -> Result<Dialect, String> {
    match word {
        "sqlite" => Ok(Dialect::Sqlite),
        "postgres" => Ok(Dialect::Postgres),
        "mysql" => Ok(Dialect::Mysql),
        "mssql" => Ok(Dialect::Mssql),
        other => Err(format!("there is no database kind '{other}'; a database is sqlite, postgres, mysql or mssql")),
    }
}

/// How many steps `database` has had, and how many it should have.
pub fn state(database: &Database) -> Result<(usize, usize), String> {
    let db = connect(database)?;
    let dialect = dialect_of(&database.dialect)?;
    Ok((steps_had(&db, &database.source, dialect)?, database.steps.len()))
}

/// Applies the steps `database` has not had yet, in order, each all or
/// nothing where the database can undo it; a database that has a schema is
/// backed up first into its backups folder, when it has one. A database
/// ahead of the steps (a newer app's) is refused.
pub fn migrate(database: &Database) -> Result<Outcome, String> {
    migrate_open(&connect(database)?, &database.source, &database.steps, database.backups.as_deref().map(std::path::Path::new))
}

/// `migrate` on a database already open as `source` of `db`, backing it up
/// into `backups` (relative: in the folder of its file) first; none takes no backup.
pub fn migrate_open(db: &Db, source: &str, steps: &[Vec<String>], backups: Option<&std::path::Path>) -> Result<Outcome, String> {
    let dialect = db.dialect(source).map_err(|e| e.to_string())?;
    let had = steps_had(db, source, dialect)?;
    if had > steps.len() {
        return Err(format!(
            "{source} has {had} migration steps, more than the {} this app was made with: it belongs to a newer version of the app",
            steps.len()
        ));
    }
    if had == steps.len() {
        return Ok(Outcome::UpToDate);
    }
    // A database that already has a schema is backed up before it changes, when the project takes backups (docs/ledger/backups.md).
    if let Some(folder) = backups
        && had > 0
        && dialect == Dialect::Sqlite
    {
        backup_first(db, source, had + 1, folder)?;
    }
    for (i, statements) in steps.iter().enumerate().skip(had) {
        apply(db, source, dialect, i + 1, statements)?;
    }
    Ok(Outcome::Migrated(steps.len() - had))
}

/// How many steps the database of `source` has had (`clarity_schema`, made when it is not there).
fn steps_had(db: &Db, source: &str, dialect: Dialect) -> Result<usize, String> {
    db.batch(source, dialect.version_table()).map_err(|e| e.to_string())?;
    match db.rows(source, "SELECT MAX(version) FROM clarity_schema", &[]).map_err(|e| e.to_string())?.first().and_then(|r| r.first()) {
        Some(SqlValue::Integer(n)) => Ok(*n as usize),
        _ => Ok(0),
    }
}

/// Runs the statements of step `version` and counts it in `clarity_schema`,
/// all or nothing where the database undoes changes to tables (SQLite,
/// PostgreSQL, SQL Server). On SQLite references are not enforced while a
/// step runs, since a step may build a table again that others refer to;
/// they are checked before the step is kept. MySQL cannot undo changes to
/// tables, so a step that fails halfway keeps what ran, and says so.
fn apply(db: &Db, source: &str, dialect: Dialect, version: usize, statements: &[String]) -> Result<(), String> {
    let run = |sql: &str| db.batch(source, sql).map_err(|e| e.to_string());
    let counted = format!("INSERT INTO clarity_schema (version) VALUES ({version})");
    let all = |begin: &str, commit: &str, undo: &str, checked: bool| -> Result<(), String> {
        let done = (|| {
            run(begin)?;
            for statement in statements {
                run(statement)?;
            }
            if checked {
                let rows = db.rows(source, "SELECT COUNT(*) FROM pragma_foreign_key_check", &[]).map_err(|e| e.to_string())?;
                if let Some(SqlValue::Integer(broken)) = rows.first().and_then(|r| r.first()).filter(|v| **v != SqlValue::Integer(0)) {
                    return Err(format!("migration step {version} leaves {broken} reference(s) to records that do not exist"));
                }
            }
            run(&counted)?;
            run(commit)
        })();
        if done.is_err() {
            let _ = db.batch(source, undo);
        }
        done
    };
    match dialect {
        Dialect::Sqlite => {
            run("PRAGMA foreign_keys = OFF")?;
            let done = all("BEGIN", "COMMIT", "ROLLBACK", true);
            run("PRAGMA foreign_keys = ON")?;
            done
        }
        Dialect::Postgres => all("BEGIN", "COMMIT", "ROLLBACK", false),
        // Each statement is a batch of its own, so a statement can use a column the one before it added.
        Dialect::Mssql => all("BEGIN TRANSACTION", "COMMIT TRANSACTION", "IF @@TRANCOUNT > 0 ROLLBACK TRANSACTION", false),
        Dialect::Mysql => {
            for (i, statement) in statements.iter().enumerate() {
                if let Err(what) = run(statement) {
                    return Err(if i == 0 { what } else { format!("{what}; MySQL cannot undo the {i} statement(s) of step {version} that ran before it") });
                }
            }
            run(&counted)
        }
    }
}

/// Backs up the SQLite database before step `step` is applied to it, into
/// `folder` (relative: in the folder of its file; conformance/backups.json),
/// and prunes the older automatic backups there; nothing for one in memory.
/// A backup that fails stops the migration.
fn backup_first(db: &Db, source: &str, step: usize, folder: &std::path::Path) -> Result<(), String> {
    let refused = |what: String| format!("the database was not migrated, since it could not be backed up first: {what}");
    let rows = db.rows(source, "SELECT file FROM pragma_database_list WHERE name = 'main'", &[]).map_err(|e| refused(e.to_string()))?;
    let file = match rows.first().and_then(|r| r.first()) {
        Some(SqlValue::Text(file)) if !file.is_empty() => std::path::PathBuf::from(file),
        _ => return Ok(()),
    };
    let dir = file.parent().unwrap_or(std::path::Path::new(".")).join(folder);
    std::fs::create_dir_all(&dir).map_err(|e| refused(format!("{}: {e}", dir.display())))?;
    let name = file.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "database".into());
    let seconds = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    let first = dir.join(backup::file_name(&name, seconds, step));
    let mut path = first.clone();
    let mut n = 2;
    while path.exists() {
        path = std::path::PathBuf::from(format!("{}_{n}.db", first.with_extension("").display()));
        n += 1;
    }
    let quoted = path.to_string_lossy().replace('\'', "''");
    db.batch(source, &format!("VACUUM INTO '{quoted}'")).map_err(|e| refused(e.to_string()))?;
    backup::prune(&dir, &name);
    Ok(())
}

fn connect(database: &Database) -> Result<Db, String> {
    let dialect = dialect_of(&database.dialect)?;
    Db::connect(&[(database.source.as_str(), dialect, database.location.as_str())]).map_err(|e| format!("{}: {e}", database.source))
}
