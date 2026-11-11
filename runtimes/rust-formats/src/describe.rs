//! What a file holds, for Import (docs/FORMATS-PLAN.md, "Import, and the
//! XML Data Binding wizard made generic"): the tables it holds and their
//! columns, found in the document model alike for every format; and the
//! rules a schema gives (the tables an XSD declares, typed: xsd.rs), which
//! are the engine's one rule set. Types are never guessed from values: a
//! file's columns are text.
//!
//! The JSON is `[{"path":"shop/orders/order","count":2,"columns":[{"name":
//! "number","column":"number","kind":"text"}]}]`. A schema's column may
//! also have `values` and `type` (an enum and its name), `size` (a text's
//! longest), `precision` and `scale` (a decimal's digits), `required`,
//! `list`, and `table` (the path of the table a record column is).

use crate::doc::{local, Document, Form, Node};
use crate::fit::plain;
use crate::tree::{self, Tree};
use crate::Nodes;

/// A column of a table: its plain name, where the field finds it, and its kind.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Column {
    pub name: String,
    pub column: String,
    /// text, integer, decimal, date, datetime, bool, enum or record.
    pub kind: String,
    pub values: Vec<String>,
    /// The schema's name of an enum's type, when it has one.
    pub type_name: Option<String>,
    pub size: Option<u32>,
    pub digits: Option<(u32, u32)>,
    pub required: bool,
    pub list: bool,
    pub table: Option<String>,
}

/// A table a file holds: where it is (`at`), how many records, its columns.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Described {
    pub path: String,
    pub count: usize,
    pub columns: Vec<Column>,
}

impl Column {
    pub(crate) fn text(name: &str, column: &str) -> Column {
        Column { name: name.to_string(), column: column.to_string(), kind: "text".into(), ..Column::default() }
    }

    fn to_tree(&self) -> Tree {
        let text = |t: &str| Tree::Text(t.to_string());
        let number = |n: u32| Tree::Number(n.to_string());
        let mut pairs = vec![("name".to_string(), text(&self.name)), ("column".into(), text(&self.column)), ("kind".into(), text(&self.kind))];
        if !self.values.is_empty() {
            pairs.push(("values".into(), Tree::List(self.values.iter().map(|v| text(v)).collect())));
        }
        if let Some(name) = &self.type_name {
            pairs.push(("type".into(), text(name)));
        }
        if let Some(size) = self.size {
            pairs.push(("size".into(), number(size)));
        }
        if let Some((p, s)) = self.digits {
            pairs.push(("precision".into(), number(p)));
            pairs.push(("scale".into(), number(s)));
        }
        if self.required {
            pairs.push(("required".into(), Tree::Bool(true)));
        }
        if self.list {
            pairs.push(("list".into(), Tree::Bool(true)));
        }
        if let Some(table) = &self.table {
            pairs.push(("table".into(), text(table)));
        }
        Tree::Object(pairs)
    }
}

/// The tables as JSON.
pub fn to_json(tables: &[Described]) -> String {
    let list = tables
        .iter()
        .map(|t| {
            Tree::Object(vec![
                ("path".into(), Tree::Text(t.path.clone())),
                ("count".into(), Tree::Number(t.count.to_string())),
                ("columns".into(), Tree::List(t.columns.iter().map(Column::to_tree).collect())),
            ])
        })
        .collect();
    tree::write(&Tree::List(list))
}

/// What one path holds across the document.
#[derive(Default)]
struct Seen {
    path: String,
    count: usize,
    record: bool,
    attributes: Vec<String>,
    /// Every child's name, and those that are text columns.
    children: Vec<String>,
    texts: Vec<String>,
    /// Child names that are no text column: repeated, or holding more than one value.
    not_texts: Vec<String>,
}

fn push_new(list: &mut Vec<String>, name: &str) {
    if !list.iter().any(|n| n == name) {
        list.push(name.to_string());
    }
}

/// A namespace declaration or an instance attribute (`xsi:type`), not data.
fn declares(name: &str) -> bool {
    name == "xmlns" || name.starts_with("xmlns:") || name.starts_with("xsi:")
}

/// A node's children, a list's items in its place, each with whether it repeats.
fn children(node: &Node) -> Vec<(&Node, bool)> {
    let mut out = Vec::new();
    for child in node.nodes() {
        if child.form == Form::List {
            out.extend(child.nodes().map(|i| (i, true)));
        } else {
            let many = node.nodes().filter(|n| n.form != Form::List && n.name == child.name).count() > 1;
            out.push((child, many));
        }
    }
    out
}

fn join(path: &str, name: &str) -> String {
    if path.is_empty() { name.to_string() } else { format!("{path}/{name}") }
}

fn walk(node: &Node, path: &str, seen: &mut Vec<Seen>, repeated: bool) {
    let at = match seen.iter().position(|s| s.path == path) {
        Some(at) => at,
        None => {
            seen.push(Seen { path: path.to_string(), ..Seen::default() });
            seen.len() - 1
        }
    };
    let attributes: Vec<&str> = node.attributes.iter().map(|(n, _)| n.as_str()).filter(|n| !declares(n)).collect();
    let entry = &mut seen[at];
    entry.count += 1;
    entry.record |= repeated || !attributes.is_empty() || node.has_nodes();
    for name in attributes {
        push_new(&mut entry.attributes, local(name));
    }
    for (child, many) in children(node) {
        let name = local(&child.name);
        let entry = &mut seen[at];
        push_new(&mut entry.children, name);
        let one_value = !many && matches!(child.form, Form::Element | Form::Value(_)) && !child.has_nodes() && child.attributes.iter().all(|(n, _)| declares(n));
        if one_value {
            push_new(&mut entry.texts, name);
        } else {
            push_new(&mut entry.not_texts, name);
        }
        walk(child, &join(path, name), seen, many);
    }
}

