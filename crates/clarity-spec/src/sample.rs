//! Deterministic sample data for previews. Values respect the field type,
//! text length, numeric bounds, enum variants and references. The same
//! project always produces the same rows, so previews do not flicker.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::model::{Entity, Field, FieldType, ListItem, Literal, Project};

pub type Row = BTreeMap<String, Value>;

pub const ROWS_PER_ENTITY: usize = 8;

const NAMES: [&str; 8] = [
    "Ada Lovelace",
    "Grace Hopper",
    "Alan Turing",
    "Edsger Dijkstra",
    "Barbara Liskov",
    "Niklaus Wirth",
    "Margaret Hamilton",
    "Ken Thompson",
];

const WORDS: [&str; 8] = [
    "alpha", "bravo", "charlie", "delta", "echo", "foxtrot", "golf", "hotel",
];

/// Sample rows for every entity, keyed by entity name.
pub fn sample_data(project: &Project) -> BTreeMap<String, Vec<Row>> {
    let mut data = BTreeMap::new();
    for entity in project.entities() {
        let rows = (0..ROWS_PER_ENTITY)
            .map(|i| sample_row(project, entity, i))
            .collect();
        data.insert(entity.name.name.clone(), rows);
    }
    data
}

/// The display text of every sample row, keyed by entity name. Reference
/// fields show these texts, and pickers offer them as choices.
pub fn display_values(project: &Project) -> BTreeMap<String, Vec<String>> {
    project
        .entities()
        .map(|entity| {
            let texts = (0..ROWS_PER_ENTITY)
                .map(|i| display_value(project, entity, i))
                .collect();
            (entity.name.name.clone(), texts)
        })
        .collect()
}

fn sample_row(project: &Project, entity: &Entity, i: usize) -> Row {
    entity
        .fields
        .iter()
        .map(|field| {
            let value = match &field.ty {
                // A reference to the record's own kind makes a tree, as a
                // tree part shows it: two rows at the top, the others under
                // the rows before them.
                FieldType::Ref { entity: target } if target.name == entity.name.name => {
                    if i < 2 { Value::Null } else { Value::String(display_value(project, entity, (i - 2) / 2)) }
                }
                _ => sample_value(project, field, i),
            };
            (field.name.name.clone(), value)
        })
        .collect()
}

fn sample_value(project: &Project, field: &Field, i: usize) -> Value {
    if i == 0
        && let Some(default) = &field.modifiers.default
    {
        return default_value(field, default);
    }

    let name = field.name.name.as_str();
    match &field.ty {
        FieldType::Text { max_length } => {
            let text = if name.contains("name") {
                NAMES[i % NAMES.len()].to_string()
            } else if field.modifiers.pattern.is_some()
                || name.contains("number")
                || name.contains("code")
            {
                format!(
                    "{}-{:04}",
                    name.to_uppercase().chars().take(3).collect::<String>(),
                    i + 1
                )
            } else {
                // A label by key is not text to show: the field's name stands in for it.
                let label = if field.label.starts_with('@') { crate::model::default_label(name) } else { field.label.clone() };
                format!("{label} {}", WORDS[i % WORDS.len()])
            };
            Value::String(truncate(&text, *max_length))
        }
        FieldType::Integer => json!(bounded(field, (i as i64 + 1) * 7)),
        FieldType::Id => json!(i + 1),
        FieldType::Decimal { scale, .. } => Value::String(format_decimal(
            bounded(field, (i as i64 + 1) * 1234),
            *scale,
        )),
        FieldType::Money => {
            Value::String(format_decimal(bounded(field, (i as i64 + 1) * 125_050), 2))
        }
        FieldType::Bool => Value::Bool(i % 3 != 2),
        // Lengths of songs: 3:05 and up.
        FieldType::Duration => Value::from(bounded(field, 185 + (i as i64 * 37) % 240)),
        // No picture in sample data: the image part shows its placeholder.
        // A record held inside: the preview shows none.
        FieldType::Image | FieldType::File | FieldType::Record { .. } => Value::Null,
        FieldType::Date => {
            Value::String(format!("2026-{:02}-{:02}", (i % 12) + 1, (i * 3 % 28) + 1))
        }
        FieldType::DateTime => Value::String(format!(
            "2026-{:02}-{:02}T{:02}:{:02}:00",
            (i % 12) + 1,
            (i * 3 % 28) + 1,
            (8 + i) % 24,
            (i * 15) % 60
        )),
        FieldType::Email => {
            let local = NAMES[i % NAMES.len()]
                .split(' ')
                .next()
                .unwrap_or("user")
                .to_lowercase();
            Value::String(format!("{local}@example.com"))
        }
        FieldType::Url => Value::String(format!("https://example.com/{}", WORDS[i % WORDS.len()])),
        // The language and the theme the application starts with.
        FieldType::Language => Value::String(project.locale.language.clone().unwrap_or_default()),
        FieldType::Theme => Value::String(project.themes.first().cloned().unwrap_or_default()),
        // The palette's middle row: clear colors.
        FieldType::Secret => Value::String(String::new()),
        FieldType::Color => Value::String(crate::colors::PALETTE[3][i % 10].to_string()),
        FieldType::Enum { name } => match project.enum_decl(&name.name) {
            Some(decl) if !decl.variants.is_empty() => {
                Value::String(decl.variants[i % decl.variants.len()].name.name.clone())
            }
            _ => Value::Null,
        },
        FieldType::Ref { entity } => match project.entity(&entity.name) {
            Some(target) => Value::String(display_value(project, target, i)),
            None => Value::Null,
        },
        FieldType::List { .. } => json!((i % 4) + 1),
        // One to three items, as a tags part shows them.
        FieldType::Values { item } => {
            let items = (0..(i % 3) + 1).map(|n| match item {
                ListItem::Text => Value::String(WORDS[(i + n) % WORDS.len()].to_lowercase()),
                ListItem::Integer => json!((i + n) as i64 + 1),
                ListItem::Decimal => Value::String(format_decimal((i + n) as i64 * 150 + 100, 2)),
                ListItem::Date => Value::String(format!("2026-{:02}-{:02}", ((i + n) % 12) + 1, ((i + n) * 3 % 28) + 1)),
                ListItem::Enum { name } => match project.enum_decl(&name.name) {
                    Some(decl) if !decl.variants.is_empty() => Value::String(decl.variants[(i + n) % decl.variants.len()].name.name.clone()),
                    _ => Value::Null,
                },
            });
            let mut seen = Vec::new();
            Value::Array(items.filter(|v| !v.is_null() && !seen.contains(v) && { seen.push(v.clone()); true }).collect())
        }
    }
}

