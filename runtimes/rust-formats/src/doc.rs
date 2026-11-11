//! The one document model every format reads into and writes from
//! (docs/FORMATS-PLAN.md, "How it is built: one document model, one
//! engine"). A node has a name, attributes where its format has them, its
//! own text or value, and what it holds in order; it keeps what a writer
//! needs to write the file back as it was (comments, CDATA, processing
//! instructions and the doctype of XML; the order of JSON's members and
//! numbers as written; the header of a CSV).
//!
//! How the formats map onto it:
//! - XML: the top node is the document; it holds the root element and the
//!   comments around it. Elements are `Element` nodes.
//! - JSON: the top node is the file's value. An object is an `Object` node
//!   whose children are its members; a list is a `List` node of the
//!   member's name whose items are its children, each named as the list
//!   is, so a list reads as repeated children of one name; a list at the
//!   top and its items have no name; a value is a `Value` node.
//! - YAML: as JSON; the document keeps its text (`Kept::text`), from which
//!   the writer keeps what did not change as it was written.
//! - CSV: the top node is a `List` of rows without names; a row is an
//!   `Object` of its cells, each a `Value` node named by its column; the
//!   header is the document's `columns`.

use crate::tree::Tree;

/// What a node is.
#[derive(Debug, Clone, PartialEq)]
pub enum Form {
    /// Named, with attributes and text; the entity says what it holds (XML).
    Element,
    /// Named members, each one node (a JSON object, a CSV row).
    Object,
    /// Items, each named as the list is (a JSON list).
    List,
    /// One value: null, yes or no, a number as written, or text.
    Value(Tree),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    /// As written, with any prefix (`o:order`).
    pub name: String,
    pub form: Form,
    /// In their order, names as written.
    pub attributes: Vec<(String, String)>,
    pub children: Vec<Content>,
}

/// What a node holds, in order.
#[derive(Debug, Clone, PartialEq)]
pub enum Content {
    Node(Node),
    Text(String),
    CData(String),
    Comment(String),
    /// A processing instruction, as written between `<?` and `?>`.
    Pi(String),
    /// The doctype, as written between `<!DOCTYPE` and `>`.
    DocType(String),
}

/// A whole file.
#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    /// The node the first step of a path is taken in.
    pub top: Node,
    /// A flat file's columns (its header row), in order.
    pub columns: Vec<String>,
    /// How the file was written, so it is written back the same; none for a new one.
    pub kept: Option<Kept>,
}

/// How a file read was written.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Kept {
    /// The encoding its declaration names, as written.
    pub encoding: Option<String>,
    pub bom: Bom,
    /// Whether the nodes around the root stand on their own lines.
    pub breaks: bool,
    /// Whether the file ends with a line break.
    pub final_break: bool,
    /// The text as read, for a format that writes a file back by what
    /// changed (YAML: its comments, quoting and spacing kept outside the
    /// values that changed). A record's node crosses a file source's table
    /// as its JSON, so what a format keeps of how it was written lives here,
    /// where it survives that.
    pub text: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Bom {
    #[default]
    None,
    Utf8,
    Utf16 {
        big: bool,
    },
}

/// A name without its prefix: `o:order` is `order`.
pub fn local(name: &str) -> &str {
    name.rsplit_once(':').map_or(name, |(_, l)| l)
}

/// Whether a name is the step's: the same, or the same without its prefix.
pub fn is_named(name: &str, step: &str) -> bool {
    name == step || local(name) == step
}

impl Node {
    pub fn new(name: &str, form: Form) -> Node {
        Node { name: name.to_string(), form, attributes: Vec::new(), children: Vec::new() }
    }

    pub fn element(name: &str) -> Node {
        Node::new(name, Form::Element)
    }

    pub fn value(name: &str, value: Tree) -> Node {
        Node::new(name, Form::Value(value))
    }

    /// The nodes it holds.
    pub fn nodes(&self) -> impl Iterator<Item = &Node> {
        self.children.iter().filter_map(|c| if let Content::Node(n) = c { Some(n) } else { None })
    }

    pub fn nodes_mut(&mut self) -> impl Iterator<Item = &mut Node> {
        self.children.iter_mut().filter_map(|c| if let Content::Node(n) = c { Some(n) } else { None })
    }

    pub fn has_nodes(&self) -> bool {
        self.nodes().next().is_some()
    }

    pub fn push(&mut self, node: Node) {
        self.children.push(Content::Node(node));
    }

