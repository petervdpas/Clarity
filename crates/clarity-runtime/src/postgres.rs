//! PostgreSQL: decimals are NUMERIC, dates DATE and TIMESTAMP, bools
//! BOOLEAN. Parameters are written as the column they go into needs them,
//! so a value fits whatever type the statement expects there.

use bytes::BytesMut;
use postgres::error::SqlState;
use postgres::types::{FromSql, IsNull, Kind, ToSql, Type, to_sql_checked};
use postgres::{Client, NoTls};

use crate::dialect::Dialect;
use crate::store::{Driver, Failure, Outcome, applied};
use crate::{Decimal, Error, Id, NaiveDate, NaiveDateTime, Result, SqlValue};

type Boxed = Box<dyn std::error::Error + Sync + Send>;

/// A value as a parameter of a statement.
#[derive(Debug)]
struct Param<'a>(&'a SqlValue);

fn textual(ty: &Type) -> bool {
    matches!(*ty, Type::TEXT | Type::VARCHAR | Type::BPCHAR | Type::NAME | Type::UNKNOWN) || matches!(ty.kind(), Kind::Enum(_))
}

fn mismatch(value: &SqlValue, ty: &Type) -> Boxed {
    format!("{value:?} does not fit a column of type {ty}").into()
}

/// A whole number as a column of type `ty` takes it.
fn whole(n: i64, ty: &Type, out: &mut BytesMut) -> std::result::Result<IsNull, Boxed> {
    match *ty {
        Type::INT2 => i16::try_from(n)?.to_sql(ty, out),
        Type::INT4 => i32::try_from(n)?.to_sql(ty, out),
        Type::INT8 => n.to_sql(ty, out),
        Type::NUMERIC => Decimal::from(n).to_sql(ty, out),
        Type::FLOAT4 | Type::FLOAT8 => (n as f64).to_sql(ty, out),
        Type::BOOL => (n != 0).to_sql(ty, out),
        _ if textual(ty) => n.to_string().to_sql(&Type::TEXT, out),
        _ => Err(mismatch(&SqlValue::Integer(n), ty)),
    }
}

impl ToSql for Param<'_> {
    fn to_sql(&self, ty: &Type, out: &mut BytesMut) -> std::result::Result<IsNull, Boxed> {
        match self.0 {
            SqlValue::Null => Ok(IsNull::Yes),
            SqlValue::Integer(n) => whole(*n, ty, out),
            SqlValue::Bool(b) if *ty == Type::BOOL => b.to_sql(ty, out),
            SqlValue::Bool(b) => whole(i64::from(*b), ty, out),
            SqlValue::Real(r) => match *ty {
                Type::FLOAT4 => (*r as f32).to_sql(ty, out),
                Type::NUMERIC => Decimal::try_from(*r)?.to_sql(ty, out),
                _ if textual(ty) => r.to_string().to_sql(&Type::TEXT, out),
                _ => r.to_sql(ty, out),
            },
            SqlValue::Decimal(d) => match *ty {
                Type::NUMERIC => d.to_sql(ty, out),
                Type::FLOAT4 | Type::FLOAT8 => f64::try_from(*d)?.to_sql(ty, out),
                Type::INT2 | Type::INT4 | Type::INT8 => whole(i64::try_from(*d)?, ty, out),
                _ if textual(ty) => d.to_string().to_sql(&Type::TEXT, out),
                _ => Err(mismatch(self.0, ty)),
            },
            SqlValue::Date(d) => match *ty {
                Type::DATE => d.to_sql(ty, out),
                Type::TIMESTAMP => d.and_time(chrono::NaiveTime::MIN).to_sql(ty, out),
                _ if textual(ty) => d.to_string().to_sql(&Type::TEXT, out),
                _ => Err(mismatch(self.0, ty)),
            },
            SqlValue::DateTime(d) => match *ty {
                Type::TIMESTAMP => d.to_sql(ty, out),
                Type::TIMESTAMPTZ => d.and_utc().to_sql(ty, out),
                Type::DATE => d.date().to_sql(ty, out),
                _ if textual(ty) => d.to_string().to_sql(&Type::TEXT, out),
                _ => Err(mismatch(self.0, ty)),
            },
            SqlValue::Text(s) => match *ty {
                Type::NUMERIC => s.parse::<Decimal>()?.to_sql(ty, out),
                Type::INT2 | Type::INT4 | Type::INT8 => whole(s.parse()?, ty, out),
                _ if textual(ty) => s.to_sql(&Type::TEXT, out),
                _ => Err(mismatch(self.0, ty)),
            },
            SqlValue::Blob(b) if *ty == Type::BYTEA => b.to_sql(ty, out),
            SqlValue::Blob(_) => Err(mismatch(self.0, ty)),
        }
    }

    fn accepts(_: &Type) -> bool {
        true
    }

    to_sql_checked!();
}

