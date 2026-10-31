//! Reordering (DESIGN 5.30), decided here, not in the screen (DESIGN 1.7):
//! a row of a list, table, tree or tree table moved by a drag or by Alt and
//! an arrow key. The result is the list's new order, each row's parent (in a
//! tree) and each row's number at its level, which goes into the field of
//! `reorder by`. A move that changes no parent or number is refused. Every
//! runtime moves alike (conformance/reorder.json).

use crate::ui::{Reply, WindowState};
use crate::{Db, Error, Result};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

/// A row of the list: its id and, in a tree, the id of its parent (null at the top).
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub id: Value,
    pub parent: Value,
}

/// Where a dragged row is dropped: before or after a row, or into it (in a tree).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DropAt {
    Before,
    After,
    Into,
}

/// A key that moves the selected row, with Alt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MoveKey {
    Up,
    Down,
    Left,
    Right,
}

/// A drag onto a row, or a key on the moved row; places count from 0 in the list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
pub enum Move {
    Drag { row: usize, #[serde(rename = "where")] at: DropAt, target: usize },
    Key { row: usize, key: MoveKey },
}

impl Move {
    /// The place of the row that moves.
    pub fn row(&self) -> usize {
        match *self {
            Move::Drag { row, .. } | Move::Key { row, .. } => row,
        }
    }
}

/// The list after a move: the old places in the new order, each row's parent and number in that order.
#[derive(Debug, Clone, PartialEq)]
pub struct Reordered {
    pub order: Vec<usize>,
    pub parents: Vec<Value>,
    pub numbers: Vec<i64>,
}

/// Whether two ids are the same: `1` and `1.0` are one number.
pub(crate) fn same_id(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x == y || x.as_f64() == y.as_f64(),
        _ => a == b,
    }
}

/// The place of each row's parent, `None` at the top; a loop is broken at
/// its row that comes first in the list (as conformance/trees.json says).
pub(crate) fn parents(rows: &[Row]) -> Vec<Option<usize>> {
    let mut at: Vec<(&Value, usize)> = vec![];
    for (i, row) in rows.iter().enumerate() {
        if !row.id.is_null() && !at.iter().any(|(id, _)| same_id(id, &row.id)) {
            at.push((&row.id, i));
        }
    }
    let mut parent: Vec<Option<usize>> = rows.iter().map(|r| at.iter().find(|(id, _)| same_id(id, &r.parent)).map(|(_, i)| *i)).collect();
    for i in 0..rows.len() {
        let mut seen = vec![];
        let mut p = Some(i);
        while let Some(at) = p {
            if seen.contains(&at) {
                break;
            }
            seen.push(at);
            p = parent[at];
        }
        if let Some(again) = p {
            let from = seen.iter().position(|s| *s == again).unwrap_or(0);
            if let Some(first) = seen[from..].iter().min() {
                parent[*first] = None;
            }
        }
    }
    parent
}

/// The places of the rows whose parent is `place` (`None`: the top), in the order of the list.
fn level(levels: &[Option<usize>], place: Option<usize>) -> Vec<usize> {
    (0..levels.len()).filter(|i| levels[*i] == place).collect()
}

/// Each row's place at its level counted from 1, in the order of the list.
fn numbered(levels: &[Option<usize>], order: &[usize]) -> Vec<i64> {
    let mut counts: HashMap<Option<usize>, i64> = HashMap::new();
    order
        .iter()
        .map(|place| {
            let n = counts.entry(levels[*place]).or_default();
            *n += 1;
            *n
        })
        .collect()
}

