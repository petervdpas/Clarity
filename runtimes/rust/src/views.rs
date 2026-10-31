//! The shown rows of tables, lists, trees and charts (DESIGN 1.7), decided
//! here in the runtime: the window's state carries a view per such part,
//! how it shows its list as the screen sends it, and after every reply the
//! runtime fills in what shows. A view event (a sort, a page, a tree row
//! opened or closed) changes the view only, and is no edit.

use crate::charts::{self, ChartRow};
use crate::reorder::{Row, same_id};
use crate::tables::{self, TableColumn, TableOrder};
use crate::trees;
use crate::ui::{Reply, WindowState, data_at};
use crate::{Error, Result};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value};
use std::collections::BTreeMap;

/// A table's view: its list, its columns, its order, the rows to a page
/// (none: no pages) and the page it is at.
#[derive(Debug, Default, Deserialize)]
struct TableView {
    #[serde(default)]
    list: String,
    #[serde(default)]
    columns: Vec<TableColumn>,
    #[serde(default)]
    order: Option<TableOrder>,
    #[serde(default)]
    page: Option<usize>,
    #[serde(default)]
    at: Option<i64>,
}

/// A tree's view: its list, its parent and id fields, and the ids shown open.
#[derive(Debug, Default, Deserialize)]
struct TreeView {
    #[serde(default)]
    list: String,
    #[serde(default)]
    parent: Option<String>,
    #[serde(default)]
    key: Option<String>,
    #[serde(default)]
    open: Option<Vec<Value>>,
}

/// A chart's view: its list, the field it groups by and its kind, the
/// measure and its field, and an enum's values in their order.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChartView {
    #[serde(default)]
    list: String,
    #[serde(default)]
    by: String,
    #[serde(default)]
    by_kind: String,
    #[serde(default)]
    measure: String,
    #[serde(default)]
    of: Option<String>,
    #[serde(default)]
    values: Option<Vec<Value>>,
}

fn view_as<T: DeserializeOwned>(view: &Map<String, Value>) -> Result<T> {
    serde_json::from_value(Value::Object(view.clone())).map_err(|err| Error::Invalid(format!("a view does not fit its kind: {err}")))
}

/// The rows of the list at `path`; a row that is no record has no fields.
fn rows_at(data: &Map<String, Value>, path: &str) -> Vec<Map<String, Value>> {
    match data_at(data, path) {
        Some(Value::Array(list)) => list.iter().map(|r| r.as_object().cloned().unwrap_or_default()).collect(),
        _ => vec![],
    }
}

fn tree_rows(rows: &[Map<String, Value>], v: &TreeView) -> Vec<Row> {
    let key = v.key.as_deref().filter(|k| !k.is_empty()).unwrap_or("id");
    let parent = v.parent.as_deref().filter(|p| !p.is_empty());
    rows.iter()
        .map(|r| Row { id: r.get(key).cloned().unwrap_or(Value::Null), parent: parent.and_then(|p| r.get(p).cloned()).unwrap_or(Value::Null) })
        .collect()
}

fn json<T: serde::Serialize>(value: T) -> Value {
    serde_json::to_value(value).unwrap_or(Value::Null)
}

/// Fills in what each view of `state` shows. `chosen` names the parts whose
/// chosen row counts: a table turns to the page that shows it, a tree opens
/// the rows above it.
pub fn fill(state: &mut WindowState, chosen: impl Fn(&str) -> bool) -> Result<()> {
    for (part, view) in state.views.iter_mut() {
        let picked = state.selected.get(part).copied().filter(|_| chosen(part));
        match view.get("kind").and_then(Value::as_str) {
            Some("table") => {
                let v: TableView = view_as(view)?;
                let rows = rows_at(&state.data, &v.list);
                let order = tables::shown_order(&rows, &v.columns, v.order.as_ref());
                let Some(size) = v.page else {
                    view.insert("shown".into(), json(&order));
                    view.remove("pages");
                    continue;
                };
                let mut at = v.at.unwrap_or(1);
                if let Some(place) = picked.and_then(|row| order.iter().position(|p| *p == row)) {
                    at = tables::page_with(size, place) as i64;
                }
                let paged = tables::page_of(order.len(), size, at);
                view.insert("at".into(), json(paged.page));
                view.insert("pages".into(), json(paged));
                let from = paged.from.saturating_sub(1);
                view.insert("shown".into(), json(&order[from..paged.to.max(from)]));
            }
            Some("tree") => {
                let v: TreeView = view_as(view)?;
                let rows = tree_rows(&rows_at(&state.data, &v.list), &v);
                let mut open = v.open.unwrap_or_default();
                if let Some(row) = picked {
                    for id in trees::above(&rows, row) {
                        if !open.iter().any(|o| same_id(o, &id)) {
                            open.push(id);
                        }
                    }
                }
                view.insert("shown".into(), json(trees::shown_tree(&rows, &open)));
                view.insert("open".into(), Value::Array(open));
            }
            Some("numbers") => {
                let points = view.get("points").and_then(Value::as_u64).unwrap_or(0) as usize;
                view.insert("numbers".into(), json(numbers_of(&state.data, &list_of(view), points)));
            }
            Some("chart") => {
                let v: ChartView = view_as(view)?;
                let of = v.of.as_deref().filter(|o| !o.is_empty());
                let rows: Vec<ChartRow> = rows_at(&state.data, &v.list)
                    .iter()
                    .map(|r| ChartRow { by: r.get(&v.by).cloned().unwrap_or(Value::Null), of: of.and_then(|o| r.get(o).cloned()).unwrap_or(Value::Null) })
                    .collect();
                view.insert("groups".into(), json(charts::groups(&rows, &v.by_kind, &v.measure, &v.values.unwrap_or_default())));
            }
            _ => {}
        }
    }
    Ok(())
}