/// The text shown for a reference: the first text field of the target row.
fn display_value(project: &Project, entity: &Entity, i: usize) -> String {
    entity
        .fields
        .iter()
        .find(|f| matches!(f.ty, FieldType::Text { .. }))
        .map(|f| match sample_value(project, f, i) {
            Value::String(s) => s,
            other => other.to_string(),
        })
        .unwrap_or_else(|| format!("{} #{}", entity.name.name, i + 1))
}

/// A default as the preview shows it: numbers in money and decimal fields
/// get the field's scale, so `default 0` on money reads `0.00`.
fn default_value(field: &Field, literal: &Literal) -> Value {
    let scale = match field.ty {
        FieldType::Money => Some(2),
        FieldType::Decimal { scale, .. } => Some(scale),
        _ => None,
    };
    match (literal, scale) {
        (Literal::Number(n), Some(scale)) => match n.parse::<f64>() {
            Ok(v) => Value::String(format_decimal(
                (v * 10_f64.powi(scale as i32)).round() as i64,
                scale,
            )),
            Err(_) => literal_value(literal),
        },
        _ => literal_value(literal),
    }
}

fn literal_value(literal: &Literal) -> Value {
    match literal {
        Literal::Number(n) => Value::String(n.clone()),
        Literal::Str(s) | Literal::Ident(s) => Value::String(s.clone()),
        Literal::Bool(b) => Value::Bool(*b),
    }
}

/// Clamps a generated value (in the smallest unit) to the field's bounds.
fn bounded(field: &Field, value: i64) -> i64 {
    let scale = match field.ty {
        FieldType::Money => 100,
        FieldType::Decimal { scale, .. } => 10_i64.pow(scale),
        _ => 1,
    };
    let bound = |literal: &Option<Literal>| match literal {
        Some(Literal::Number(n)) => n.parse::<f64>().ok().map(|v| (v * scale as f64) as i64),
        _ => None,
    };
    let mut value = value;
    if let Some(min) = bound(&field.modifiers.min) {
        value = value.max(min);
    }
    if let Some(max) = bound(&field.modifiers.max) {
        value = value.min(max);
    }
    value
}

fn format_decimal(units: i64, scale: u32) -> String {
    if scale == 0 {
        return units.to_string();
    }
    let divisor = 10_i64.pow(scale);
    let sign = if units < 0 { "-" } else { "" };
    let units = units.abs();
    format!(
        "{sign}{}.{:0width$}",
        units / divisor,
        units % divisor,
        width = scale as usize
    )
}

fn truncate(text: &str, max_length: Option<u32>) -> String {
    match max_length {
        Some(max) => text.chars().take(max as usize).collect(),
        None => text.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_decimals_with_scale() {
        assert_eq!(format_decimal(125_050, 2), "1250.50");
        assert_eq!(format_decimal(-5, 2), "-0.05");
        assert_eq!(format_decimal(42, 0), "42");
    }

    #[test]
    fn truncates_to_max_length() {
        assert_eq!(truncate("Edsger Dijkstra", Some(6)), "Edsger");
        assert_eq!(truncate("short", None), "short");
    }
}
