//! The values of an engine written in Clarity and what code does with them,
//! written once: compiled code reaches them through the `rt_*` functions,
//! the interpreter calls them directly, so both give the same answers.

use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use serde_json::{Value as Json, json};

#[derive(Debug, Clone)]
pub enum Value {
    None,
    Int(i64),
    /// A decimal or money: exact, as rust_decimal has it.
    Dec(Decimal),
    Bool(bool),
    Text(String),
    List(Vec<Value>),
    /// A record (`record Dot`): its fields in order.
    Record(Vec<(String, Value)>),
}

impl PartialEq for Value {
    fn eq(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::None, Value::None) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Text(a), Value::Text(b)) => a == b,
            (Value::List(a), Value::List(b)) => a == b,
            (Value::Record(a), Value::Record(b)) => a == b,
            (a, b) => match (a.number(), b.number()) {
                (Some(x), Some(y)) => x == y,
                _ => false,
            },
        }
    }
}

impl Value {
    pub fn json(&self) -> Json {
        match self {
            Value::None => Json::Null,
            Value::Int(n) => json!(n),
            Value::Dec(d) => json!(d.to_string()),
            Value::Bool(b) => json!(b),
            Value::Text(t) => json!(t),
            Value::List(items) => Json::Array(items.iter().map(Value::json).collect()),
            Value::Record(fields) => Json::Object(fields.iter().map(|(name, value)| (name.clone(), value.json())).collect()),
        }
    }

    pub fn from_json(value: &Json) -> Value {
        match value {
            Json::Null => Value::None,
            Json::Bool(b) => Value::Bool(*b),
            Json::Number(n) => n.as_i64().map(Value::Int).unwrap_or(Value::None),
            Json::String(t) => Value::Text(t.clone()),
            Json::Array(items) => Value::List(items.iter().map(Value::from_json).collect()),
            Json::Object(fields) => Value::Record(fields.iter().map(|(name, value)| (name.clone(), Value::from_json(value))).collect()),
        }
    }

    /// The value as text, as `+` joins it to text.
    pub fn show(&self) -> String {
        match self {
            Value::None => String::new(),
            Value::Int(n) => n.to_string(),
            Value::Dec(d) => d.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Text(t) => t.clone(),
            Value::List(items) => items.iter().map(Value::show).collect::<Vec<_>>().join(", "),
            Value::Record(_) => self.json().to_string(),
        }
    }

    /// A whole number or a decimal as a decimal; a decimal inside a record
    /// or list that came as JSON is still its exact text.
    pub fn number(&self) -> Option<Decimal> {
        match self {
            Value::Int(n) => Some(Decimal::from(*n)),
            Value::Dec(d) => Some(*d),
            Value::Text(t) => t.parse().ok(),
            _ => None,
        }
    }

    /// A decimal read from JSON: exact text, or a number.
    pub fn decimal(json: &Json) -> Option<Value> {
        match json {
            Json::String(t) => t.parse::<Decimal>().ok().map(Value::Dec),
            Json::Number(n) => n.to_string().parse::<Decimal>().ok().map(Value::Dec),
            _ => None,
        }
    }

    /// As a whole number: 0 when it is none.
    pub fn int(&self) -> i64 {
        match self {
            Value::Int(n) => *n,
            other => other.number().and_then(|d| d.to_i64()).unwrap_or(0),
        }
    }

    pub fn truth(&self) -> bool {
        matches!(self, Value::Bool(true))
    }

    pub fn items(&self) -> Vec<Value> {
        if let Value::List(items) = self { items.clone() } else { Vec::new() }
    }
}

// Arguments of a call, by how the function reads them.

pub fn arg_int(args: &[Value], index: usize) -> Result<Value, String> {
    match args.get(index) {
        Some(Value::Int(n)) => Ok(Value::Int(*n)),
        Some(_) => Err(format!("value {} is no whole number", index + 1)),
        None => Err(format!("it takes more than {index} values")),
    }
}

pub fn arg_bool(args: &[Value], index: usize) -> Result<Value, String> {
    match args.get(index) {
        Some(Value::Bool(b)) => Ok(Value::Bool(*b)),
        Some(_) => Err(format!("value {} is no yes or no", index + 1)),
        None => Err(format!("it takes more than {index} values")),
    }
}

pub fn arg(args: &[Value], index: usize) -> Result<Value, String> {
    args.get(index).cloned().ok_or_else(|| format!("it takes more than {index} values"))
}

pub fn arg_decimal(json: &[Json], index: usize) -> Result<Value, String> {
    json.get(index).and_then(Value::decimal).ok_or_else(|| format!("value {} is no decimal", index + 1))
}

pub fn arg_decimals(json: &[Json], index: usize) -> Result<Value, String> {
    match json.get(index) {
        Some(Json::Array(items)) => items.iter().map(Value::decimal).collect::<Option<Vec<_>>>().map(Value::List),
        _ => None,
    }
    .ok_or_else(|| format!("value {} is no list of decimals", index + 1))
}