impl<'a> FromSql<'a> for SqlValue {
    fn from_sql(ty: &Type, raw: &'a [u8]) -> std::result::Result<Self, Boxed> {
        Ok(match *ty {
            Type::BOOL => SqlValue::Bool(bool::from_sql(ty, raw)?),
            Type::INT2 => SqlValue::Integer(i16::from_sql(ty, raw)?.into()),
            Type::INT4 => SqlValue::Integer(i32::from_sql(ty, raw)?.into()),
            Type::INT8 => SqlValue::Integer(i64::from_sql(ty, raw)?),
            Type::FLOAT4 => SqlValue::Real(f32::from_sql(ty, raw)?.into()),
            Type::FLOAT8 => SqlValue::Real(f64::from_sql(ty, raw)?),
            Type::NUMERIC => SqlValue::Decimal(Decimal::from_sql(ty, raw)?),
            Type::DATE => SqlValue::Date(NaiveDate::from_sql(ty, raw)?),
            Type::TIMESTAMP => SqlValue::DateTime(NaiveDateTime::from_sql(ty, raw)?),
            Type::TIMESTAMPTZ => SqlValue::DateTime(chrono::DateTime::<chrono::Utc>::from_sql(ty, raw)?.naive_utc()),
            Type::BYTEA => SqlValue::Blob(raw.to_vec()),
            _ if textual(ty) => SqlValue::Text(String::from_utf8_lossy(raw).into_owned()),
            _ => return Err(format!("columns of type {ty} cannot be read yet").into()),
        })
    }

    fn from_sql_null(_: &Type) -> std::result::Result<Self, Boxed> {
        Ok(SqlValue::Null)
    }

    fn accepts(_: &Type) -> bool {
        true
    }
}

impl From<postgres::Error> for Failure {
    fn from(err: postgres::Error) -> Self {
        let message = err.as_db_error().map_or_else(|| err.to_string(), |e| e.message().to_string());
        if err.code() == Some(&SqlState::FOREIGN_KEY_VIOLATION) {
            Failure::Reference(message)
        } else {
            Failure::Other(message)
        }
    }
}

pub(crate) struct Postgres(Client);

fn params(values: &[SqlValue]) -> Vec<Param<'_>> {
    values.iter().map(Param).collect()
}

fn refs<'a>(params: &'a [Param<'a>]) -> Vec<&'a (dyn ToSql + Sync)> {
    params.iter().map(|p| p as &(dyn ToSql + Sync)).collect()
}

impl Postgres {
    pub(crate) fn open(url: &str) -> Result<Postgres> {
        Client::connect(url, NoTls)
            .map(Postgres)
            .map_err(|err| Error::Storage(format!("cannot connect to PostgreSQL: {err}")))
    }
}

impl Driver for Postgres {
    fn dialect(&self) -> Dialect {
        Dialect::Postgres
    }

    fn execute(&mut self, sql: &str, values: &[SqlValue]) -> Outcome<u64> {
        let params = params(values);
        Ok(self.0.execute(sql, &refs(&params))?)
    }

    fn insert(&mut self, sql: &str, values: &[SqlValue]) -> Outcome<Id> {
        let params = params(values);
        let row = self.0.query_one(sql, &refs(&params))?;
        match row.try_get::<_, SqlValue>(0)? {
            SqlValue::Integer(id) => Ok(id),
            other => Err(Failure::Other(format!("the new key is {other:?}, not a whole number"))),
        }
    }

    fn rows(&mut self, sql: &str, values: &[SqlValue]) -> Outcome<Vec<Vec<SqlValue>>> {
        let params = params(values);
        let rows = self.0.query(sql, &refs(&params))?;
        let mut out = Vec::new();
        for row in rows {
            out.push((0..row.len()).map(|i| row.try_get::<_, SqlValue>(i)).collect::<std::result::Result<Vec<_>, _>>()?);
        }
        Ok(out)
    }

    fn batch(&mut self, sql: &str) -> Outcome<()> {
        Ok(self.0.batch_execute(sql)?)
    }

    /// PostgreSQL undoes changes to tables too, so a step that fails
    /// leaves the database as it was.
    fn apply(&mut self, version: usize, statements: &[&str]) -> Outcome<()> {
        let mut tx = self.0.transaction()?;
        for statement in statements {
            tx.batch_execute(statement)?;
        }
        tx.batch_execute(&applied(version))?;
        Ok(tx.commit()?)
    }
}