/// The rows after `mv`, or none when it changes nothing. In a list that is
/// no tree, rows have no parent.
pub fn reorder(given: &[Row], mv: &Move, tree: bool) -> Option<Reordered> {
    let r = mv.row();
    if r >= given.len() {
        return None;
    }
    let rows: Vec<Row> = given.iter().map(|g| Row { id: g.id.clone(), parent: if tree { g.parent.clone() } else { Value::Null } }).collect();
    let levels = parents(&rows);
    let under = |place: usize| {
        let mut p = Some(place);
        while let Some(at) = p {
            if at == r {
                return true;
            }
            p = levels[at];
        }
        false
    };
    // The last of a row and the rows right under it, leaving out the moved row.
    let last_of = |place: usize| level(&levels, Some(place)).into_iter().filter(|i| *i != r).chain([place]).max().unwrap_or(place);
    let (anchor, after, parent) = match *mv {
        Move::Key { key, .. } => {
            let siblings = level(&levels, levels[r]);
            let k = siblings.iter().position(|s| *s == r)?;
            match key {
                MoveKey::Up | MoveKey::Down => {
                    let next = if key == MoveKey::Up { k.checked_sub(1)? } else { k + 1 };
                    (*siblings.get(next)?, key == MoveKey::Down, rows[r].parent.clone())
                }
                _ if !tree => return None,
                MoveKey::Left => {
                    let up = levels[r]?;
                    (up, true, rows[up].parent.clone())
                }
                MoveKey::Right => {
                    let before = siblings[k.checked_sub(1)?];
                    (last_of(before), true, rows[before].id.clone())
                }
            }
        }
        Move::Drag { at, target, .. } => {
            if target >= rows.len() || target == r || (tree && under(target)) {
                return None;
            }
            match at {
                DropAt::Into if !tree => return None,
                DropAt::Into => (last_of(target), true, rows[target].id.clone()),
                _ => (target, at == DropAt::After, if tree { rows[target].parent.clone() } else { Value::Null }),
            }
        }
    };
    let mut order: Vec<usize> = (0..rows.len()).filter(|i| *i != r).collect();
    let at = order.iter().position(|i| *i == anchor)? + usize::from(after);
    order.insert(at, r);
    let mut moved = rows.clone();
    moved[r].parent = parent;
    let in_order: Vec<Row> = order.iter().map(|i| moved[*i].clone()).collect();
    let new_levels = parents(&in_order);
    // Levels by old place, so both lists are numbered alike.
    let mut by_place = vec![None; rows.len()];
    for (i, place) in order.iter().enumerate() {
        by_place[*place] = new_levels[i].map(|l| order[l]);
    }
    let numbers = numbered(&by_place, &order);
    let before = numbered(&levels, &(0..rows.len()).collect::<Vec<_>>());
    let same = order.iter().enumerate().all(|(i, place)| numbers[i] == before[*place] && same_id(&moved[*place].parent, &rows[*place].parent));
    if same {
        return None;
    }
    Some(Reordered { parents: order.iter().map(|i| moved[*i].parent.clone()).collect(), order, numbers })
}

/// `reorder_event` (DESIGN 1.7): the rows of `list` in the window's data,
/// moved by `mv`. The list takes its new order, each row its number in `by`
/// and, when `parent` names a field, its parent; the moved row is selected
/// in `part`. Then `run` runs the handler as an edit ends (`edits::ended`).
/// A refused move answers the window as it is (`draw`).
#[allow(clippy::too_many_arguments)]
pub fn event(
    db: &Db,
    mut state: WindowState,
    part: &str,
    list: &str,
    by: &str,
    parent: Option<&str>,
    mv: &Move,
    scope: Option<&str>,
    run: impl FnOnce(WindowState) -> Result<Reply>,
    draw: impl FnOnce(WindowState) -> Result<Reply>,
) -> Result<Reply> {
    let parent = parent.filter(|p| !p.is_empty());
    let given = match crate::ui::data_at(&state.data, list) {
        Some(Value::Array(rows)) => rows.clone(),
        Some(Value::Null) | None => vec![],
        Some(_) => return Err(Error::Invalid(format!("{list} is no list"))),
    };
    let rows: Vec<Row> = given
        .iter()
        .map(|g| Row { id: g.get("id").cloned().unwrap_or(Value::Null), parent: parent.and_then(|p| g.get(p).cloned()).unwrap_or(Value::Null) })
        .collect();
    let Some(moved) = reorder(&rows, mv, parent.is_some()) else { return draw(state) };
    let mut after = Vec::with_capacity(given.len());
    for (i, place) in moved.order.iter().enumerate() {
        let mut row = given[*place].clone();
        if let Some(fields) = row.as_object_mut() {
            if !by.is_empty() {
                fields.insert(by.to_string(), Value::from(moved.numbers[i]));
            }
            if let Some(p) = parent {
                fields.insert(p.to_string(), moved.parents[i].clone());
            }
        }
        after.push(row);
    }
    crate::ui::set_data_at(&mut state.data, list, Value::Array(after))?;
    if let Some(at) = moved.order.iter().position(|place| *place == mv.row()) {
        state.selected.insert(part.to_string(), at);
    }
    crate::edits::ended(db, state, part, scope, run)
}
