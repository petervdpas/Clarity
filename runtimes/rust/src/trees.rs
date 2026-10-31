//! Trees of U1c, decided here in the runtime (DESIGN 1.7,
//! conformance/trees.json): the rows of a list arranged by their parent, a
//! row under its parent when that is in the list and at the top otherwise,
//! each level in the order of the list, a closed row hiding what is under it.

use crate::reorder::{Row, parents, same_id};
use serde::Serialize;
use serde_json::Value;

/// A row a tree shows: its place in the list, its depth, and whether rows
/// are under it; JSON writes it as `[place, depth, children]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct TreeShown(pub usize, pub usize, pub bool);

/// The rows shown, top to bottom, with `open` the ids of the rows shown open.
pub fn shown_tree(rows: &[Row], open: &[Value]) -> Vec<TreeShown> {
    let parent = parents(rows);
    let mut under: Vec<Vec<usize>> = vec![vec![]; rows.len()];
    let mut tops = vec![];
    for (i, p) in parent.iter().enumerate() {
        match p {
            Some(p) => under[*p].push(i),
            None => tops.push(i),
        }
    }
    let mut shown = vec![];
    // A loop is broken in `parents`, so the walk ends.
    fn walk(place: usize, depth: usize, rows: &[Row], under: &[Vec<usize>], open: &[Value], shown: &mut Vec<TreeShown>) {
        shown.push(TreeShown(place, depth, !under[place].is_empty()));
        if open.iter().any(|o| same_id(o, &rows[place].id)) {
            for child in &under[place] {
                walk(*child, depth + 1, rows, under, open, shown);
            }
        }
    }
    for top in tops {
        walk(top, 0, rows, &under, open, &mut shown);
    }
    shown
}

/// The ids of the rows above the row at place `at`, top first: what a tree
/// opens when that row is chosen.
pub fn above(rows: &[Row], at: usize) -> Vec<Value> {
    if at >= rows.len() {
        return vec![];
    }
    let parent = parents(rows);
    let mut ids = vec![];
    let mut p = parent[at];
    while let Some(place) = p {
        ids.insert(0, rows[place].id.clone());
        p = parent[place];
    }
    ids
}