fn list_of(view: &Map<String, Value>) -> String {
    view.get("list").and_then(Value::as_str).unwrap_or_default().to_string()
}

/// The numbers of the list at `path` a chart of numbers draws: its last
/// `points` (all when 0), decimals as their value; what is no number, or
/// blank text, is left out.
pub fn numbers_of(data: &Map<String, Value>, path: &str, points: usize) -> Vec<f64> {
    let numbers: Vec<f64> = match data_at(data, path) {
        Some(Value::Array(list)) => list
            .iter()
            .filter_map(|item| match item {
                Value::Number(n) => n.as_f64(),
                Value::String(s) if !s.trim().is_empty() => s.trim().parse::<f64>().ok(),
                _ => None,
            })
            .filter(|n| n.is_finite())
            .collect(),
        _ => vec![],
    };
    let skip = if points > 0 { numbers.len().saturating_sub(points) } else { 0 };
    numbers[skip..].to_vec()
}

/// What the screen sent with a message, which its reply's views start from.
#[derive(Debug, Clone, Default)]
pub struct Sent {
    instance: u64,
    views: BTreeMap<String, Map<String, Value>>,
    /// The chosen rows as the screen sent them, for a view event only: there
    /// only a chosen row that changed counts. On every other reply each
    /// chosen row counts, as a move can put it under a closed row.
    selected: Option<BTreeMap<String, usize>>,
}

impl Sent {
    /// A message of any kind but a view event.
    pub fn of(state: &WindowState) -> Sent {
        Sent { instance: state.instance, views: state.views.clone(), selected: None }
    }

    /// A view event: the person's own sort, page or close stays.
    pub fn view_event(state: &WindowState) -> Sent {
        Sent { selected: Some(state.selected.clone()), ..Sent::of(state) }
    }

    /// A message that brings no window, as opening one.
    pub fn none() -> Sent {
        Sent::default()
    }
}

/// Fills the views of a window's reply (DESIGN 1.7); a reply of the same
/// window that lost its views keeps those the screen sent.
pub fn replied(sent: &Sent, reply: &mut Reply) -> Result<()> {
    if reply.state.views.is_empty() && reply.state.instance == sent.instance {
        reply.state.views = sent.views.clone();
    }
    let selected = reply.state.selected.clone();
    fill(&mut reply.state, |part| match &sent.selected {
        None => true,
        Some(was) => was.get(part) != selected.get(part),
    })
}

/// `view_event` (DESIGN 1.7): the view of `part` changed by the person's
/// `action`: `sort` by a field (the same field again: the other way, from
/// the first page), `page` to a page, `open` or `close` a tree row by its
/// id. An empty action changes nothing: the screen declared a view, which
/// every reply fills. Then `draw` answers the window.
pub fn event(mut state: WindowState, part: &str, action: &Map<String, Value>, draw: impl FnOnce(WindowState) -> Result<Reply>) -> Result<Reply> {
    if action.is_empty() {
        return draw(state);
    }
    let view = state.views.get_mut(part).ok_or_else(|| Error::Invalid(format!("the window shows no rows in {part}")))?;
    let unfit = |name: &str| Error::Invalid(format!("a view's {name} does not fit: {}", action[name]));
    if let Some(field) = action.get("sort") {
        let field = field.as_str().ok_or_else(|| unfit("sort"))?;
        // Only a sortable table sorts by a header.
        if view.get("sortable") != Some(&Value::Bool(true)) {
            return draw(state);
        }
        let v: TableView = view_as(view)?;
        view.insert("order".into(), json(tables::next_order(v.order.as_ref(), field)));
        view.insert("at".into(), Value::from(1));
    } else if let Some(page) = action.get("page") {
        let page = page.as_i64().ok_or_else(|| unfit("page"))?;
        view.insert("at".into(), Value::from(page));
    } else if let Some((name, id)) = action.get_key_value("open").or_else(|| action.get_key_value("close")) {
        let v: TreeView = view_as(view)?;
        let mut open: Vec<Value> = v.open.unwrap_or_default().into_iter().filter(|o| !same_id(o, id)).collect();
        if name == "open" {
            open.push(id.clone());
        }
        view.insert("open".into(), Value::Array(open));
    } else {
        return Err(Error::Invalid("a view event sorts, turns a page, or opens or closes a row".into()));
    }
    draw(state)
}
