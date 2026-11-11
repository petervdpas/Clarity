//! Writing, the engine's half that is the same for every format: a value in
//! Clarity's one form (normal.rs) laid into a new document where a mapping
//! says, which the format then writes.
//!
//! By the nodes a format has (`Nodes`):
//! - values (JSON): the value whole, as Clarity's one form gives it, inside
//!   the members `at` names; a mapped field moved to its path;
//! - elements (XML): `at` always, naming the elements around the records and
//!   the record's own; each field of the entity in its order as a child
//!   element of its name (or at its path: `@name` an attribute, `.` the
//!   record's text); null, the id and `other` fields left out; a list of
//!   values as repeated elements; a list of records inside an element of
//!   the field's name, one element per record named after its entity in
//!   camelCase, or at the `at` of its `nested` mapping;
//! - rows (CSV): a row per record of the entity's fields that are one
//!   value, in order, a list of values as its JSON text.

use crate::doc::{Content, Document, Form, Node};
use crate::kind::{Kind, Shape};
use crate::mapping::Mapping;
use crate::path::{Path, Step};
use crate::shape::column;
use crate::tree::{self, Tree};
use crate::{FormatInfo, Nodes, Problem, ProblemKind};

fn bad(text: String) -> Problem {
    Problem::new(ProblemKind::BadOptions, text)
}

/// `value`, of `kind`, as a new document laid out by `map`.
pub fn write(info: &FormatInfo, map: &Mapping, value: &Tree, kind: &Kind) -> Result<Document, Problem> {
    let top = match info.nodes {
        Nodes::Values => values(map, value, kind),
        Nodes::Elements => elements(info, map, value, kind)?,
        Nodes::Rows => {
            let (shape, records) = match (kind, value) {
                (Kind::Records(shape), Tree::List(items)) => (shape, items.clone()),
                (Kind::Record(shape), Tree::Object(_)) => (shape, vec![value.clone()]),
                _ => return Err(Problem::new(ProblemKind::Failed, format!("{} writes a record or a list of records", info.name))),
            };
            let flat: Vec<_> = shape.fields.iter().filter(|f| !matches!(f.kind, Kind::Other | Kind::Record(_) | Kind::Records(_))).collect();
            let columns: Vec<String> = flat.iter().map(|f| column(f, map)).collect();
            let mut top = Node::new("", Form::List);
            for record in &records {
                let mut row = Node::new("", Form::Object);
                for (field, name) in flat.iter().zip(&columns) {
                    let cell = match record.get(&field.name).cloned().unwrap_or(Tree::Null) {
                        list @ (Tree::List(_) | Tree::Object(_)) => Tree::Text(tree::write(&list)),
                        other => other,
                    };
                    row.push(Node::value(name, cell));
                }
                top.push(row);
            }
            return Ok(Document { top, columns, kept: None });
        }
    };
    Ok(Document { top, columns: Vec::new(), kept: None })
}

/// The names of a path written into, `.` left out; none when it goes up.
fn steps(path: &Path) -> Option<Vec<&Step>> {
    if path.steps.contains(&Step::Up) {
        return None;
    }
    Some(path.steps.iter().filter(|s| **s != Step::Here).collect())
}

fn values(map: &Mapping, value: &Tree, kind: &Kind) -> Node {
    let mut node = Node::from_tree("", value);
    match (kind, &node.form) {
        (Kind::Record(shape), Form::Object) => move_fields(&mut node, shape, map),
        (Kind::Records(shape), Form::List) => node.nodes_mut().for_each(|i| move_fields(i, shape, map)),
        _ => {}
    }
    let Some(at) = map.at.as_ref().filter(|a| !a.steps.is_empty()) else { return node };
    let names = at.names();
    let mut node = node.renamed(names[names.len() - 1]);
    for step in names[..names.len() - 1].iter().rev() {
        let mut outer = Node::new(step, Form::Object);
        outer.push(node);
        node = outer;
    }
    let mut top = Node::new("", Form::Object);
    top.push(node);
    top
}

