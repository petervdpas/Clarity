//! Paths, one style for every format (docs/FORMATS-PLAN.md, step 4): names
//! with slashes between (`shop/orders/order`), `@name` an attribute where a
//! format has them, `.` a record's own text, `..` the record it was found
//! in (for the records at `at`, the node that holds them; inside a path,
//! the node before the last step). An `at` without a slash is read with dots between too (`data.orders`,
//! as JSON's `at` was released); a name holding a slash cannot be named. In
//! a flat format (CSV) a path is a column's name, written whole.
//!
//! Names match as the engine matches them everywhere: a step is the name,
//! or the name without its prefix (`o:order` is `order`). A path of one
//! name finds a field as a field's name does: the child nodes of that name
//! (else of that name ignoring case, underscores, hyphens and spaces), else
//! an attribute of it the same way; the steps of a longer path are exact.
//! A step through a list goes into its first item.

use crate::doc::{is_named, local, Form, Node};
use crate::fit::plain;
use crate::Nodes;

#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    Name(String),
    Attribute(String),
    /// `.`: here; last, the node's own text.
    Here,
    /// `..`: the record it was found in.
    Up,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Path {
    pub steps: Vec<Step>,
    /// What it was written with between its steps, for messages.
    pub sep: char,
}

/// Why a text is no path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Why {
    NoPath,
    /// It names an attribute in a format without them.
    Attribute,
}

impl Path {
    /// One name.
    pub fn name(name: &str) -> Path {
        Path { steps: vec![Step::Name(name.to_string())], sep: '/' }
    }

    /// An `at`: names with slashes, a leading slash allowed; without a slash, with dots.
    pub fn at(text: &str) -> Path {
        let sep = if text.contains('/') || !text.contains('.') { '/' } else { '.' };
        let text = text.strip_prefix('/').unwrap_or(text);
        let steps = if text.is_empty() { Vec::new() } else { text.split(sep).map(|s| Step::Name(s.to_string())).collect() };
        Path { steps, sep }
    }

    /// A field's path, read as the format's `nodes` read one.
    pub fn field(text: &str, nodes: Nodes, attributes: bool) -> Result<Path, Why> {
        if nodes == Nodes::Rows {
            return Ok(Path::name(text));
        }
        if text == "." {
            return Ok(Path { steps: vec![Step::Here], sep: '/' });
        }
        let parts: Vec<&str> = text.split('/').collect();
        let mut steps = Vec::with_capacity(parts.len());
        for (i, part) in parts.iter().enumerate() {
            steps.push(match *part {
                "" => return Err(Why::NoPath),
                ".." => Step::Up,
                "." => Step::Here,
                p if p.starts_with('@') => {
                    if i + 1 != parts.len() || p.len() == 1 {
                        return Err(Why::NoPath);
                    }
                    if !attributes {
                        return Err(Why::Attribute);
                    }
                    Step::Attribute(p[1..].to_string())
                }
                p => Step::Name(p.to_string()),
            });
        }
        Ok(Path { steps, sep: '/' })
    }

    /// The one name it is, when it is one.
    pub fn single(&self) -> Option<&str> {
        match self.steps.as_slice() {
            [Step::Name(n)] => Some(n),
            _ => None,
        }
    }

    /// Its first `n` steps as written.
    pub fn shown(&self, n: usize) -> String {
        let names: Vec<&str> = self.steps[..n].iter().map(|s| if let Step::Name(n) = s { n.as_str() } else { "" }).collect();
        names.join(&self.sep.to_string())
    }

    /// The names of its steps (an `at` has only names).
    pub fn names(&self) -> Vec<&str> {
        self.steps.iter().map(|s| if let Step::Name(n) = s { n.as_str() } else { "" }).collect()
    }
}

