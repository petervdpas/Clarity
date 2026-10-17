//! Conversions between field values and SQLite values, used by the
//! `Record` implementations that Clarity generates. Decimals and dates are
//! stored as text, so no precision is lost; enums are stored by name.

use std::str::FromStr;

use crate::{Decimal, Error, Id, NaiveDate, NaiveDateTime, Result, SqlValue};

const DATE: &str = "%Y-%m-%d";
const DATE_TIME: &str = "%Y-%m-%dT%H:%M:%S";

pub fn text(value: &str) -> SqlValue {
    SqlValue::Text(value.to_string())
}

pub fn integer(value: i64) -> SqlValue {
    SqlValue::Integer(value)
}

pub fn decimal(value: Decimal) -> SqlValue {
    SqlValue::Text(value.to_string())
}

pub fn boolean(value: bool) -> SqlValue {
    SqlValue::Integer(i64::from(value))
}

pub fn date(value: Option<NaiveDate>) -> SqlValue {
    value.map_or(SqlValue::Null, |d| SqlValue::Text(d.format(DATE).to_string()))
}

pub fn date_time(value: Option<NaiveDateTime>) -> SqlValue {
    value.map_or(SqlValue::Null, |d| SqlValue::Text(d.format(DATE_TIME).to_string()))
}

pub fn reference(value: Option<Id>) -> SqlValue {
    value.map_or(SqlValue::Null, SqlValue::Integer)
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
        other => Err(invalid(column, other)),
    }
}

pub fn get_integer(column: &str, value: &SqlValue) -> Result<i64> {
    match value {
        SqlValue::Integer(n) => Ok(*n),
        SqlValue::Null => Ok(0),
        other => Err(invalid(column, other)),
    }
}

pub fn get_decimal(column: &str, value: &SqlValue) -> Result<Decimal> {
    match value {
        SqlValue::Text(s) => Decimal::from_str(s).map_err(|_| invalid(column, value)),
        SqlValue::Integer(n) => Ok(Decimal::from(*n)),
        SqlValue::Real(r) => Decimal::try_from(*r).map_err(|_| invalid(column, value)),
        SqlValue::Null => Ok(Decimal::ZERO),
        other => Err(invalid(column, other)),
    }
}

pub fn get_boolean(column: &str, value: &SqlValue) -> Result<bool> {
    Ok(get_integer(column, value)? != 0)
}

pub fn get_date(column: &str, value: &SqlValue) -> Result<Option<NaiveDate>> {
    match value {
        SqlValue::Null => Ok(None),
        // A date can be written with a time of day after it.
        SqlValue::Text(s) => NaiveDate::parse_from_str(s.get(..10).unwrap_or(s), DATE).map(Some).map_err(|_| invalid(column, value)),
        other => Err(invalid(column, other)),
    }
}

pub fn get_date_time(column: &str, value: &SqlValue) -> Result<Option<NaiveDateTime>> {
    match value {
        SqlValue::Null => Ok(None),
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
