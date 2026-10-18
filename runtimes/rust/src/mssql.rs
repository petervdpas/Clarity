//! SQL Server: decimals are DECIMAL, dates DATE and DATETIME2, bools BIT.
//! The driver is asynchronous; each call runs it to completion, so the
//! rest of the runtime stays the same as for the other databases.

use std::borrow::Cow;

use tiberius::{Client, ColumnData, Config, FromSql, ToSql};
use tokio::net::TcpStream;
use tokio::runtime::Runtime;
use tokio_util::compat::{Compat, TokioAsyncWriteCompatExt};

use crate::dialect::Dialect;
use crate::store::{Driver, Failure, Outcome, applied};
use crate::{Decimal, Error, Id, NaiveDate, NaiveDateTime, Result, SqlValue};

/// A value as a parameter of a statement.
struct Param<'a>(&'a SqlValue);

impl ToSql for Param<'_> {
    fn to_sql(&self) -> ColumnData<'_> {
        match self.0 {
            // Text converts to every type, so a missing value fits any column.
            SqlValue::Null | SqlValue::Kept => ColumnData::String(None),
            SqlValue::Integer(n) => ColumnData::I64(Some(*n)),
            SqlValue::Real(r) => ColumnData::F64(Some(*r)),
            SqlValue::Text(s) => ColumnData::String(Some(Cow::Borrowed(s.as_str()))),
            SqlValue::Blob(b) => ColumnData::Binary(Some(Cow::Borrowed(b.as_slice()))),
            SqlValue::Decimal(d) => d.to_sql(),
            SqlValue::Bool(b) => ColumnData::Bit(Some(*b)),
            SqlValue::Date(d) => d.to_sql(),
            SqlValue::DateTime(d) => d.to_sql(),
        }
    }
}

fn read<'a, T: FromSql<'a>>(data: &'a ColumnData<'static>, wrap: fn(T) -> SqlValue) -> Outcome<SqlValue> {
    let value = T::from_sql(data).map_err(|err| Failure::Other(err.to_string()))?;
    Ok(value.map_or(SqlValue::Null, wrap))
}

fn value(data: &ColumnData<'static>) -> Outcome<SqlValue> {
    Ok(match data {
        ColumnData::U8(n) => n.map_or(SqlValue::Null, |n| SqlValue::Integer(n.into())),
        ColumnData::I16(n) => n.map_or(SqlValue::Null, |n| SqlValue::Integer(n.into())),
        ColumnData::I32(n) => n.map_or(SqlValue::Null, |n| SqlValue::Integer(n.into())),
        ColumnData::I64(n) => n.map_or(SqlValue::Null, SqlValue::Integer),
        ColumnData::F32(r) => r.map_or(SqlValue::Null, |r| SqlValue::Real(r.into())),
        ColumnData::F64(r) => r.map_or(SqlValue::Null, SqlValue::Real),
        ColumnData::Bit(b) => b.map_or(SqlValue::Null, SqlValue::Bool),
        ColumnData::String(s) => s.as_ref().map_or(SqlValue::Null, |s| SqlValue::Text(s.to_string())),
        ColumnData::Guid(g) => g.map_or(SqlValue::Null, |g| SqlValue::Text(g.to_string())),
        ColumnData::Binary(b) => b.as_ref().map_or(SqlValue::Null, |b| SqlValue::Blob(b.to_vec())),
        ColumnData::Numeric(_) => return read::<Decimal>(data, SqlValue::Decimal),
        ColumnData::Date(_) => return read::<NaiveDate>(data, SqlValue::Date),
        ColumnData::DateTime(_) | ColumnData::SmallDateTime(_) | ColumnData::DateTime2(_) => {
            return read::<NaiveDateTime>(data, SqlValue::DateTime);
        }
        ColumnData::DateTimeOffset(_) => {
            return read::<chrono::DateTime<chrono::Utc>>(data, |d| SqlValue::DateTime(d.naive_utc()));
        }
        other => return Err(Failure::Other(format!("columns like {other:?} cannot be read yet"))),
    })
}

