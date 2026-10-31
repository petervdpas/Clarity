//! A list field's typed item (U1c), decided here, not in the screen
//! (DESIGN 1.7): trimmed; empty text and an item the list has add nothing;
//! text that is no value of the list's kind is refused, and the screen keeps
//! it as typed. Every runtime adds alike (conformance/list-fields.json, added).

use crate::ui::{Reply, WindowState};
use crate::{Db, Decimal, Error, NaiveDate, Result};
use serde::Deserialize;
use serde_json::Value;

/// What the items of a list field are: `list<text>`, `list<Genre>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ItemKind {
    Text,
    Integer,
    Decimal,
    Date,
    Enum,
}

/// What a typed item does to a list.
#[derive(Debug, Clone, PartialEq)]
pub enum Added {
    /// The list it gives: the same list when nothing is added.
    Items(Vec<Value>),
    /// The text is no value of the list's kind.
    Refused,
}

/// Digits with a minus in front, if any: `+5` and `5.` are no whole number.
fn whole(text: &str) -> bool {
    let digits = text.strip_prefix('-').unwrap_or(text);
    !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
}

fn decimal(text: &str) -> bool {
    match text.split_once('.') {
        Some((w, f)) => whole(w) && !f.is_empty() && f.bytes().all(|b| b.is_ascii_digit()),
        None => whole(text),
    }
}

/// Whether two items are the same value: `1.50` and `1.5` are one decimal.
fn same(kind: ItemKind, a: &Value, b: &Value) -> bool {
    if kind == ItemKind::Decimal {
        let number = |v: &Value| match v {
            Value::String(s) => s.parse::<Decimal>().ok(),
            Value::Number(n) => n.to_string().parse::<Decimal>().ok(),
            _ => None,
        };
        if let (Some(x), Some(y)) = (number(a), number(b)) {
            return x == y;
        }
    }
    a == b
}

/// The list `items` with the item typed as `typed` added at its end; an
/// enum's item must be one of its `values`.
pub fn added(kind: ItemKind, values: &[String], items: &[Value], typed: &str) -> Added {
    let text = typed.trim();
    if text.is_empty() {
        return Added::Items(items.to_vec());
    }
    let item = match kind {
        ItemKind::Integer => match text.parse::<i64>() {
            Ok(n) if whole(text) => Value::from(n),
            _ => return Added::Refused,
        },
        ItemKind::Decimal if !decimal(text) || text.parse::<Decimal>().is_err() => return Added::Refused,
        ItemKind::Date if NaiveDate::parse_from_str(text, "%Y-%m-%d").is_err() || text.len() != 10 => return Added::Refused,
        ItemKind::Enum if !values.iter().any(|v| v == text) => return Added::Refused,
        _ => Value::from(text),
    };
    let mut list = items.to_vec();
    if !list.iter().any(|i| same(kind, i, &item)) {
        list.push(item);
    }
    Added::Items(list)
}

/// `item_event` (DESIGN 1.7): an item typed for the list at `path` in the
/// window's data. An added item sets the list, then `run` runs the handler
/// as an edit ends (`edits::ended`); nothing added answers the window as it
/// is (`draw`), and a refused item too, with the reply saying `refused`.
#[allow(clippy::too_many_arguments)]
pub fn event(
    db: &Db,
    mut state: WindowState,
    part: &str,
    path: &str,
    kind: ItemKind,
    values: &[String],
    typed: &str,
    scope: Option<&str>,
    run: impl FnOnce(WindowState) -> Result<Reply>,
    draw: impl FnOnce(WindowState) -> Result<Reply>,
) -> Result<Reply> {
    let items = match crate::ui::data_at(&state.data, path) {
        Some(Value::Array(items)) => items.clone(),
        Some(Value::Null) | None => vec![],
        Some(_) => return Err(Error::Invalid(format!("{path} is no list"))),
    };
    match added(kind, values, &items, typed) {
        Added::Refused => {
            let mut reply = draw(state)?;
            reply.refused = true;
            Ok(reply)
        }
        Added::Items(list) if list == items => draw(state),
        Added::Items(list) => {
            crate::ui::set_data_at(&mut state.data, path, Value::Array(list))?;
            crate::edits::ended(db, state, part, scope, run)
        }
    }
}
