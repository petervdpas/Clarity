//! A graph read from its two lists (docs/GRAPH-PLAN.md): each record of
//! the nodes' list a node, each record of the edges' list an edge between
//! the nodes whose keys its ends hold. The part names the fields that play
//! a role; every other field is the record's own, shown when pointed at.
//! A key is matched as a value (`1` and `1.0` are one key); a reference
//! that comes as a record is matched by its `id`.

use serde_json::Value;

/// The value of `field` of a record: its key as written, else in any case
/// (`finishedBy` for `finished_by` too), as Clarity.UI reads a record's fields.
pub fn field<'a>(record: &'a Value, field: &str) -> Option<&'a Value> {
    let fields = record.as_object()?;
    if let Some(value) = fields.get(field) {
        return Some(value);
    }
    let plain = |s: &str| s.replace('_', "").to_lowercase();
    let wanted = plain(field);
    fields.iter().find(|(k, _)| plain(k) == wanted).map(|(_, v)| v)
}

/// The fields that play a role in a graph's records.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Roles {
    pub key: String,
    pub label: Option<String>,
    pub kind: Option<String>,
    pub from: String,
    pub to: String,
    pub edge_label: Option<String>,
    pub edge_kind: Option<String>,
}

impl Default for Roles {
    fn default() -> Roles {
        Roles { key: "id".into(), label: None, kind: None, from: "source".into(), to: "target".into(), edge_label: None, edge_kind: None }
    }
}

/// A node: the list it comes from (0 for a graph of one list), its place
/// in that list, its label and its kind.
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub list: usize,
    pub row: usize,
    pub label: String,
    pub kind: String,
}

/// An edge: its place in the edges' list (none for a reference of the
/// data model, which has no record of its own), the nodes it joins (their
/// places in `Graph::nodes`), its label and kind.
#[derive(Debug, Clone, PartialEq)]
pub struct Edge {
    pub row: Option<usize>,
    pub from: usize,
    pub to: usize,
    pub label: String,
    pub kind: String,
}

/// What a graph shows, and how many nodes its list holds when it shows fewer.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Graph {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub count: usize,
}

impl Graph {
    pub fn cut(&self) -> bool {
        self.count > self.nodes.len()
    }

    /// The node standing for the record at `row` of the nodes' list.
    pub fn node_of_row(&self, row: usize) -> Option<usize> {
        self.node_at(0, row)
    }

    /// The node standing for the record at `row` of list `list`.
    pub fn node_at(&self, list: usize, row: usize) -> Option<usize> {
        self.nodes.iter().position(|n| n.list == list && n.row == row)
    }

    /// The kinds of the nodes in the order they first come.
    pub fn kinds(&self) -> Vec<String> {
        let mut kinds: Vec<String> = Vec::new();
        for node in &self.nodes {
            if !kinds.contains(&node.kind) {
                kinds.push(node.kind.clone());
            }
        }
        kinds
    }
}

/// A key as it is matched: a number by its value, text as it is, a record by its `id`; none for nothing.
pub fn key_of(value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::String(s) if s.is_empty() => None,
        Value::String(s) => Some(format!("t:{s}")),
        Value::Number(n) => n.as_f64().map(|f| format!("n:{f}")),
        Value::Bool(b) => Some(format!("b:{b}")),
        Value::Object(record) => record.get("id").and_then(key_of),
        Value::Array(_) => None,
    }
}

/// A value as a graph says it: text as it is, a number or yes and no as
/// written, a record by its label fields, a list as its items.
pub fn said(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Array(items) => items.iter().map(said).filter(|s| !s.is_empty()).collect::<Vec<_>>().join(", "),
        Value::Object(record) => ["name", "title", "label", "id"]
            .iter()
            .find_map(|f| record.get(*f).filter(|v| !v.is_null()).map(said))
            .unwrap_or_else(|| value.to_string()),
    }
}

