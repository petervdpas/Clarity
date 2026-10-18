//! SQLite: one file per data source. Decimals and dates are stored as
//! text, bools as 0 and 1.

use std::path::Path;

use rusqlite::types::{ToSqlOutput, Value, ValueRef};

use crate::dialect::Dialect;
use crate::sql::{DATE, DATE_TIME};
use crate::store::{Driver, Failure, Outcome, applied};
use crate::{Error, Id, Result, SqlValue};

impl rusqlite::ToSql for SqlValue {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(match self {
            SqlValue::Null | SqlValue::Kept => ToSqlOutput::Owned(Value::Null),
            SqlValue::Integer(n) => ToSqlOutput::Owned(Value::Integer(*n)),
            SqlValue::Real(r) => ToSqlOutput::Owned(Value::Real(*r)),
            SqlValue::Text(s) => ToSqlOutput::Borrowed(ValueRef::Text(s.as_bytes())),
            SqlValue::Blob(b) => ToSqlOutput::Borrowed(ValueRef::Blob(b)),
            SqlValue::Decimal(d) => ToSqlOutput::Owned(Value::Text(d.to_string())),
            SqlValue::Bool(b) => ToSqlOutput::Owned(Value::Integer(i64::from(*b))),
            SqlValue::Date(d) => ToSqlOutput::Owned(Value::Text(d.format(DATE).to_string())),
            SqlValue::DateTime(d) => ToSqlOutput::Owned(Value::Text(d.format(DATE_TIME).to_string())),
        })
    }
}

fn value(value: ValueRef<'_>) -> SqlValue {
    match value {
        ValueRef::Null => SqlValue::Null,
        ValueRef::Integer(n) => SqlValue::Integer(n),
        ValueRef::Real(r) => SqlValue::Real(r),
        ValueRef::Text(t) => SqlValue::Text(String::from_utf8_lossy(t).into_owned()),
        ValueRef::Blob(b) => SqlValue::Blob(b.to_vec()),
    }
}

impl From<rusqlite::Error> for Failure {
    fn from(err: rusqlite::Error) -> Self {
        match &err {
            rusqlite::Error::SqliteFailure(e, _) if e.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_FOREIGNKEY => {
                Failure::Reference(err.to_string())
            }
            _ => Failure::Other(err.to_string()),
        }
    }
}

impl From<rusqlite::Error> for Error {
    fn from(err: rusqlite::Error) -> Self {
        Error::Storage(err.to_string())
    }
}

pub(crate) struct Sqlite(pub(crate) rusqlite::Connection);

impl Sqlite {
    pub(crate) fn open(path: &Path) -> Result<Sqlite> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|err| Error::Storage(err.to_string()))?;
        }
        Sqlite::new(rusqlite::Connection::open(path)?)
    }

    pub(crate) fn new(conn: rusqlite::Connection) -> Result<Sqlite> {
        conn.execute_batch("PRAGMA foreign_keys = ON")?;
        Ok(Sqlite(conn))
    }

    fn step(&mut self, version: usize, statements: &[&str]) -> Outcome<()> {
        let tx = self.0.transaction()?;
        for statement in statements {
            tx.execute_batch(statement)?;
        }
        let broken: i64 = tx.query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| row.get(0))?;
        if broken > 0 {
            return Err(Failure::Other(format!(
                "migration step {version} leaves {broken} reference(s) to records that do not exist"
            )));
        }
        tx.execute_batch(&applied(version))?;
        tx.commit()?;
        Ok(())
    }
}

impl Driver for Sqlite {
    fn dialect(&self) -> Dialect {
        Dialect::Sqlite
    }

    fn execute(&mut self, sql: &str, params: &[SqlValue]) -> Outcome<u64> {
        Ok(self.0.execute(sql, rusqlite::params_from_iter(params))? as u64)
    }

    fn insert(&mut self, sql: &str, params: &[SqlValue]) -> Outcome<Id> {
        self.0.execute(sql, rusqlite::params_from_iter(params))?;
        Ok(self.0.last_insert_rowid())
    }

    fn rows(&mut self, sql: &str, params: &[SqlValue]) -> Outcome<Vec<Vec<SqlValue>>> {
        let mut statement = self.0.prepare(sql)?;
        let width = statement.column_count();
        let mut rows = statement.query(rusqlite::params_from_iter(params))?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push((0..width).map(|i| row.get_ref(i).map(value)).collect::<rusqlite::Result<Vec<_>>>()?);
        }
        Ok(out)
    }

    fn batch(&mut self, sql: &str) -> Outcome<()> {
        Ok(self.0.execute_batch(sql)?)
    }

    /// References are not enforced while a step runs, since a step may
    /// build a table again that others refer to; they are checked before
    /// the step is kept.
    fn apply(&mut self, version: usize, statements: &[&str]) -> Outcome<()> {
        self.0.execute_batch("PRAGMA foreign_keys = OFF")?;
        let done = self.step(version, statements);
        self.0.execute_batch("PRAGMA foreign_keys = ON")?;
        done
    }
}
