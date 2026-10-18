//! MySQL: decimals are DECIMAL, dates DATE and DATETIME, bools BOOLEAN.
//! MySQL cannot undo changes to tables, so a migration step that fails
//! halfway keeps what ran before the failure.

use mysql::consts::ColumnType;
use mysql::prelude::Queryable;
use mysql::{Conn, Opts, Params, Value};

use chrono::{Datelike, Timelike};

use crate::dialect::Dialect;
use crate::store::{Driver, Failure, Outcome, applied};
use crate::{Error, Id, NaiveDate, Result, SqlValue};

fn param(value: &SqlValue) -> Value {
    match value {
        SqlValue::Null => Value::NULL,
        SqlValue::Integer(n) => Value::Int(*n),
        SqlValue::Real(r) => Value::Double(*r),
        SqlValue::Text(s) => Value::Bytes(s.as_bytes().to_vec()),
        SqlValue::Blob(b) => Value::Bytes(b.clone()),
        SqlValue::Decimal(d) => Value::Bytes(d.to_string().into_bytes()),
        SqlValue::Bool(b) => Value::Int(i64::from(*b)),
        SqlValue::Date(d) => Value::Date(d.year() as u16, d.month() as u8, d.day() as u8, 0, 0, 0, 0),
        SqlValue::DateTime(d) => Value::Date(
            d.year() as u16,
            d.month() as u8,
            d.day() as u8,
            d.hour() as u8,
            d.minute() as u8,
            d.second() as u8,
            d.and_utc().timestamp_subsec_micros(),
        ),
    }
}

fn params(values: &[SqlValue]) -> Params {
    if values.is_empty() {
        Params::Empty
    } else {
        Params::Positional(values.iter().map(param).collect())
    }
}

/// A value read from a column of type `ty`.
fn value(value: Value, ty: ColumnType) -> SqlValue {
    match value {
        Value::NULL => SqlValue::Null,
        Value::Int(n) => SqlValue::Integer(n),
        Value::UInt(n) => SqlValue::Integer(n as i64),
        Value::Float(r) => SqlValue::Real(r.into()),
        Value::Double(r) => SqlValue::Real(r),
        Value::Bytes(b) if matches!(ty, ColumnType::MYSQL_TYPE_BLOB | ColumnType::MYSQL_TYPE_LONG_BLOB) && std::str::from_utf8(&b).is_err() => {
            SqlValue::Blob(b)
        }
        Value::Bytes(b) => SqlValue::Text(String::from_utf8_lossy(&b).into_owned()),
        Value::Date(y, m, d, h, i, s, us) => {
            let Some(day) = NaiveDate::from_ymd_opt(y.into(), m.into(), d.into()) else {
                return SqlValue::Null;
            };
            if ty == ColumnType::MYSQL_TYPE_DATE {
                SqlValue::Date(day)
            } else {
                day.and_hms_micro_opt(h.into(), i.into(), s.into(), us).map_or(SqlValue::Null, SqlValue::DateTime)
            }
        }
        Value::Time(negative, days, h, i, s, _) => {
            let sign = if negative { "-" } else { "" };
            SqlValue::Text(format!("{sign}{:02}:{i:02}:{s:02}", u32::from(h) + days * 24))
        }
    }
}

impl From<mysql::Error> for Failure {
    fn from(err: mysql::Error) -> Self {
        match &err {
            // A row is referred to (1451, 1217), or refers to nothing (1452, 1216).
            mysql::Error::MySqlError(e) if matches!(e.code, 1216 | 1217 | 1451 | 1452) => Failure::Reference(e.message.clone()),
            mysql::Error::MySqlError(e) => Failure::Other(e.message.clone()),
            _ => Failure::Other(err.to_string()),
        }
    }
}

pub(crate) struct Mysql(Conn);

impl Mysql {
    pub(crate) fn open(url: &str) -> Result<Mysql> {
        let opts = Opts::from_url(url).map_err(|err| Error::Storage(format!("the MySQL connection is not valid: {err}")))?;
        Conn::new(opts)
            .map(Mysql)
            .map_err(|err| Error::Storage(format!("cannot connect to MySQL: {err}")))
    }
}

impl Driver for Mysql {
    fn dialect(&self) -> Dialect {
        Dialect::Mysql
    }

    fn execute(&mut self, sql: &str, values: &[SqlValue]) -> Outcome<u64> {
        self.0.exec_drop(sql, params(values))?;
        Ok(self.0.affected_rows())
    }

    fn insert(&mut self, sql: &str, values: &[SqlValue]) -> Outcome<Id> {
        self.0.exec_drop(sql, params(values))?;
        Ok(self.0.last_insert_id() as Id)
    }

    fn rows(&mut self, sql: &str, values: &[SqlValue]) -> Outcome<Vec<Vec<SqlValue>>> {
        let result = self.0.exec_iter(sql, params(values))?;
        let mut out = Vec::new();
        for row in result {
            let row = row?;
            let types: Vec<ColumnType> = row.columns_ref().iter().map(|c| c.column_type()).collect();
            out.push(row.unwrap().into_iter().zip(types).map(|(v, ty)| value(v, ty)).collect());
        }
        Ok(out)
    }

    fn batch(&mut self, sql: &str) -> Outcome<()> {
        Ok(self.0.query_drop(sql)?)
    }

    fn apply(&mut self, version: usize, statements: &[&str]) -> Outcome<()> {
        for (i, statement) in statements.iter().enumerate() {
            if let Err(err) = self.0.query_drop(statement) {
                let failure = Failure::from(err);
                let (Failure::Reference(what) | Failure::Other(what)) = &failure;
                return Err(Failure::Other(if i == 0 {
                    what.clone()
                } else {
                    format!("{what}; MySQL cannot undo the {i} statement(s) of step {version} that ran before it")
                }));
            }
        }
        Ok(self.0.query_drop(applied(version))?)
    }
}
