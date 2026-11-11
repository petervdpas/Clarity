//! Reading, the engine's half that is the same for every format: the nodes
//! a mapping names in a document, shaped by what the program wants into the
//! tree the fit reads (fit.rs). A node that says what it is (an object, a
//! list, a value: JSON, a CSV row) is taken as it is, so the fit says where
//! it does not fit; an element (XML) is shaped by the entity, since XML has
//! no lists or types of its own.
//!
//! Where the records are: at `at`, its last step naming them (every node of
//! that name in the node the steps before lead to; a list node of that name
//! is the list). Nothing of that name is no records in an element, which
//! repeats names; in an object, or at the top, the path is not there.
//! Without `at`, the format's own place (`Nodes`): the whole file (JSON),
//! the root element's children and the root as one record (XML), the rows
//! and the one row as one record (CSV).
//!
//! An element field takes a child element of its name, else an attribute
//! of its name; a record field the element of its name; a list field the
//! elements of its name, or the items inside one element of its name whose
//! children all have one name that is no field of the item's entity; one
//! empty element is an empty list. A field the mapping nests (`nested` with
//! its own `at`) takes the nodes there, each a record.

use crate::doc::{local, Document, Form, Node};
use crate::fit::plain;
use crate::kind::{Field, Kind, Shape};
use crate::mapping::Mapping;
use crate::path::{self, Path, Target};
use crate::tree::Tree;
use crate::{FormatInfo, Nodes, Problem, ProblemKind};

/// The nodes where the records are, and the records they were found in.
pub struct Found<'a> {
    pub nodes: Vec<&'a Node>,
    pub up: Vec<&'a Node>,
}

/// Where the records (`many`) or the record are in `doc`: at `at`, else at the format's own place.
pub fn found<'a>(doc: &'a Document, info: &FormatInfo, at: Option<&Path>, many: bool) -> Result<Found<'a>, Problem> {
    let Some(at) = at.filter(|a| !a.steps.is_empty()) else {
        return Ok(match info.nodes {
            Nodes::Values => Found { nodes: vec![&doc.top], up: Vec::new() },
            Nodes::Elements => match doc.top.nodes().next() {
                Some(root) if many => Found { nodes: root.nodes().collect(), up: vec![root] },
                Some(root) => Found { nodes: vec![root], up: Vec::new() },
                None => Found { nodes: Vec::new(), up: Vec::new() },
            },
            Nodes::Rows => {
                let rows: Vec<&Node> = doc.top.nodes().collect();
                match rows.as_slice() {
                    [one] if !many => Found { nodes: vec![*one], up: vec![&doc.top] },
                    _ => Found { nodes: vec![&doc.top], up: Vec::new() },
                }
            }
        });
    };
    let names = at.names();
    let has_no = |n: usize| Problem::new(ProblemKind::NotFitting, format!("the {} has no '{}'", info.title, at.shown(n)));
    let mut here = &doc.top;
    for (i, step) in names[..names.len() - 1].iter().enumerate() {
        here = path::through(here, step).ok_or_else(|| has_no(i + 1))?;
    }
    let nodes = path::named(here, names[names.len() - 1], false);
    if nodes.is_empty() && (std::ptr::eq(here, &doc.top) || here.form != Form::Element || !many) {
        return Err(has_no(names.len()));
    }
    Ok(Found { nodes, up: vec![here] })
}

/// What `doc` holds where `map` says, shaped as `kind` wants, for the fit.
pub fn read(doc: &Document, info: &FormatInfo, map: &Mapping, kind: &Kind) -> Result<Tree, Problem> {
    let many = matches!(kind, Kind::Records(_) | Kind::Values(_));
    let found = found(doc, info, map.at.as_ref(), many)?;
    let Some(first) = found.nodes.first() else { return Ok(Tree::List(Vec::new())) };
    Ok(match kind {
        Kind::Records(shape) => records(&found.nodes, &found.up, shape, map),
        Kind::Values(_) => match found.nodes.as_slice() {
            [one] if one.form == Form::List => Tree::List(one.nodes().map(Node::to_tree).collect()),
            [one] if one.form != Form::Element => one.to_tree(),
            nodes => Tree::List(nodes.iter().map(|n| n.to_tree()).collect()),
        },
        Kind::Record(shape) => one_record(first, &found.up, shape, map),
        _ => first.to_tree(),
    })
}

fn is_record(node: &Node) -> bool {
    matches!(node.form, Form::Element | Form::Object)
}

/// One record: an element or object shaped, else as it is (which the fit refuses).
fn one_record(node: &Node, up: &[&Node], shape: &Shape, map: &Mapping) -> Tree {
    if is_record(node) { record(node, up, shape, map) } else { node.to_tree() }
}

/// The records `nodes` are: one list node's items, elements each a record; anything else as it is.
fn records(nodes: &[&Node], up: &[&Node], shape: &Shape, map: &Mapping) -> Tree {
    match nodes {
        [one] if one.form == Form::List => Tree::List(one.nodes().map(|i| one_record(i, up, shape, map)).collect()),
        [one] if one.form != Form::Element => one.to_tree(),
        nodes => Tree::List(nodes.iter().map(|n| one_record(n, up, shape, map)).collect()),
    }
}