/// The graph of `nodes` and `edges` with the fields of `roles`, at most
/// `limit` nodes, the first ones of the list; an edge whose end is not
/// shown, or holds no key, is left out.
pub fn read(nodes: &[Value], edges: &[Value], roles: &Roles, limit: usize) -> Graph {
    let mut graph = Graph { count: nodes.len(), ..Graph::default() };
    let mut keys: Vec<(String, usize)> = Vec::new();
    for (row, record) in nodes.iter().enumerate().take(limit) {
        let key = field(record, &roles.key).and_then(key_of);
        let label = roles.label.as_deref().and_then(|f| field(record, f)).map(said).filter(|l| !l.is_empty());
        let label = label.or_else(|| field(record, &roles.key).map(said)).unwrap_or_default();
        let kind = roles.kind.as_deref().and_then(|f| field(record, f)).map(said).unwrap_or_default();
        if let Some(key) = key.filter(|k| !keys.iter().any(|(seen, _)| seen == k)) {
            keys.push((key, graph.nodes.len()));
        }
        graph.nodes.push(Node { list: 0, row, label, kind });
    }
    let node_of = |value: Option<&Value>| value.and_then(key_of).and_then(|k| keys.iter().find(|(seen, _)| *seen == k).map(|(_, n)| *n));
    for (row, record) in edges.iter().enumerate() {
        let (Some(from), Some(to)) = (node_of(field(record, &roles.from)), node_of(field(record, &roles.to))) else { continue };
        let label = roles.edge_label.as_deref().and_then(|f| field(record, f)).map(said).unwrap_or_default();
        let kind = roles.edge_kind.as_deref().and_then(|f| field(record, f)).map(said).unwrap_or_default();
        graph.edges.push(Edge { row: Some(row), from, to, label, kind });
    }
    graph
}

/// The fields of a record a graph tells when it is pointed at: the ones
/// named (`show`), else every field that plays no role, with its value.
pub fn told(record: &Value, show: &[String], roles: &[&str]) -> Vec<(String, String)> {
    let Some(fields) = record.as_object() else { return Vec::new() };
    if show.is_empty() {
        return fields.iter().filter(|(name, _)| !roles.contains(&name.as_str())).map(|(name, value)| (name.clone(), said(value))).filter(|(_, v)| !v.is_empty()).collect();
    }
    show.iter().map(|name| (name.clone(), field(record, name).map(said).unwrap_or_default())).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn roles() -> Roles {
        Roles { label: Some("name".into()), kind: Some("kind".into()), edge_label: Some("field".into()), ..Roles::default() }
    }

    #[test]
    fn records_become_nodes_and_edges_join_them_by_key() {
        let nodes = [json!({"id": 1, "name": "Ada", "kind": "person"}), json!({"id": 2, "name": "Shop", "kind": "place"}), json!({"id": 3, "name": "", "kind": "person"})];
        let edges = [json!({"source": 1, "target": 2.0, "field": "works at"}), json!({"source": 2, "target": 9}), json!({"source": null, "target": 1})];
        let graph = read(&nodes, &edges, &roles(), 100);
        assert_eq!(graph.nodes.iter().map(|n| (n.label.as_str(), n.kind.as_str())).collect::<Vec<_>>(), [("Ada", "person"), ("Shop", "place"), ("3", "person")]);
        assert_eq!(graph.edges, [Edge { row: Some(0), from: 0, to: 1, label: "works at".into(), kind: String::new() }], "an end that is not there leaves the edge out");
        assert!(!graph.cut());
    }

    #[test]
    fn an_end_that_comes_as_a_record_is_matched_by_its_id() {
        let nodes = [json!({"id": "a"}), json!({"id": "b"})];
        let edges = [json!({"source": {"id": "a", "name": "A"}, "target": "b"})];
        assert_eq!(read(&nodes, &edges, &Roles::default(), 100).edges.len(), 1);
    }

    #[test]
    fn a_graph_past_its_limit_shows_the_first_nodes_and_their_edges() {
        let nodes: Vec<Value> = (0..5).map(|i| json!({"id": i})).collect();
        let edges = [json!({"source": 0, "target": 1}), json!({"source": 1, "target": 4})];
        let graph = read(&nodes, &edges, &Roles::default(), 3);
        assert_eq!((graph.nodes.len(), graph.count, graph.edges.len()), (3, 5, 1));
        assert!(graph.cut());
    }

    #[test]
    fn a_record_tells_its_own_fields_or_the_ones_named() {
        let record = json!({"id": 1, "name": "Ada", "kind": "person", "owner": {"id": 7, "name": "Bob"}, "tags": ["a", "b"], "empty": null});
        assert_eq!(told(&record, &[], &["id", "name", "kind"]), [("owner".to_string(), "Bob".to_string()), ("tags".to_string(), "a, b".to_string())]);
        assert_eq!(told(&record, &["kind".into(), "missing".into()], &[]), [("kind".to_string(), "person".to_string()), ("missing".to_string(), String::new())]);
    }
}