/// The child nodes called `name`, as they are (a list as its node): by the
/// exact name, else (`plainly`) by the plain name.
pub fn named<'a>(node: &'a Node, name: &str, plainly: bool) -> Vec<&'a Node> {
    let exact: Vec<&Node> = node.nodes().filter(|n| is_named(&n.name, name)).collect();
    if !exact.is_empty() || !plainly {
        return exact;
    }
    let wanted = plain(name);
    node.nodes().filter(|n| plain(local(&n.name)) == wanted).collect()
}

/// A namespace declaration, which is no attribute of the data.
pub fn is_namespace(name: &str) -> bool {
    name == "xmlns" || name.starts_with("xmlns:")
}

/// Its attribute `name`: by the exact local name, else (`plainly`) by the plain name.
pub fn attribute<'a>(node: &'a Node, name: &str, plainly: bool) -> Option<&'a str> {
    let attributes = || node.attributes.iter().filter(|(n, _)| !is_namespace(n));
    let wanted = plain(name);
    attributes().find(|(n, _)| local(n) == name).or_else(|| attributes().find(|(n, _)| plainly && plain(local(n)) == wanted)).map(|(_, v)| v.as_str())
}

/// A step through: the first child of the name; a list its first item.
pub fn through<'a>(node: &'a Node, name: &str) -> Option<&'a Node> {
    let found = node.nodes().find(|n| is_named(&n.name, name))?;
    if found.form == Form::List { found.nodes().next() } else { Some(found) }
}

/// What a path finds from a record.
#[derive(Debug)]
pub enum Target<'a> {
    /// Nodes, as they are, and the records they were found in (nearest last).
    Nodes(Vec<&'a Node>, Vec<&'a Node>),
    /// An attribute's value.
    Text(String),
    /// The record's own text or value.
    Own(&'a Node),
    Missing,
}

/// What `path` finds from `node`, which was found in the records `up`.
pub fn resolve<'a>(node: &'a Node, up: &[&'a Node], path: &Path) -> Target<'a> {
    let within = |found: Vec<&'a Node>, at: &'a Node, mut chain: Vec<&'a Node>| {
        if found.is_empty() {
            return Target::Missing;
        }
        chain.push(at);
        Target::Nodes(found, chain)
    };
    if let Some(name) = path.single() {
        let found = named(node, name, true);
        if !found.is_empty() {
            return within(found, node, up.to_vec());
        }
        return attribute(node, name, true).map_or(Target::Missing, |v| Target::Text(v.to_string()));
    }
    let mut here = node;
    let mut chain = up.to_vec();
    let Some((last, before)) = path.steps.split_last() else { return Target::Own(node) };
    for step in before {
        match step {
            Step::Name(n) => {
                let Some(next) = through(here, n) else { return Target::Missing };
                chain.push(here);
                here = next;
            }
            Step::Here => {}
            Step::Up => match chain.pop() {
                Some(outer) => here = outer,
                None => return Target::Missing,
            },
            Step::Attribute(_) => return Target::Missing,
        }
    }
    match last {
        Step::Name(n) => within(named(here, n, false), here, chain),
        Step::Attribute(a) => attribute(here, a, false).map_or(Target::Missing, |v| Target::Text(v.to_string())),
        Step::Here => Target::Own(here),
        Step::Up => match chain.pop() {
            Some(outer) => Target::Nodes(vec![outer], chain),
            None => Target::Missing,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_read_slashes_and_dots_when_there_is_no_slash() {
        assert_eq!(Path::at("data.orders").names(), ["data", "orders"]);
        assert_eq!(Path::at("/shop/orders/order").names(), ["shop", "orders", "order"]);
        assert_eq!(Path::at("/my.root").names(), ["my.root"]);
        assert_eq!(Path::field("a/@b", Nodes::Elements, true).unwrap().steps.last(), Some(&Step::Attribute("b".into())));
        assert_eq!(Path::field("@a/b", Nodes::Elements, true), Err(Why::NoPath));
        assert_eq!(Path::field("@a", Nodes::Values, false), Err(Why::Attribute));
        assert_eq!(Path::field("a/b", Nodes::Rows, false).unwrap().single(), Some("a/b"), "a flat format's path is a column's name");
    }
}
