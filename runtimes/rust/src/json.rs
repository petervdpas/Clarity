//! JSON in code (DESIGN 5.23): `json of value` and `read Order from json
//! text`. A service's JSON is not Clarity's own, so what is read is fitted to
//! the entity first: names match ignoring case, underscores and hyphens,
//! numbers and strings of numbers stand for each other, what is left out
//! keeps its default. Every runtime fits alike (conformance/json.json); the
//! record's own decoder then reads what was fitted.

use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Map, Value};

use crate::{Error, Result};

/// What a field of an entity takes from JSON.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind<'a> {
    Text,
    Integer,
    /// Decimals and money, written as exact text.
    Decimal,
    Bool,
    Date,
    DateTime,
    /// One of these values, matched in any case.
    Enum(&'a [&'a str]),
    /// A list whose items fit the kind.
    Values(&'a Kind<'a>),
    /// A record inside the record (DESIGN 5.25): an object fitted to its own entity.
    Record(&'a Nested<'a>),
    /// Records inside the record: a list of objects, each fitted to its entity.
    Records(&'a Nested<'a>),
    /// Never read: a reference, a picture, a file, a list of stored records.
    Other,
}

/// A field of an entity, as JSON is fitted to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Field<'a> {
    pub name: &'a str,
    pub kind: Kind<'a>,
}

/// The entity of a record inside a record: its name, its fields, and a new
/// record of it as Clarity writes it, which what JSON leaves out keeps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Nested<'a> {
    pub entity: &'a str,
    pub fields: &'a [Field<'a>],
    pub base: &'a Value,
}

/// The JSON of a value, as Clarity writes values everywhere.
pub fn of<T: Serialize + ?Sized>(value: &T) -> String {
    serde_json::to_string(value).unwrap_or_default()
}

/// Reads `text` as one record of `entity` (or, with `many`, a list of them),
/// starting at `at` (object names with dots between), fitted to `fields`
/// over `base`, a new record as Clarity writes it, and decoded as `T`.
pub fn read<T: DeserializeOwned>(text: &str, at: Option<&str>, many: bool, entity: &str, fields: &[Field], base: &Value) -> Result<T> {
    let fitted = fit(text, at, many, entity, fields, base).map_err(Error::Value)?;
    serde_json::from_value(fitted).map_err(|e| Error::Value(format!("the JSON does not fit {entity}: {e}")))
}

/// What `read` decodes: the JSON fitted to the entity, or why it does not fit.
pub fn fit(text: &str, at: Option<&str>, many: bool, entity: &str, fields: &[Field], base: &Value) -> std::result::Result<Value, String> {
    let mut value: Value = serde_json::from_str(text).map_err(|_| "the text is no JSON".to_string())?;
    if let Some(at) = at.filter(|a| !a.is_empty()) {
        let mut walked = Vec::new();
        for step in at.split('.') {
            walked.push(step);
            value = match value {
                Value::Object(mut object) => object.remove(step),
                _ => None,
            }
            .ok_or_else(|| format!("the JSON has no '{}'", walked.join(".")))?;
        }
    }
    if many {
        let Value::Array(items) = value else { return Err(format!("the JSON is not a list of {entity}")) };
        let fitted = items
            .iter()
            .enumerate()
            .map(|(i, item)| match item {
                Value::Object(object) => record(object, entity, fields, base).map_err(|why| format!("item {}: {why}", i + 1)),
                _ => Err(format!("item {} of the list is not one {entity}", i + 1)),
            })
            .collect::<std::result::Result<Vec<_>, _>>()?;
        return Ok(Value::Array(fitted));
    }
    match &value {
        Value::Object(object) => record(object, entity, fields, base),
        _ => Err(format!("the JSON is not one {entity}")),
    }
}

/// A name as names are matched: lower case, without underscores, hyphens and spaces.
fn plain(name: &str) -> String {
    name.chars().filter(|c| !matches!(c, '_' | '-' | ' ')).flat_map(char::to_lowercase).collect()
}

fn record(object: &Map<String, Value>, entity: &str, fields: &[Field], base: &Value) -> std::result::Result<Value, String> {
    let mut out = base.as_object().cloned().unwrap_or_default();
    for field in fields {
        let given = object.get(field.name).or_else(|| object.iter().find(|(name, _)| plain(name) == plain(field.name)).map(|(_, v)| v));
        let Some(given) = given.filter(|v| !v.is_null()) else { continue };
        let value = match field.kind {
            Kind::Other => continue,
            Kind::Record(nested) => match given {
                Value::Object(inner) => record(inner, nested.entity, nested.fields, nested.base).map_err(|why| format!("field '{}' of {entity}: {why}", field.name))?,
                other => return Err(format!("field '{}' of {entity} is one {}, not {}", field.name, nested.entity, compact(other))),
            },
            Kind::Records(nested) => {
                let Value::Array(items) = given else {
                    return Err(format!("field '{}' of {entity} is a list of {}, not {}", field.name, nested.entity, compact(given)));
                };
                let mut fitted = Vec::new();
                for (i, item) in items.iter().enumerate() {
                    let Value::Object(inner) = item else {
                        return Err(format!("field '{}' of {entity}, item {} is not one {}", field.name, i + 1, nested.entity));
                    };
                    fitted.push(record(inner, nested.entity, nested.fields, nested.base).map_err(|why| format!("field '{}' of {entity}, item {}: {why}", field.name, i + 1))?);
                }
                Value::Array(fitted)
            }
            kind => value(given, kind).ok_or_else(|| format!("field '{}' of {entity} is {}, not {}", field.name, wanted(kind), compact(given)))?,
        };
        out.insert(field.name.to_string(), value);
    }
    Ok(Value::Object(out))
}

