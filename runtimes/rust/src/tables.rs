//! How a table's rows show, decided here in the runtime (DESIGN 1.7,
//! conformance/table-order.json, table-pages.json): text ignoring case,
//! numbers as numbers, a choice by the label it shows, an empty value last
//! either way, equal rows in the order of the data; then cut into pages.
//! Rows keep their place in the data; only the order they show in changes.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::cmp::Ordering;

/// A column a table sorts by: its field, its kind (text, number, bool,
/// choice) and, for a choice, each value with its label.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TableColumn {
    pub field: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub choices: Vec<Vec<Value>>,
}

/// A table's order: by a field, `desc` the other way.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableOrder {
    pub field: String,
    #[serde(default)]
    pub desc: bool,
}

/// What a click on the header of `field` makes of `order`: by it, then the other way.
pub fn next_order(order: Option<&TableOrder>, field: &str) -> TableOrder {
    match order {
        Some(o) if o.field == field => TableOrder { field: field.to_string(), desc: !o.desc },
        _ => TableOrder { field: field.to_string(), desc: false },
    }
}

/// A value as text: text as it is, anything else as JSON writes it.
pub(crate) fn as_text(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// What a value sorts by; none when it is empty.
enum Key {
    Number(f64),
    Text(String),
}

fn sort_key(column: &TableColumn, value: Option<&Value>) -> Option<Key> {
    let value = value.filter(|v| !v.is_null() && v.as_str() != Some(""))?;
    match column.kind.as_str() {
        "choice" => {
            let label = column
                .choices
                .iter()
                .find(|c| c.len() == 2 && crate::reorder::same_id(&c[0], value))
                .map(|c| as_text(&c[1]))
                .unwrap_or_else(|| as_text(value));
            (!label.is_empty()).then_some(Key::Text(label))
        }
        "number" => as_text(value).trim().parse::<f64>().ok().map(Key::Number),
        "bool" => Some(Key::Number(if value == &Value::Bool(true) { 1.0 } else { 0.0 })),
        _ => Some(Key::Text(as_text(value))),
    }
}

/// The places in the data of `rows`, in the order they show.
pub fn shown_order(rows: &[Map<String, Value>], columns: &[TableColumn], order: Option<&TableOrder>) -> Vec<usize> {
    let mut places: Vec<usize> = (0..rows.len()).collect();
    let Some(order) = order else { return places };
    let Some(column) = columns.iter().find(|c| c.field == order.field) else { return places };
    let keys: Vec<Option<Key>> = rows.iter().map(|r| sort_key(column, r.get(&order.field))).collect();
    places.sort_by(|a, b| match (&keys[*a], &keys[*b]) {
        (None, None) => Ordering::Equal,
        (None, _) => Ordering::Greater,
        (_, None) => Ordering::Less,
        (Some(x), Some(y)) => {
            let by = match (x, y) {
                (Key::Number(x), Key::Number(y)) => x.partial_cmp(y).unwrap_or(Ordering::Equal),
                (x, y) => text_of(x).to_lowercase().cmp(&text_of(y).to_lowercase()),
            };
            if order.desc { by.reverse() } else { by }
        }
    });
    places
}

fn text_of(key: &Key) -> String {
    match key {
        Key::Number(n) => n.to_string(),
        Key::Text(t) => t.clone(),
    }
}

/// The page of a table shown a page at a time: the page shown, how many
/// pages there are, the first and last row shown counted from 1 (0 and 0
/// for none), and how many rows there are.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Paged {
    pub page: usize,
    pub pages: usize,
    pub from: usize,
    pub to: usize,
    pub count: usize,
}

impl Paged {
    /// The bar's text in plain digits, which the screen writes in its own.
    pub fn text(&self) -> String {
        if self.count == 0 { "0 / 0".to_string() } else { format!("{}-{} / {}", self.from, self.to, self.count) }
    }
}

/// Page `page` of `count` rows, `size` to a page, within the pages there are.
pub fn page_of(count: usize, size: usize, page: i64) -> Paged {
    let size = size.max(1);
    let pages = count.div_ceil(size).max(1);
    if count == 0 {
        return Paged { page: 1, pages: 1, from: 0, to: 0, count: 0 };
    }
    let shown = (page.max(1) as u64).min(pages as u64) as usize;
    let from = (shown - 1) * size + 1;
    Paged { page: shown, pages, from, to: (shown * size).min(count), count }
}

/// The page that shows the row at place `at` of the shown order, counted from 0.
pub fn page_with(size: usize, at: usize) -> usize {
    at / size.max(1) + 1
}
