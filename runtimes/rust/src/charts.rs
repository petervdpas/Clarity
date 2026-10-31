//! A chart's groups (U1c), decided here in the runtime (DESIGN 1.7,
//! conformance/charts.json, `groups`): a list's rows grouped by a field,
//! each group counted, summed or averaged, in the order of their values. The
//! axis and the colors of a pie's slices are how it is drawn, the renderer's.

use crate::reorder::same_id;
use crate::tables::as_text;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::cmp::Ordering;

/// A row of a chart: the value it is grouped by, and its number.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ChartRow {
    #[serde(default)]
    pub by: Value,
    #[serde(default)]
    pub of: Value,
}

/// A group of a chart: its value, and its count, sum or average.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChartGroup {
    pub key: Value,
    pub value: f64,
}

fn number(value: &Value) -> Option<f64> {
    match value {
        Value::Number(n) => n.as_f64(),
        Value::String(s) if !s.trim().is_empty() => s.trim().parse().ok(),
        _ => None,
    }
}

/// The groups of `rows` by the value they have (`by_kind`: text, number,
/// date, enum or bool, with an enum's values in `order`), each the count,
/// sum or average (`measure`) of its numbers. Rows with no value are left out.
pub fn groups(rows: &[ChartRow], by_kind: &str, measure: &str, order: &[Value]) -> Vec<ChartGroup> {
    struct Group {
        key: Value,
        count: usize,
        numbers: usize,
        sum: f64,
    }
    let mut groups: Vec<Group> = vec![];
    for row in rows {
        if row.by.is_null() || row.by.as_str() == Some("") {
            continue;
        }
        let at = match groups.iter().position(|g| same_id(&g.key, &row.by)) {
            Some(at) => at,
            None => {
                groups.push(Group { key: row.by.clone(), count: 0, numbers: 0, sum: 0.0 });
                groups.len() - 1
            }
        };
        let g = &mut groups[at];
        g.count += 1;
        if let Some(n) = number(&row.of) {
            g.sum += n;
            g.numbers += 1;
        }
    }
    let place = |key: &Value| order.iter().position(|v| same_id(v, key)).map_or(-1, |p| p as i64);
    groups.sort_by(|a, b| match by_kind {
        "number" => number(&a.key).unwrap_or(0.0).partial_cmp(&number(&b.key).unwrap_or(0.0)).unwrap_or(Ordering::Equal),
        "enum" => place(&a.key).cmp(&place(&b.key)),
        "bool" => (a.key == Value::Bool(true)).cmp(&(b.key == Value::Bool(true))),
        _ => {
            let (x, y) = (as_text(&a.key), as_text(&b.key));
            let ignoring = if by_kind == "date" { Ordering::Equal } else { x.to_lowercase().cmp(&y.to_lowercase()) };
            ignoring.then_with(|| x.cmp(&y))
        }
    });
    groups
        .into_iter()
        .map(|g| {
            let value = match measure {
                "sum" => g.sum,
                "average" if g.numbers > 0 => g.sum / g.numbers as f64,
                "average" => 0.0,
                _ => g.count as f64,
            };
            // Worked out to 12 decimals, as on every runtime.
            ChartGroup { key: g.key, value: (value * 1e12).round() / 1e12 }
        })
        .collect()
}