    /// Its own text: the text and CDATA it holds directly, joined.
    pub fn text(&self) -> String {
        self.children
            .iter()
            .filter_map(|c| match c {
                Content::Text(t) | Content::CData(t) => Some(t.as_str()),
                _ => None,
            })
            .collect()
    }

    /// Replaces its own text, keeping the nodes and comments it holds.
    pub fn set_text(&mut self, text: &str) {
        self.children.retain(|c| !matches!(c, Content::Text(_) | Content::CData(_)));
        if !text.is_empty() {
            self.children.insert(0, Content::Text(text.to_string()));
        }
    }

    /// The value of an attribute, by its local name.
    pub fn attribute(&self, name: &str) -> Option<&str> {
        self.attributes.iter().find(|(n, _)| local(n) == name).map(|(_, v)| v.as_str())
    }

    /// Sets an attribute, by its local name, adding it when it is not there.
    pub fn set_attribute(&mut self, name: &str, value: &str) {
        match self.attributes.iter_mut().find(|(n, _)| local(n) == name) {
            Some((_, v)) => *v = value.to_string(),
            None => self.attributes.push((name.to_string(), value.to_string())),
        }
    }

    pub fn remove_attribute(&mut self, name: &str) {
        self.attributes.retain(|(n, _)| local(n) != name);
    }

    /// The node as Clarity's tree: an object, a list, a value; an element its text.
    pub fn to_tree(&self) -> Tree {
        match &self.form {
            Form::Object => Tree::Object(self.nodes().map(|n| (n.name.clone(), n.to_tree())).collect()),
            Form::List => Tree::List(self.nodes().map(Node::to_tree).collect()),
            Form::Value(v) => v.clone(),
            Form::Element => Tree::Text(self.text()),
        }
    }

    /// A tree as a node called `name`: an object's members by their names, a list's items by the list's.
    pub fn from_tree(name: &str, tree: &Tree) -> Node {
        match tree {
            Tree::Object(pairs) => {
                let mut node = Node::new(name, Form::Object);
                node.children = pairs.iter().map(|(n, v)| Content::Node(Node::from_tree(n, v))).collect();
                node
            }
            Tree::List(items) => {
                let mut node = Node::new(name, Form::List);
                node.children = items.iter().map(|i| Content::Node(Node::from_tree(name, i))).collect();
                node
            }
            other => Node::value(name, other.clone()),
        }
    }

    /// Renames it; a list's items are renamed with it.
    pub fn renamed(mut self, name: &str) -> Node {
        if self.form == Form::List {
            for item in self.nodes_mut() {
                item.name = name.to_string();
            }
        }
        self.name = name.to_string();
        self
    }
}

/// Puts `new` in the places of the nodes `is_slot` picks: item i takes the
/// place of old item i; a place left over goes with the blank text before
/// it; more follow the last, each after a copy of that blank; with no old
/// ones, at the end.
pub fn replace(holder: &mut Node, is_slot: impl Fn(&Node) -> bool, new: Vec<Node>) {
    let slots: Vec<usize> = holder.children.iter().enumerate().filter(|(_, c)| matches!(c, Content::Node(n) if is_slot(n))).map(|(i, _)| i).collect();
    let blank = |c: &Content| matches!(c, Content::Text(t) if t.trim().is_empty());
    let mut new = new.into_iter();
    let Some(&last) = slots.last() else {
        holder.children.extend(new.map(Content::Node));
        return;
    };
    let gap = last.checked_sub(1).map(|i| &holder.children[i]).filter(|c| blank(c)).cloned();
    let mut out = Vec::with_capacity(holder.children.len());
    for (i, content) in std::mem::take(&mut holder.children).into_iter().enumerate() {
        if !slots.contains(&i) {
            out.push(content);
            continue;
        }
        match new.next() {
            Some(n) => out.push(Content::Node(n)),
            None => {
                if out.last().is_some_and(blank) {
                    out.pop();
                }
            }
        }
        if i == last {
            for n in new.by_ref() {
                out.extend(gap.clone());
                out.push(Content::Node(n));
            }
        }
    }
    holder.children = out;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree;

    #[test]
    fn a_tree_is_a_node_and_back_with_names_and_numbers_as_written() {
        let t = tree::parse(r#"{"a":1.50,"orders":[{"n":1},{"n":2}],"tags":[]}"#).unwrap();
        let node = Node::from_tree("", &t);
        let orders: Vec<&str> = node.nodes().nth(1).unwrap().nodes().map(|n| n.name.as_str()).collect();
        assert_eq!(orders, ["orders", "orders"], "a list's items are named as the list is");
        assert_eq!(node.to_tree(), t);
    }
}
