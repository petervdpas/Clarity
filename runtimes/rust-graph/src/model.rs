//! A graph of the data model (docs/GRAPH-PLAN.md, step 2): the records of
//! several lists, each of its own entity, as nodes whose kind is their
//! entity; each reference field of a record to an entity shown is an edge
//! to that record's node, labelled with the field. A node is named by its
//! entity and id, so records of two entities never share a node and a
//! record in two lists is one node.

use serde_json::Value;

use crate::read::{field, key_of, said, Edge, Graph, Node};

/// What the graph knows of one list from the data model: its entity, the
/// field its records show as their label, and its references to other entities.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModelList {
    pub entity: String,
    pub label: Option<String>,
    /// Each reference field and the entity it refers to.
    pub refs: Vec<(String, String)>,
}

/// The graph of the records of `lists` (each with what the model says of
/// it), at most `limit` nodes, the first lists' records first.
pub fn read(lists: &[(ModelList, Vec<Value>)], limit: usize) -> Graph {
    let mut graph = Graph { count: lists.iter().map(|(_, rows)| rows.len()).sum(), ..Graph::default() };
    let mut keys: Vec<((String, String), usize)> = Vec::new();
    'lists: for (list, (model, rows)) in lists.iter().enumerate() {
        for (row, record) in rows.iter().enumerate() {
            let Some(id) = field(record, "id").and_then(key_of) else { continue };
            let key = (model.entity.clone(), id);
            if keys.iter().any(|(k, _)| *k == key) {
                // A record in two lists is one node; it counts once.
                graph.count -= 1;
                continue;
            }
            if graph.nodes.len() >= limit {
                break 'lists;
            }
            let label = model.label.as_deref().and_then(|f| field(record, f)).map(said).filter(|l| !l.is_empty());
            let label = label.unwrap_or_else(|| format!("{} {}", model.entity, field(record, "id").map(said).unwrap_or_default()));
            keys.push((key, graph.nodes.len()));
            graph.nodes.push(Node { list, row, label, kind: model.entity.clone() });
        }
    }
    let node_of = |entity: &str, value: Option<&Value>| {
        let id = value.and_then(key_of)?;
        keys.iter().find(|((e, k), _)| e == entity && *k == id).map(|(_, n)| *n)
    };
    for from in 0..graph.nodes.len() {
        let (model, rows) = &lists[graph.nodes[from].list];
        let record = &rows[graph.nodes[from].row];
        for (name, to_entity) in &model.refs {
            if let Some(to) = node_of(to_entity, field(record, name)) {
                graph.edges.push(Edge { row: None, from, to, label: name.clone(), kind: String::new() });
            }
        }
    }
    graph
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn shop() -> Vec<(ModelList, Vec<Value>)> {
        let customer = ModelList { entity: "Customer".into(), label: Some("name".into()), refs: vec![] };
        let order = ModelList { entity: "Order".into(), label: Some("number".into()), refs: vec![("customer".into(), "Customer".into()), ("supplier".into(), "Supplier".into())] };
        vec![
            (customer, vec![json!({"id": 1, "name": "Ada"}), json!({"id": 2, "name": ""})]),
            (order, vec![json!({"id": 1, "number": "O-1", "customer": 1, "supplier": 4}), json!({"id": 2, "number": "O-2", "customer": {"id": 2}})]),
        ]
    }

    #[test]
    fn records_of_each_entity_are_nodes_of_its_kind_and_references_are_edges() {
        let graph = read(&shop(), 100);
        let nodes: Vec<(&str, &str)> = graph.nodes.iter().map(|n| (n.kind.as_str(), n.label.as_str())).collect();
        assert_eq!(nodes, [("Customer", "Ada"), ("Customer", "Customer 2"), ("Order", "O-1"), ("Order", "O-2")], "an empty label shows the entity and id");
        let edges: Vec<(usize, usize, &str)> = graph.edges.iter().map(|e| (e.from, e.to, e.label.as_str())).collect();
        assert_eq!(edges, [(2, 0, "customer"), (3, 1, "customer")], "a reference to an entity not shown is no edge; order 1 and customer 1 are two nodes");
        assert!(graph.edges.iter().all(|e| e.row.is_none()));
    }

    #[test]
    fn a_record_in_two_lists_is_one_node_and_the_limit_takes_the_first() {
        let mut lists = shop();
        lists.push((lists[0].0.clone(), vec![json!({"id": 1, "name": "Ada again"})]));
        let graph = read(&lists, 100);
        assert_eq!((graph.nodes.len(), graph.count), (4, 4));
        let cut = read(&shop(), 3);
        assert_eq!((cut.nodes.len(), cut.count, cut.edges.len()), (3, 4, 1));
    }
}
