//! The fit: what a format read is laid over what the program wants, the
//! same for every format and every stack. Names match by their exact name,
//! else ignoring case, underscores, hyphens and spaces, in the order they
//! were written; what is left out (or null) keeps the new record's value;
//! names the entity does not have, `id` and what is never read are left
//! aside; numbers and strings of numbers stand for each other; a value that
//! does not fit gives a message that says where, word for word on every stack.

use crate::kind::{Kind, Shape};
use crate::numbers;
use crate::tree::{self, Tree};

/// `tree`, read from a format called `name` (`JSON`), from `at` inward
/// (object names with dots between), fitted to `kind`.
pub fn fit(tree: Tree, at: Option<&str>, kind: &Kind, name: &str) -> Result<Tree, String> {
    let mut value = tree;
    if let Some(at) = at.filter(|a| !a.is_empty()) {
        let mut walked = Vec::new();
        for step in at.split('.') {
            walked.push(step);
            value = match value {
                Tree::Object(pairs) => pairs.into_iter().find(|(n, _)| n == step).map(|(_, v)| v),
                _ => None,
            }
            .ok_or_else(|| format!("the {name} has no '{}'", walked.join(".")))?;
        }
    }
    match kind {
        Kind::Records(shape) => {
            let Tree::List(items) = value else { return Err(format!("the {name} is not a list of {}", shape.entity)) };
            let mut fitted = Vec::with_capacity(items.len());
            for (i, item) in items.iter().enumerate() {
                match item {
                    Tree::Object(_) => fitted.push(record(item, shape).map_err(|why| format!("item {}: {why}", i + 1))?),
                    _ => return Err(format!("item {} of the list is not one {}", i + 1, shape.entity)),
                }
            }
            Ok(Tree::List(fitted))
        }
        Kind::Record(shape) => match &value {
            Tree::Object(_) => record(&value, shape),
            _ => Err(format!("the {name} is not one {}", shape.entity)),
        },
        other => self::value(&value, other).ok_or_else(|| format!("the {name} is {}, not {}", other.wanted(), tree::write(&value))),
    }
}

/// A name as names are matched: lower case, without underscores, hyphens and spaces.
pub fn plain(name: &str) -> String {
    name.chars().filter(|c| !matches!(c, '_' | '-' | ' ')).flat_map(char::to_lowercase).collect()
}

/// The value a record gives for `field`: by its exact name, else the first by its plain name, in the order written.
pub fn found<'a>(object: &'a Tree, field: &str) -> Option<&'a Tree> {
    let Tree::Object(pairs) = object else { return None };
    pairs.iter().find(|(n, _)| n == field).or_else(|| {
        let wanted = plain(field);
        pairs.iter().find(|(n, _)| plain(n) == wanted)
    })
    .map(|(_, v)| v)
}

fn record(object: &Tree, shape: &Shape) -> Result<Tree, String> {
    let entity = &shape.entity;
    let mut out = shape.base.clone();
    for field in &shape.fields {
        let Some(given) = found(object, &field.name).filter(|v| **v != Tree::Null) else { continue };
        let value = match &field.kind {
            Kind::Other => continue,
            Kind::Record(nested) => match given {
                Tree::Object(_) => record(given, nested).map_err(|why| format!("field '{}' of {entity}: {why}", field.name))?,
                other => return Err(format!("field '{}' of {entity} is one {}, not {}", field.name, nested.entity, tree::write(other))),
            },
            Kind::Records(nested) => {
                let Tree::List(items) = given else {
                    return Err(format!("field '{}' of {entity} is a list of {}, not {}", field.name, nested.entity, tree::write(given)));
                };
                let mut fitted = Vec::with_capacity(items.len());
                for (i, item) in items.iter().enumerate() {
                    if !matches!(item, Tree::Object(_)) {
                        return Err(format!("field '{}' of {entity}, item {} is not one {}", field.name, i + 1, nested.entity));
                    }
                    fitted.push(record(item, nested).map_err(|why| format!("field '{}' of {entity}, item {}: {why}", field.name, i + 1))?);
                }
                Tree::List(fitted)
            }
            kind => value(given, kind).ok_or_else(|| format!("field '{}' of {entity} is {}, not {}", field.name, kind.wanted(), tree::write(given)))?,
        };
        out.set(&field.name, value);
    }
    Ok(out)
}

/// `given` fitted to a plain `kind`, or none when it does not fit.
pub fn value(given: &Tree, kind: &Kind) -> Option<Tree> {
    match (kind, given) {
        (Kind::Text, Tree::Text(_)) => Some(given.clone()),
        // A number or yes and no as text, as it was written.
        (Kind::Text, Tree::Number(n)) => Some(Tree::Text(n.clone())),
        (Kind::Text, Tree::Bool(b)) => Some(Tree::Text(b.to_string())),
        (Kind::Integer, Tree::Number(n)) => numbers::integer(n).map(Tree::Number),
        (Kind::Integer, Tree::Text(s)) => numbers::integer_text(s.trim()).map(Tree::Number),
        (Kind::Decimal, Tree::Number(n)) => numbers::plain(n).map(Tree::Text),
        (Kind::Decimal, Tree::Text(s)) if numbers::is_decimal(s.trim()) => Some(Tree::Text(s.trim().to_string())),
        (Kind::Bool, Tree::Bool(_)) => Some(given.clone()),
        (Kind::Date, Tree::Text(s)) if numbers::is_date(s) => Some(given.clone()),
        (Kind::Date, Tree::Text(s)) => numbers::date_time(s).map(|d| Tree::Text(d[..10].to_string())),
        (Kind::DateTime, Tree::Text(s)) => numbers::date_time(s).map(Tree::Text),
        (Kind::Enum(values), Tree::Text(s)) => values.iter().find(|v| v.eq_ignore_ascii_case(s)).map(|v| Tree::Text(v.clone())),
        (Kind::Values(item), Tree::List(items)) => items.iter().map(|i| value(i, item)).collect::<Option<Vec<_>>>().map(Tree::List),
        _ => None,
    }
}