/// What a kind is, as a message says it.
fn wanted(kind: Kind) -> String {
    match kind {
        Kind::Text => "text".into(),
        Kind::Integer => "an integer".into(),
        Kind::Decimal => "a decimal".into(),
        Kind::Bool => "true or false".into(),
        Kind::Date => "a date".into(),
        Kind::DateTime => "a date and time".into(),
        Kind::Enum(values) => match values {
            [] => "an enum value".into(),
            [one] => (*one).to_string(),
            [rest @ .., last] => format!("{} or {last}", rest.join(", ")),
        },
        Kind::Values(_) => "a list".into(),
        Kind::Record(nested) => format!("one {}", nested.entity),
        Kind::Records(nested) => format!("a list of {}", nested.entity),
        Kind::Other => "something else".into(),
    }
}

fn compact(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_default()
}

fn is_decimal(text: &str) -> bool {
    let digits = text.strip_prefix('-').unwrap_or(text);
    let (whole, fraction) = digits.split_once('.').unwrap_or((digits, ""));
    !whole.is_empty() && whole.bytes().all(|b| b.is_ascii_digit()) && (fraction.is_empty() && !digits.ends_with('.') || fraction.bytes().all(|b| b.is_ascii_digit()) && !fraction.is_empty())
}

fn is_date(text: &str) -> bool {
    let b = text.as_bytes();
    b.len() == 10 && b[4] == b'-' && b[7] == b'-' && [0, 1, 2, 3, 5, 6, 8, 9].iter().all(|&i| b[i].is_ascii_digit())
}

/// A date and time as Clarity writes it, `YYYY-MM-DDTHH:MM:SS`, from the
/// same with or without seconds, a fraction of a second and a `Z`.
fn date_time(text: &str) -> Option<String> {
    let text = text.strip_suffix('Z').unwrap_or(text);
    let (date, time) = text.split_once(['T', ' '])?;
    if !is_date(date) {
        return None;
    }
    let time = time.split('.').next()?;
    let parts: Vec<&str> = time.split(':').collect();
    let two = |p: &str| p.len() == 2 && p.bytes().all(|b| b.is_ascii_digit());
    match parts.as_slice() {
        [h, m] if two(h) && two(m) => Some(format!("{date}T{h}:{m}:00")),
        [h, m, s] if two(h) && two(m) && two(s) => Some(format!("{date}T{h}:{m}:{s}")),
        _ => None,
    }
}

/// `given` fitted to `kind`, or none when it does not fit.
fn value(given: &Value, kind: Kind) -> Option<Value> {
    match (kind, given) {
        (Kind::Text, Value::String(_)) => Some(given.clone()),
        (Kind::Text, Value::Number(_) | Value::Bool(_)) => Some(Value::String(compact(given))),
        (Kind::Integer, Value::Number(n)) => n.as_i64().or_else(|| n.as_f64().filter(|f| f.fract() == 0.0 && f.abs() < 9.0e15).map(|f| f as i64)).map(Value::from),
        (Kind::Integer, Value::String(s)) => s.trim().parse::<i64>().ok().map(Value::from),
        (Kind::Decimal, Value::Number(n)) => Some(Value::String(n.to_string())),
        (Kind::Decimal, Value::String(s)) if is_decimal(s.trim()) => Some(Value::String(s.trim().to_string())),
        (Kind::Bool, Value::Bool(_)) => Some(given.clone()),
        (Kind::Date, Value::String(s)) if is_date(s) => Some(given.clone()),
        (Kind::Date, Value::String(s)) => date_time(s).map(|d| Value::String(d[..10].to_string())),
        (Kind::DateTime, Value::String(s)) => date_time(s).map(Value::String),
        (Kind::Enum(values), Value::String(s)) => values.iter().find(|v| v.eq_ignore_ascii_case(s)).map(|v| Value::String((*v).to_string())),
        (Kind::Values(item), Value::Array(items)) => items.iter().map(|i| value(i, *item)).collect::<Option<Vec<_>>>().map(Value::Array),
        _ => None,
    }
}
