//! The connection to one database, behind the same few calls for every
//! kind: run a statement, insert and get the new key, read rows, and apply
//! a migration step. Each kind is a cargo feature, so an application only
//! carries the drivers its data sources need.

use crate::dialect::Dialect;
use crate::{Error, Id, Result, SqlValue};

/// Why a statement failed.
pub(crate) enum Failure {
    /// A reference between records would no longer fit: a record that is
    /// referred to, or a reference to a record that does not exist.
    Reference(String),
    Other(String),
}

impl From<Failure> for Error {
    fn from(failure: Failure) -> Self {
        match failure {
            Failure::Reference(what) | Failure::Other(what) => Error::Storage(what),
        }
    }
}

pub(crate) type Outcome<T> = std::result::Result<T, Failure>;

pub(crate) trait Driver: Send {
    fn dialect(&self) -> Dialect;
    /// Runs a statement and gives the number of rows it changed.
    fn execute(&mut self, sql: &str, params: &[SqlValue]) -> Outcome<u64>;
    /// Runs an INSERT from `Dialect::insert` and gives the new record's key.
    fn insert(&mut self, sql: &str, params: &[SqlValue]) -> Outcome<Id>;
    fn rows(&mut self, sql: &str, params: &[SqlValue]) -> Outcome<Vec<Vec<SqlValue>>>;
    /// Runs SQL without parameters, such as a statement that creates a table.
    fn batch(&mut self, sql: &str) -> Outcome<()>;
    /// Runs the statements of migration step `version` and records that
    /// the database has it, all or nothing where the database can undo
    /// changes to tables.
    fn apply(&mut self, version: usize, statements: &[&str]) -> Outcome<()>;
}

/// The statement that records migration step `version`.
pub(crate) fn applied(version: usize) -> String {
    format!("INSERT INTO clarity_schema (version) VALUES ({version})")
}

/// The location of a database: as written, or from the environment
/// variable that `env:NAME` names, so passwords stay out of the program.
fn location(text: &str) -> Result<String> {
    match text.strip_prefix("env:") {
        Some(name) => std::env::var(name).map_err(|_| Error::Storage(format!("the environment variable {name} is not set"))),
        None => Ok(text.to_string()),
    }
}

/// Connects to the database of a data source.
pub(crate) fn open(dialect: Dialect, at: &str) -> Result<Box<dyn Driver>> {
    let at = location(at)?;
    match dialect {
        #[cfg(feature = "sqlite")]
        Dialect::Sqlite => Ok(Box::new(crate::sqlite::Sqlite::open(std::path::Path::new(&at))?)),
        #[cfg(feature = "postgres")]
        Dialect::Postgres => Ok(Box::new(crate::postgres::Postgres::open(&at)?)),
        #[cfg(feature = "mysql")]
        Dialect::Mysql => Ok(Box::new(crate::mysql::Mysql::open(&at)?)),
        #[cfg(feature = "mssql")]
        Dialect::Mssql => Ok(Box::new(crate::mssql::Mssql::open(&at)?)),
        #[allow(unreachable_patterns)]
        other => Err(Error::Storage(format!("this application was built without {}", other.label()))),
    }
}
