//! The nodes near one node, and the graph of the nodes kept: a graph shows
//! only what lies a number of edges from the picked node (`hops`), and only
//! the kinds not hidden (its legend). Edges count both ways.

use std::collections::VecDeque;

use crate::read::Graph;

/// For each node, whether it lies at most `hops` edges from `focus`, either way along them.
pub fn within(graph: &Graph, focus: usize, hops: usize) -> Vec<bool> {
    let mut away = vec![usize::MAX; graph.nodes.len()];
    if focus >= away.len() {
        return vec![true; away.len()];
    }
    let mut next = vec![Vec::new(); graph.nodes.len()];
    for e in &graph.edges {
        next[e.from].push(e.to);
        next[e.to].push(e.from);
    }
    away[focus] = 0;
    let mut queue = VecDeque::from([focus]);
    while let Some(node) = queue.pop_front() {
        if away[node] == hops {
            continue;
        }
        for &n in &next[node] {
            if away[n] == usize::MAX {
                away[n] = away[node] + 1;
                queue.push_back(n);
            }
        }
    }
    away.into_iter().map(|a| a != usize::MAX).collect()
}

/// The graph of the nodes `keep` keeps, in their order, with the edges between them; it still counts what its lists hold.
pub fn kept(graph: &Graph, keep: &[bool]) -> Graph {
    let mut place = vec![None; graph.nodes.len()];
    let mut out = Graph { count: graph.count, ..Graph::default() };
    for (n, node) in graph.nodes.iter().enumerate() {
        if keep.get(n).copied().unwrap_or(true) {
            place[n] = Some(out.nodes.len());
            out.nodes.push(node.clone());
        }
    }
    for e in &graph.edges {
        if let (Some(from), Some(to)) = (place[e.from], place[e.to]) {
            out.edges.push(crate::read::Edge { from, to, ..e.clone() });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::read::{read, Roles};
    use serde_json::json;

    fn chain() -> Graph {
        let nodes: Vec<_> = (0..5).map(|i| json!({"id": i})).collect();
        let edges = [json!({"source": 0, "target": 1}), json!({"source": 2, "target": 1}), json!({"source": 2, "target": 3})];
        read(&nodes, &edges, &Roles::default(), 100)
    }

    #[test]
    fn hops_count_edges_either_way() {
        assert_eq!(within(&chain(), 1, 1), [true, true, true, false, false]);
        assert_eq!(within(&chain(), 0, 3), [true, true, true, true, false], "4 is joined to nothing");
        assert_eq!(within(&chain(), 9, 1), [true; 5], "no such node keeps all");
    }

    #[test]
    fn the_kept_graph_has_the_edges_between_its_nodes() {
        let kept = kept(&chain(), &[false, true, true, true, false]);
        assert_eq!(kept.nodes.iter().map(|n| n.row).collect::<Vec<_>>(), [1, 2, 3]);
        assert_eq!(kept.edges.iter().map(|e| (e.from, e.to)).collect::<Vec<_>>(), [(1, 0), (1, 2)]);
        assert_eq!(kept.count, 5);
    }
}