/// The object's member called `name`, taken out, with the place it had.
fn take(node: &mut Node, name: &str) -> Option<(usize, Node)> {
    let at = node.children.iter().position(|c| matches!(c, Content::Node(n) if n.name == name))?;
    match node.children.remove(at) {
        Content::Node(n) => Some((at, n)),
        _ => None,
    }
}

/// The member at `names`, objects made where missing (the first in the
/// place `at`); what was there replaced.
fn put(node: &mut Node, names: &[&str], value: Node, at: usize) {
    let (last, before) = names.split_last().expect("a path has a step");
    let mut here = node;
    let mut place = Some(at);
    for step in before {
        let found = here.children.iter().position(|c| matches!(c, Content::Node(n) if n.name == *step));
        let index = match found {
            Some(index) => index,
            None => {
                let index = place.unwrap_or(here.children.len()).min(here.children.len());
                here.children.insert(index, Content::Node(Node::new(step, Form::Object)));
                index
            }
        };
        place = None;
        let Content::Node(next) = &mut here.children[index] else { unreachable!("a node's place") };
        here = next;
    }
    let value = value.renamed(last);
    let found = here.children.iter().position(|c| matches!(c, Content::Node(n) if n.name == *last));
    match found {
        Some(index) => here.children[index] = Content::Node(value),
        None => {
            let index = place.unwrap_or(here.children.len()).min(here.children.len());
            here.children.insert(index, Content::Node(value));
        }
    }
}

/// A record's mapped fields moved to their paths, records inside by their own mappings.
fn move_fields(node: &mut Node, shape: &Shape, map: &Mapping) {
    for field in &shape.fields {
        let inner = map.inner(&field.name);
        if let Some(child) = node.nodes_mut().find(|n| n.name == field.name) {
            match (&field.kind, &child.form) {
                (Kind::Record(s), Form::Object) => move_fields(child, s, &inner),
                (Kind::Records(s), Form::List) => child.nodes_mut().for_each(|i| move_fields(i, s, &inner)),
                _ => {}
            }
        }
        let to = map.nested(&field.name).and_then(|n| n.at.as_ref()).or_else(|| map.path(&field.name));
        let Some(names) = to.and_then(steps).map(|s| s.iter().map(|s| if let Step::Name(n) = s { n.as_str() } else { "" }).collect::<Vec<_>>()) else { continue };
        if names.is_empty() || names == [field.name.as_str()] {
            continue;
        }
        if let Some((at, child)) = take(node, &field.name) {
            put(node, &names, child, at);
        }
    }
}

/// One value as an element's text; none for null.
fn value_text(value: &Tree) -> Option<String> {
    match value {
        Tree::Null => None,
        Tree::Bool(b) => Some(b.to_string()),
        Tree::Number(n) | Tree::Text(n) => Some(n.clone()),
        other => Some(tree::write(other)),
    }
}

fn text_element(name: &str, text: &str) -> Node {
    let mut e = Node::element(name);
    e.set_text(text);
    e
}

/// A name as an element's: `Line` is `line`.
fn camel(name: &str) -> String {
    let mut chars = name.chars();
    chars.next().map(|c| c.to_lowercase().chain(chars).collect()).unwrap_or_default()
}