/// The tables a document holds: each path whose nodes are records
/// (repeated, list items, or holding attributes or children), with the
/// attributes and the children that hold one value as columns; a path
/// without columns is left out (a wrapper such as `<orders>`). A flat file
/// is one table, its own columns.
pub fn describe(doc: &Document, nodes: Nodes) -> Vec<Described> {
    if nodes == Nodes::Rows {
        return vec![Described { path: String::new(), count: doc.top.nodes().count(), columns: doc.columns.iter().map(|c| Column::text(c, c)).collect() }];
    }
    let mut seen = Vec::new();
    for (child, many) in children(&doc.top) {
        walk(child, local(&child.name), &mut seen, many);
    }
    seen.into_iter()
        .filter(|s| s.record)
        .map(|s| {
            let children: Vec<String> = s.children.iter().map(|e| plain(e)).collect();
            let mut columns: Vec<Column> = Vec::new();
            for a in &s.attributes {
                // A field takes a child of its name before an attribute.
                let column = if children.contains(&plain(a)) { format!("@{a}") } else { a.clone() };
                columns.push(Column::text(a, &column));
            }
            for t in s.texts.iter().filter(|t| !s.not_texts.contains(t)) {
                columns.push(Column::text(t, t));
            }
            Described { path: s.path, count: s.count, columns }
        })
        .filter(|t| !t.columns.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A format's tables, as Import reaches them.
    fn of(format: &str, input: &[u8], options: &str) -> Vec<Described> {
        let json = crate::describe(format, input, options).unwrap();
        let parsed = tree::parse(&json).unwrap();
        let Tree::List(tables) = parsed else { panic!() };
        tables
            .iter()
            .map(|t| Described {
                path: t.get("path").and_then(Tree::as_text).unwrap().to_string(),
                count: t.get("count").map(tree::write).unwrap().parse().unwrap(),
                columns: match t.get("columns") {
                    Some(Tree::List(c)) => c.iter().map(|c| Column::text(c.get("name").and_then(Tree::as_text).unwrap(), c.get("column").and_then(Tree::as_text).unwrap())).collect(),
                    _ => panic!(),
                },
            })
            .collect()
    }

    #[test]
    fn a_csv_is_one_table_of_its_header() {
        let t = of("csv", b"Login URL;visits\r\na;1\r\nb;2\r\n", r#"{"separator":";"}"#);
        assert_eq!(to_json(&t), r#"[{"path":"","count":2,"columns":[{"name":"Login URL","column":"Login URL","kind":"text"},{"name":"visits","column":"visits","kind":"text"}]}]"#);
        let t = of("csv", b"a,1,x\nb,2\n", r#"{"noHeader":true}"#);
        assert_eq!(t[0].columns.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(), ["column1", "column2", "column3"]);
        assert_eq!(t[0].count, 2);
    }

    #[test]
    fn json_lists_its_records_by_path_with_slashes() {
        let t = of("json", br#"{"data":{"orders":[{"a":1},{"a":2,"b":{"c":1}}],"tags":["x"]},"users":[{"name":"Ada"}]}"#, "");
        assert_eq!(
            to_json(&t),
            r#"[{"path":"data/orders","count":2,"columns":[{"name":"a","column":"a","kind":"text"}]},{"path":"data/orders/b","count":1,"columns":[{"name":"c","column":"c","kind":"text"}]},{"path":"users","count":1,"columns":[{"name":"name","column":"name","kind":"text"}]}]"#
        );
    }

    #[test]
    fn xml_lists_its_records_by_path_with_attributes_and_text_elements() {
        let text = br#"<?xml version="1.0"?><shop xmlns:o="urn:o"><orders><o:order number="A-1" note="x"><note>leave</note><customer><name>Ada</name></customer><lines><line product="Coffee" quantity="2"/><line product="Tea"/></lines><tag>a</tag><tag>b</tag></o:order><o:order number="A-2"><placed>2026-11-01</placed></o:order></orders></shop>"#;
        let t = of("xml", text, "");
        let paths: Vec<(&str, usize)> = t.iter().map(|t| (t.path.as_str(), t.count)).collect();
        assert_eq!(paths, [("shop/orders/order", 2), ("shop/orders/order/customer", 1), ("shop/orders/order/lines/line", 2)]);
        let order: Vec<(&str, &str)> = t[0].columns.iter().map(|c| (c.name.as_str(), c.column.as_str())).collect();
        // `note` is both: the attribute is `@note`, the element is found by its name.
        assert_eq!(order, [("number", "number"), ("note", "@note"), ("note", "note"), ("placed", "placed")]);
        assert_eq!(t[2].columns.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(), ["product", "quantity"]);
    }

    #[test]
    fn xml_in_windows_1252_is_read() {
        let t = of("xml", b"<?xml version=\"1.0\" encoding=\"windows-1252\"?><a><b caf\xe9=\"1\"/></a>", "");
        assert_eq!(t[0].columns[0].name, "caf\u{e9}");
    }
}