// Texts and numbers.

pub fn decimal(text: &str) -> Value {
    text.parse::<Decimal>().map(Value::Dec).unwrap_or(Value::None)
}

pub fn join(a: &Value, b: &Value) -> Value {
    Value::Text(a.show() + &b.show())
}

/// Text by its letters, numbers by their value: below 0, 0 or above.
pub fn compare(a: &Value, b: &Value) -> i32 {
    match (a, b) {
        (Value::Text(x), Value::Text(y)) => x.cmp(y) as i32,
        (x, y) => match (x.number(), y.number()) {
            (Some(x), Some(y)) => x.cmp(&y) as i32,
            _ => 0,
        },
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Num {
    Add,
    Sub,
    Mul,
    Div,
}

pub fn num(op: Num, a: &Value, b: &Value) -> Result<Value, String> {
    let (x, y) = (a.number().unwrap_or_default(), b.number().unwrap_or_default());
    let done = match op {
        Num::Add => x.checked_add(y),
        Num::Sub => x.checked_sub(y),
        Num::Mul => x.checked_mul(y),
        Num::Div if y.is_zero() => return Err("it divided by zero".into()),
        Num::Div => x.checked_div(y),
    };
    done.map(Value::Dec).ok_or_else(|| "it went beyond what a decimal holds".into())
}

/// `round(value, places)`, half away from zero (conformance/rounding.json).
pub fn round(value: &Value, places: i64) -> Value {
    let d = value.number().unwrap_or_default();
    let places = places.clamp(0, 28) as u32;
    Value::Dec(if d.scale() <= places { d } else { d.round_dp_with_strategy(places, rust_decimal::RoundingStrategy::MidpointAwayFromZero) })
}

/// `sin(degrees)` or `cos(degrees)`, rounded to 10 decimals (conformance/drawing.json).
pub fn angle(degrees: &Value, of: fn(f64) -> f64) -> Value {
    let radians = degrees.number().unwrap_or_default().to_f64().unwrap_or(0.0).to_radians();
    let value = Decimal::from_f64_retain(of(radians)).unwrap_or_default();
    let rounded = if value.scale() <= 10 { value } else { value.round_dp_with_strategy(10, rust_decimal::RoundingStrategy::MidpointAwayFromZero) };
    Value::Dec(rounded.normalize())
}

/// `random(max)`: 1 to `max` from the host's next number.
pub fn random(next: i64, max: i64) -> Result<i64, String> {
    if max < 1 {
        return Err(format!("random takes a highest number of 1 or more, not {max}"));
    }
    Ok(1 + ((next as u64) % (max as u64)) as i64)
}

// Lists, items counted from 1.

pub fn push(list: &Value, item: Value) -> Value {
    let mut items = list.items();
    items.push(item);
    Value::List(items)
}

pub fn count(list: &Value) -> i64 {
    if let Value::List(items) = list { items.len() as i64 } else { 0 }
}

pub fn item(list: &Value, at: i64) -> Result<Value, String> {
    let items = list.items();
    if at < 1 || at as usize > items.len() {
        return Err(format!("there is no item {at} in a list of {}", items.len()));
    }
    Ok(items[at as usize - 1].clone())
}

/// The list with item `at` set: a new list, the old one as it was.
pub fn set_item(list: &Value, at: i64, value: Value) -> Result<Value, String> {
    let mut items = list.items();
    if at < 1 || at as usize > items.len() {
        return Err(format!("there is no item {at} in a list of {}", items.len()));
    }
    items[at as usize - 1] = value;
    Ok(Value::List(items))
}

pub fn concat(a: &Value, b: &Value) -> Value {
    let mut items = a.items();
    items.extend(b.items());
    Value::List(items)
}

/// Whether a list holds a value, or a text holds a part, ignoring case.
pub fn contains(whole: &Value, part: &Value) -> bool {
    match (whole, part) {
        (Value::List(items), wanted) => items.contains(wanted),
        (Value::Text(text), Value::Text(part)) => text.to_lowercase().contains(&part.to_lowercase()),
        _ => false,
    }
}

// Records, their fields by name.

/// The record with field `name` set: a new record, the old one as it was.
pub fn with_field(record: &Value, name: &str, value: Value) -> Value {
    let mut fields = if let Value::Record(fields) = record { fields.clone() } else { Vec::new() };
    match fields.iter_mut().find(|(n, _)| n == name) {
        Some(field) => field.1 = value,
        None => fields.push((name.to_string(), value)),
    }
    Value::Record(fields)
}

pub fn field(record: &Value, name: &str) -> Value {
    match record {
        Value::Record(fields) => fields.iter().find(|(n, _)| n == name).map(|(_, v)| v.clone()).unwrap_or(Value::None),
        _ => Value::None,
    }
}

/// A tell with its values, as the answer carries it.
pub fn tell(name: &str, values: &Value) -> Json {
    json!({ "tell": name, "values": Json::Array(values.items().iter().map(Value::json).collect()) })
}