fn elements(info: &FormatInfo, map: &Mapping, value: &Tree, kind: &Kind) -> Result<Node, Problem> {
    let name = info.name;
    let at = map.at.as_ref().filter(|a| !a.steps.is_empty()).ok_or_else(|| bad(format!("{name} of needs 'at', the elements around the records and the record's own: at \"orders/order\"")))?;
    let names = at.names();
    if let Some(step) = names.iter().find(|s| s.is_empty()) {
        return Err(bad(format!("'{step}' in at \"{}\" is no {} name", at.shown(names.len()), info.title)));
    }
    let many = matches!(kind, Kind::Records(_) | Kind::Values(_));
    if many && names.len() < 2 {
        return Err(bad(format!("{name} of a list needs the element around its items and the item's own: at \"orders/order\"")));
    }
    let last = names[names.len() - 1];
    let mut inner: Vec<Node> = match (kind, value) {
        (Kind::Records(shape), Tree::List(items)) => items
            .iter()
            .map(|item| {
                let mut e = Node::element(last);
                fill(&mut e, item, shape, map);
                e
            })
            .collect(),
        (Kind::Values(_), Tree::List(items)) => items.iter().filter_map(value_text).map(|t| text_element(last, &t)).collect(),
        (Kind::Record(shape), Tree::Object(_)) => {
            let mut e = Node::element(last);
            fill(&mut e, value, shape, map);
            vec![e]
        }
        (Kind::Record(_), _) => vec![Node::element(last)],
        (Kind::Records(_) | Kind::Values(_), _) => Vec::new(),
        (_, value) => vec![text_element(last, &value_text(value).unwrap_or_default())],
    };
    for step in names[..names.len() - 1].iter().rev() {
        let mut e = Node::element(step);
        inner.into_iter().for_each(|n| e.push(n));
        inner = vec![e];
    }
    let mut top = Node::element("");
    inner.into_iter().for_each(|n| top.push(n));
    Ok(top)
}

/// The element at `names` inside `e`, elements made where missing.
fn ensure<'a>(e: &'a mut Node, names: &[&Step]) -> &'a mut Node {
    let mut here = e;
    for step in names {
        let Step::Name(n) = step else { continue };
        let found = here.nodes().position(|c| c.name == *n);
        let at = match found {
            Some(at) => at,
            None => {
                here.push(Node::element(n));
                here.nodes().count() - 1
            }
        };
        here = here.nodes_mut().nth(at).expect("just found");
    }
    here
}

/// A new element at `path` inside `e`, the elements before it made where missing.
fn make<'a>(e: &'a mut Node, path: &[&Step], name: &str) -> &'a mut Node {
    let holder = ensure(e, path);
    holder.push(Node::element(name));
    holder.nodes_mut().last().expect("just pushed")
}

/// A record's fields as the children of `e`, in the entity's order.
fn fill(e: &mut Node, value: &Tree, shape: &Shape, map: &Mapping) {
    for field in &shape.fields {
        let Some(given) = value.get(&field.name).filter(|v| **v != Tree::Null) else { continue };
        let inner = map.inner(&field.name);
        let own = Path::name(&field.name);
        let Some(path) = steps(map.path(&field.name).unwrap_or(&own)) else { continue };
        let Some((last, before)) = path.split_last() else {
            // `.`: the record's own text.
            if let Some(text) = value_text(given).filter(|_| !matches!(field.kind, Kind::Record(_) | Kind::Records(_) | Kind::Values(_))) {
                e.set_text(&text);
            }
            continue;
        };
        let last_name = match last {
            Step::Name(n) => n.as_str(),
            Step::Attribute(a) => {
                if let Some(text) = value_text(given).filter(|_| !matches!(field.kind, Kind::Other | Kind::Record(_) | Kind::Records(_) | Kind::Values(_))) {
                    ensure(e, before).set_attribute(a, &text);
                }
                continue;
            }
            _ => continue,
        };
        match (&field.kind, given) {
            (Kind::Other, _) => {}
            (Kind::Record(nested), Tree::Object(_)) => fill(make(e, before, last_name), given, nested, &inner),
            (Kind::Records(nested), Tree::List(items)) => {
                let (holder, item) = match map.nested(&field.name).and_then(|n| n.at.as_ref()).and_then(steps) {
                    Some(at) if !at.is_empty() => {
                        let (item, around) = at.split_last().expect("not empty");
                        let Step::Name(item) = item else { continue };
                        (ensure(e, around), item.clone())
                    }
                    _ => (make(e, before, last_name), camel(&nested.entity)),
                };
                for value in items {
                    let mut child = Node::element(&item);
                    fill(&mut child, value, nested, &inner);
                    holder.push(child);
                }
            }
            (Kind::Values(_), Tree::List(items)) => {
                let holder = ensure(e, before);
                for text in items.iter().filter_map(value_text) {
                    holder.push(text_element(last_name, &text));
                }
            }
            (_, given) => {
                if let Some(text) = value_text(given) {
                    make(e, before, last_name).set_text(&text);
                }
            }
        }
    }
}