/// A record node as the object the fit reads: each field found, what is not found left out.
pub fn record(node: &Node, up: &[&Node], shape: &Shape, map: &Mapping) -> Tree {
    Tree::Object(shape.fields.iter().filter_map(|f| field(node, up, f, map).map(|v| (f.name.clone(), v))).collect())
}

fn is_plain(kind: &Kind) -> bool {
    !matches!(kind, Kind::Record(_) | Kind::Records(_) | Kind::Values(_) | Kind::Other)
}

fn field(node: &Node, up: &[&Node], field: &Field, map: &Mapping) -> Option<Tree> {
    if field.kind == Kind::Other {
        return None;
    }
    let inner = map.inner(&field.name);
    if let Some(at) = map.nested(&field.name).and_then(|n| n.at.as_ref()) {
        // Records inside at their own path: the nodes there, each a record.
        // `..` from them is this record, not what the path went through.
        if let (Kind::Records(shape), Target::Nodes(found, _)) = (&field.kind, path::resolve(node, up, at)) {
            let mut around = up.to_vec();
            around.push(node);
            return Some(records(&found, &around, shape, &inner));
        }
    }
    let path = map.path(&field.name).cloned().unwrap_or_else(|| Path::name(&field.name));
    shape_target(path::resolve(node, up, &path), &field.kind, &inner)
}

fn shape_target(target: Target, kind: &Kind, map: &Mapping) -> Option<Tree> {
    let (found, chain) = match target {
        Target::Missing => return None,
        Target::Text(t) => return (is_plain(kind) && !t.is_empty()).then_some(Tree::Text(t)),
        Target::Own(node) => {
            return match &node.form {
                _ if !is_plain(kind) => None,
                Form::Element => plain_text(node).map(Tree::Text),
                Form::Value(v) => Some(v.clone()),
                _ => None,
            };
        }
        Target::Nodes(found, chain) => (found, chain),
    };
    let first = found[0];
    if first.form != Form::Element {
        return Some(match kind {
            Kind::Record(shape) => one_record(first, &chain, shape, map),
            Kind::Records(shape) => records(&found[..1], &chain, shape, map),
            Kind::Values(_) if first.form == Form::List => Tree::List(first.nodes().map(Node::to_tree).collect()),
            _ => first.to_tree(),
        });
    }
    match kind {
        Kind::Record(shape) => Some(record(first, &chain, shape, map)),
        Kind::Records(shape) => {
            let items: Vec<&Node> = match wrapped(&found, Some(shape)) {
                Some(items) => items,
                None if found.len() == 1 && is_empty(first) => Vec::new(),
                None => found,
            };
            Some(Tree::List(items.iter().map(|i| record(i, &chain, shape, map)).collect()))
        }
        Kind::Values(_) => {
            let items: Vec<&Node> = match wrapped(&found, None) {
                Some(items) => items,
                None if found.len() == 1 && is_empty(first) => Vec::new(),
                None => found,
            };
            Some(Tree::List(items.iter().map(|i| Tree::Text(i.text())).collect()))
        }
        _ => plain_text(first).map(Tree::Text),
    }
}

/// An element's text as one value: none when empty, or only space between elements.
fn plain_text(e: &Node) -> Option<String> {
    let text = e.text();
    if text.is_empty() || (e.has_nodes() && text.trim().is_empty()) {
        return None;
    }
    Some(text)
}

/// Whether an element holds nothing: no elements, no attributes, no text but space.
fn is_empty(e: &Node) -> bool {
    !e.has_nodes() && e.attributes.iter().all(|(n, _)| path::is_namespace(n)) && e.text().trim().is_empty()
}

/// The children of a lone element when they all have one name that is no
/// field of the items (`fields`): a wrapper of items.
fn wrapped<'a>(found: &[&'a Node], fields: Option<&Shape>) -> Option<Vec<&'a Node>> {
    let [one] = found else { return None };
    let first = one.nodes().next()?;
    let name = local(&first.name);
    if !one.nodes().all(|c| local(&c.name) == name) {
        return None;
    }
    if fields.is_some_and(|s| s.fields.iter().any(|f| plain(&f.name) == plain(name))) {
        return None;
    }
    Some(one.nodes().collect())
}

/// The names a flat file without a header row gives its columns: the
/// entity's fields that are one value, in order, by their mapped names.
pub fn names(kind: &Kind, map: &Mapping) -> Vec<String> {
    match kind {
        Kind::Record(shape) | Kind::Records(shape) => shape.fields.iter().filter(|f| is_plain(&f.kind) || matches!(f.kind, Kind::Values(_))).map(|f| column(f, map)).collect(),
        _ => Vec::new(),
    }
}

/// The name a field has in a flat file: its mapped path, else its own.
pub fn column(field: &Field, map: &Mapping) -> String {
    map.path(&field.name).and_then(Path::single).unwrap_or(&field.name).to_string()
}
