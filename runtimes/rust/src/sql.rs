//! The values records are stored as, and the conversions between field
//! values and them, used by the `Record` implementations that Clarity
//! generates. Each database driver stores a value its own way: SQLite keeps
//! decimals and dates as text, so no precision is lost, and the server
//! databases use their own number and date types. Enums are stored by name.

use std::str::FromStr;

use crate::{Decimal, Error, Id, NaiveDate, NaiveDateTime, Result};

pub(crate) const DATE: &str = "%Y-%m-%d";
pub(crate) const DATE_TIME: &str = "%Y-%m-%dT%H:%M:%S";

/// A stored value, as records are written and read.
#[derive(Debug, Clone, PartialEq)]
pub enum SqlValue {
    Null,
    Integer(i64),
    Real(f64),
    Text(String),
    Blob(Vec<u8>),
    Decimal(Decimal),
    Bool(bool),
    Date(NaiveDate),
    DateTime(NaiveDateTime),
    /// A picture the record has stored already: its column is left as it
    /// is when the record is saved. It never reaches a driver.
    Kept,
}

pub fn text(value: &str) -> SqlValue {
    SqlValue::Text(value.to_string())
}

pub fn integer(value: i64) -> SqlValue {
    SqlValue::Integer(value)
}

pub fn decimal(value: Decimal) -> SqlValue {
    SqlValue::Decimal(value)
}

pub fn boolean(value: bool) -> SqlValue {
    SqlValue::Bool(value)
}

pub fn date(value: Option<NaiveDate>) -> SqlValue {
    value.map_or(SqlValue::Null, SqlValue::Date)
}

pub fn date_time(value: Option<NaiveDateTime>) -> SqlValue {
    value.map_or(SqlValue::Null, SqlValue::DateTime)
}

pub fn reference(value: Option<Id>) -> SqlValue {
    value.map_or(SqlValue::Null, SqlValue::Integer)
}

/// A picture: none, the one stored already (kept), or a new one.
pub fn picture(value: &Option<crate::picture::Picture>) -> SqlValue {
    match value {
        None => SqlValue::Null,
        Some(crate::picture::Picture::Stored) => SqlValue::Kept,
        Some(crate::picture::Picture::New(bytes)) => SqlValue::Blob(bytes.clone()),
        Some(crate::picture::Picture::InStore(path)) => SqlValue::Text(path.clone()),
    }
}

/// Whether a record has a picture: what the select list reads of one kept
/// in the database (1 or 0), or the path of its file in a file store.
pub fn get_picture(column: &str, value: &SqlValue) -> Result<Option<crate::picture::Picture>> {
    let there = match value {
        SqlValue::Null => false,
        SqlValue::Integer(n) => *n != 0,
        SqlValue::Bool(b) => *b,
        SqlValue::Text(path) => !path.is_empty(),
        SqlValue::Blob(bytes) => !bytes.is_empty(),
        other => return Err(Error::Storage(format!("{column}: {other:?} is no picture"))),
    };
    Ok(there.then_some(crate::picture::Picture::Stored))
}

/// An enum value, stored by its name.
pub fn name<E: ToString>(value: &E) -> SqlValue {
    SqlValue::Text(value.to_string())
}

fn invalid(column: &str, value: &SqlValue) -> Error {
    Error::Invalid(format!("{column} cannot be {value:?}"))
}

pub fn get_text(column: &str, value: &SqlValue) -> Result<String> {
    match value {
        SqlValue::Text(s) => Ok(s.clone()),
        SqlValue::Null => Ok(String::new()),
        // Databases written by other programs may hold numbers in text columns.
        SqlValue::Integer(n) => Ok(n.to_string()),
        SqlValue::Real(r) => Ok(r.to_string()),
        SqlValue::Decimal(d) => Ok(d.to_string()),
        other => Err(invalid(column, other)),
    }
}

pub fn get_integer(column: &str, value: &SqlValue) -> Result<i64> {
    match value {
        SqlValue::Integer(n) => Ok(*n),
        SqlValue::Null => Ok(0),
        SqlValue::Bool(b) => Ok(i64::from(*b)),
        SqlValue::Decimal(d) if d.fract().is_zero() => i64::try_from(*d).map_err(|_| invalid(column, value)),
        other => Err(invalid(column, other)),
    }
}

pub fn get_decimal(column: &str, value: &SqlValue) -> Result<Decimal> {
    match value {
        SqlValue::Decimal(d) => Ok(*d),
        SqlValue::Text(s) => Decimal::from_str(s).map_err(|_| invalid(column, value)),
        SqlValue::Integer(n) => Ok(Decimal::from(*n)),
        SqlValue::Real(r) => Decimal::try_from(*r).map_err(|_| invalid(column, value)),
        SqlValue::Null => Ok(Decimal::ZERO),
        other => Err(invalid(column, other)),
    }
}

pub fn get_boolean(column: &str, value: &SqlValue) -> Result<bool> {
    match value {
        SqlValue::Bool(b) => Ok(*b),
        other => Ok(get_integer(column, other)? != 0),
    }
}

pub fn get_date(column: &str, value: &SqlValue) -> Result<Option<NaiveDate>> {
    match value {
        SqlValue::Null => Ok(None),
        SqlValue::Date(d) => Ok(Some(*d)),
        SqlValue::DateTime(d) => Ok(Some(d.date())),
        // A date can be written with a time of day after it.
        SqlValue::Text(s) => NaiveDate::parse_from_str(s.get(..10).unwrap_or(s), DATE).map(Some).map_err(|_| invalid(column, value)),
        other => Err(invalid(column, other)),
    }
}

pub fn get_date_time(column: &str, value: &SqlValue) -> Result<Option<NaiveDateTime>> {
    match value {
        SqlValue::Null => Ok(None),
        SqlValue::DateTime(d) => Ok(Some(*d)),
        SqlValue::Date(d) => Ok(Some(d.and_time(chrono::NaiveTime::MIN))),
        SqlValue::Text(s) => [DATE_TIME, "%Y-%m-%d %H:%M:%S", "%Y-%m-%dT%H:%M", "%Y-%m-%d %H:%M"]
            .iter()
            .find_map(|format| NaiveDateTime::parse_from_str(s, format).ok())
            .map(Some)
            .ok_or_else(|| invalid(column, value)),
        other => Err(invalid(column, other)),
    }
}

pub fn get_reference(column: &str, value: &SqlValue) -> Result<Option<Id>> {
    match value {
        SqlValue::Null => Ok(None),
        SqlValue::Integer(n) => Ok(Some(*n)),
        other => Err(invalid(column, other)),
    }
}

/// An enum value by its name; an empty or missing value gives the default.
pub fn get_name<E: FromStr + Default>(column: &str, value: &SqlValue) -> Result<E> {
    match value {
        SqlValue::Null => Ok(E::default()),
        SqlValue::Text(s) if s.is_empty() => Ok(E::default()),
        SqlValue::Text(s) => E::from_str(s).map_err(|_| invalid(column, value)),
        other => Err(invalid(column, other)),
    }
}