impl From<tiberius::error::Error> for Failure {
    fn from(err: tiberius::error::Error) -> Self {
        match &err {
            // 547: a statement conflicts with a reference between tables.
            tiberius::error::Error::Server(e) if e.code() == 547 => Failure::Reference(e.message().to_string()),
            tiberius::error::Error::Server(e) => Failure::Other(e.message().to_string()),
            _ => Failure::Other(err.to_string()),
        }
    }
}

impl From<std::io::Error> for Failure {
    fn from(err: std::io::Error) -> Self {
        Failure::Other(err.to_string())
    }
}

pub(crate) struct Mssql {
    runtime: Runtime,
    client: Client<Compat<TcpStream>>,
}

impl Mssql {
    /// Connects with an ADO.NET connection string, such as
    /// `server=tcp:localhost,1433;database=shop;user=app;password=...`.
    pub(crate) fn open(connection: &str) -> Result<Mssql> {
        let fail = |err: String| Error::Storage(format!("cannot connect to SQL Server: {err}"));
        let config = Config::from_ado_string(connection).map_err(|err| fail(err.to_string()))?;
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .build()
            .map_err(|err| fail(err.to_string()))?;
        let client = runtime
            .block_on(async {
                let tcp = TcpStream::connect(config.get_addr()).await?;
                tcp.set_nodelay(true)?;
                Client::connect(config, tcp.compat_write()).await
            })
            .map_err(|err| fail(err.to_string()))?;
        Ok(Mssql { runtime, client })
    }

    fn query(&mut self, sql: &str, values: &[SqlValue]) -> Outcome<Vec<Vec<SqlValue>>> {
        let params: Vec<Param> = values.iter().map(Param).collect();
        let refs: Vec<&dyn ToSql> = params.iter().map(|p| p as &dyn ToSql).collect();
        let client = &mut self.client;
        let rows = self.runtime.block_on(async { client.query(sql, &refs).await?.into_first_result().await })?;
        rows.into_iter().map(|row| row.cells().map(|(_, data)| value(data)).collect()).collect()
    }

    fn simple(&mut self, sql: &str) -> Outcome<()> {
        let client = &mut self.client;
        self.runtime.block_on(async { client.simple_query(sql).await?.into_results().await })?;
        Ok(())
    }

    fn step(&mut self, version: usize, statements: &[&str]) -> Outcome<()> {
        for statement in statements {
            self.simple(statement)?;
        }
        self.simple(&applied(version))?;
        self.simple("COMMIT TRANSACTION")
    }
}

impl Driver for Mssql {
    fn dialect(&self) -> Dialect {
        Dialect::Mssql
    }

    fn execute(&mut self, sql: &str, values: &[SqlValue]) -> Outcome<u64> {
        let params: Vec<Param> = values.iter().map(Param).collect();
        let refs: Vec<&dyn ToSql> = params.iter().map(|p| p as &dyn ToSql).collect();
        let client = &mut self.client;
        let done = self.runtime.block_on(async { client.execute(sql, &refs).await })?;
        Ok(done.total())
    }

    fn insert(&mut self, sql: &str, values: &[SqlValue]) -> Outcome<Id> {
        match self.query(sql, values)?.first().and_then(|row| row.first()) {
            Some(SqlValue::Integer(id)) => Ok(*id),
            other => Err(Failure::Other(format!("the new key is {other:?}, not a whole number"))),
        }
    }

    fn rows(&mut self, sql: &str, values: &[SqlValue]) -> Outcome<Vec<Vec<SqlValue>>> {
        self.query(sql, values)
    }

    fn batch(&mut self, sql: &str) -> Outcome<()> {
        self.simple(sql)
    }

    /// Each statement is a batch of its own, so a statement can use a
    /// column the one before it added. SQL Server undoes changes to tables
    /// too, so a step that fails leaves the database as it was.
    fn apply(&mut self, version: usize, statements: &[&str]) -> Outcome<()> {
        self.simple("BEGIN TRANSACTION")?;
        let done = self.step(version, statements);
        if done.is_err() {
            let _ = self.simple("IF @@TRANCOUNT > 0 ROLLBACK TRANSACTION");
        }
        done
    }
}
