//! The accessibility tree for the browser's page (DESIGN 7.9, A1 phase 2):
//! the page keeps a hidden DOM tree with ARIA beside the canvas, one element
//! per node, and this says what to change in it: the whole tree the first
//! time, then only the nodes new or changed and the ids gone, as
//! `kit::update` does for AccessKit.
//!
//! Nodes go as `AccessNode::json` gives them, ids as text. What a person is to hear without moving there (a
//! message, a status that changed) goes in `say`, for the page's live region.

use std::collections::HashMap;

use serde_json::{Value, json};

use super::{AccessNode, AccessTree, Role};


/// What the page changes in its mirror: none when nothing did. `{ whole,
/// root, focus, nodes, gone, say }`: `whole` true when the page drops what
/// it had (no tree before); `nodes` new or changed, in tree order; `gone`
/// the ids no longer in the tree; `say` [{text, alert}] for the live
/// region: an alert dialog that opened (alert), a status that came or
/// changed. Nothing is said for the whole tree.
pub fn update(before: Option<&AccessTree>, now: &AccessTree) -> Option<Value> {
    let old: HashMap<u64, &AccessNode> = before.map(|b| b.nodes.iter().map(|n| (n.id, n)).collect()).unwrap_or_default();
    let mut nodes = Vec::new();
    let mut say = Vec::new();
    for n in &now.nodes {
        let was = old.get(&n.id).copied();
        if was == Some(n) {
            continue;
        }
        nodes.push(n.json());
        if before.is_none() {
            continue;
        }
        let renamed = was.is_none_or(|w| w.name != n.name || w.role != n.role);
        let text = match (&n.description, n.name.is_empty()) {
            (Some(d), false) if n.role == Role::AlertDialog => format!("{}. {d}", n.name),
            _ => n.name.clone(),
        };
        if renamed && !text.is_empty() {
            match n.role {
                Role::AlertDialog => say.push(json!({ "text": text, "alert": true })),
                Role::Status => say.push(json!({ "text": text, "alert": false })),
                _ => {}
            }
        }
    }
    let kept: std::collections::HashSet<u64> = now.nodes.iter().map(|n| n.id).collect();
    let gone: Vec<String> = before.map(|b| b.nodes.iter().filter(|n| !kept.contains(&n.id)).map(|n| n.id.to_string()).collect()).unwrap_or_default();
    let focus_moved = before.is_none_or(|b| b.focus != now.focus);
    if nodes.is_empty() && gone.is_empty() && !focus_moved {
        return None;
    }
    Some(json!({
        "whole": before.is_none(),
        "root": super::ROOT.to_string(),
        "focus": now.focus.to_string(),
        "nodes": nodes,
        "gone": gone,
        "say": say,
    }))
}
