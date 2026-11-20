//! A value as every stack writes it: what a stack's codec wrote, put in
//! Clarity's one form, so `json of` gives the same bytes on every stack. A
//! record's `id` comes first, then its fields in the entity's order, then
//! anything else as it came; a decimal is exact text in its shortest form
//! (no zeros at the end of its fraction, which decimal types keep or drop
//! differently); a whole number has no
//! fraction or exponent; a date and time is `YYYY-MM-DDTHH:MM:SS`; an enum
//! value is written as declared.

use crate::kind::{Kind, Shape};
use crate::numbers;
use crate::tree::Tree;

pub fn normal(value: &Tree, kind: &Kind) -> Tree {
    match (kind, value) {
        (Kind::Record(shape), Tree::Object(_)) => record(value, shape),
        (Kind::Records(shape), Tree::List(items)) => Tree::List(items.iter().map(|i| if matches!(i, Tree::Object(_)) { record(i, shape) } else { i.clone() }).collect()),
        (Kind::Integer, Tree::Number(n)) => numbers::integer(n).map(Tree::Number).unwrap_or_else(|| value.clone()),
        (Kind::Decimal, Tree::Number(n) | Tree::Text(n)) => numbers::shortest(n).map(Tree::Text).unwrap_or_else(|| value.clone()),
        (Kind::Date, Tree::Text(s)) if !numbers::is_date(s) => numbers::date_time(s).map(|d| Tree::Text(d[..10].to_string())).unwrap_or_else(|| value.clone()),
        (Kind::DateTime, Tree::Text(s)) => numbers::date_time(s).map(Tree::Text).unwrap_or_else(|| value.clone()),
        (Kind::Enum(values), Tree::Text(s)) => values.iter().find(|v| v.eq_ignore_ascii_case(s)).map(|v| Tree::Text(v.clone())).unwrap_or_else(|| value.clone()),
        (Kind::Values(item), Tree::List(items)) => Tree::List(items.iter().map(|i| normal(i, item)).collect()),
        _ => value.clone(),
    }
}

fn record(value: &Tree, shape: &Shape) -> Tree {
    let Tree::Object(pairs) = value else { return value.clone() };
    let mut out: Vec<(String, Tree)> = Vec::with_capacity(pairs.len());
    if let Some(id) = value.get("id") {
        out.push(("id".into(), id.clone()));
    }
    for field in &shape.fields {
        if let Some(given) = value.get(&field.name) {
            out.push((field.name.clone(), normal(given, &field.kind)));
        }
    }
    for (name, given) in pairs {
        if !out.iter().any(|(n, _)| n == name) {
            out.push((name.clone(), given.clone()));
        }
    }
    Tree::Object(out)
}
