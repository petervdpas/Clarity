//! A file in a format as tables, for a data source that is a file
//! (docs/FORMATS-PLAN.md, "A file as a data source"), the same for every
//! format. The runtime keeps the tables in an in-memory SQLite database
//! while the program runs and writes them back after each change, so a file
//! source finds, saves and deletes as a database does.
//!
//! One table is JSON: `{"columns":["#node","number"],"rows":[["<order/>","A-1"]]}`.
//! - A flat file (CSV): its own columns in its order, so writing back keeps
//!   the order and the columns no entity maps; every mapped column must be
//!   one of them.
//! - A file that nests (XML, JSON): the first column, `#node`, holds each
//!   record's node as its format writes one (an element's markup, an
//!   object's JSON), so writing back keeps what no column maps in its
//!   place; then one column per field the entity maps, found at its path
//!   (its name, or its path in the mapping's `fields`). Nothing found, or
//!   an empty element or attribute, is null.
//!
//! The columns an entity maps are given with their kinds: what is read is
//! fitted to them as SQLite keeps the stack's values (a yes or no as 1 or 0,
//! a decimal, a date or a list of values as text), and turned back on
//! writing; other columns keep their text.
//!
//! Several tables, when the mapping nests records (`nested`): the field
//! holding them is given among the fields with its kind (`records`, its
//! entity and fields), and the JSON is `{"tables":[{"name":"","columns":
//! [...],"rows":[...]},{"name":"lines","entity":"Line","columns":["#node",
//! "#parent",...],"rows":[...]}]}`: the source's own table first, named "",
//! then each nested one, named by the fields that lead to it (`lines`,
//! `lines/notes`), with its entity. Its second column links a row to the
//! row it sits in: the place (from 1) of that row in its table, named by
//! the nested mapping's `parent`, else `#parent`. Writing back puts each
//! nested row in the record of the row it links to, in the order given; a
//! row linking to no row is left out.

use crate::check;
use crate::doc::{is_named, local, Content, Document, Form, Node};
use crate::fit::{self, plain};
use crate::format::Format;
use crate::kind::{Field, Kind};
use crate::mapping::Mapping;
use crate::markup;
use crate::path::{self, Path, Step, Target, Why};
use crate::shape::{self, column};
use crate::tree::{self, Tree};
use crate::{FormatInfo, Nodes, Problem, ProblemKind};

/// The column that holds each record's node.
pub const NODE: &str = "#node";
/// The column that links a nested row to its row, when the mapping names none.
pub const PARENT: &str = "#parent";

/// A table's columns and rows.
#[derive(Debug, Clone, PartialEq)]
pub struct Table {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Tree>>,
}

impl Table {
    fn to_tree(&self) -> Tree {
        let columns = Tree::List(self.columns.iter().map(|c| Tree::Text(c.clone())).collect());
        let rows = Tree::List(self.rows.iter().map(|r| Tree::List(r.clone())).collect());
        Tree::Object(vec![("columns".into(), columns), ("rows".into(), rows)])
    }

    pub fn to_json(&self) -> String {
        tree::write(&self.to_tree())
    }

    fn from_tree(table: &Tree) -> Option<Table> {
        let Some(Tree::List(columns)) = table.get("columns") else { return None };
        let Some(Tree::List(rows)) = table.get("rows") else { return None };
        let columns = columns.iter().map(|c| c.as_text().map(str::to_string)).collect::<Option<Vec<_>>>()?;
        let rows = rows.iter().map(|r| if let Tree::List(cells) = r { Some(cells.clone()) } else { None }).collect::<Option<Vec<_>>>()?;
        Some(Table { columns, rows })
    }

    pub fn from_json(text: &str) -> Option<Table> {
        Table::from_tree(&tree::parse(text)?)
    }
}

fn not(text: impl Into<String>) -> Problem {
    Problem::new(ProblemKind::NotFitting, text)
}

/// The mapped column a file column is: by its exact name, else by its plain name.
fn mapped<'a>(column: &str, fields: &'a [(String, &'a Field)]) -> Option<&'a Field> {
    fields.iter().find(|(n, _)| n == column).or_else(|| fields.iter().find(|(n, _)| plain(n) == plain(column))).map(|(_, f)| *f)
}

/// A cell read, fitted to the kind of its column as SQLite keeps it.
fn stored(cell: Tree, kind: &Kind) -> Option<Tree> {
    if cell == Tree::Null {
        return Some(Tree::Null);
    }
    let fitted = fit::value(&cell, kind)?;
    Some(match fitted {
        Tree::Bool(b) => Tree::Number(if b { "1" } else { "0" }.into()),
        Tree::List(_) => Tree::Text(tree::write(&fitted)),
        other => other,
    })
}

/// A cell fitted, or why not: `row 2: column 'visits' is an integer, not "many"`.
fn fitted(cell: Tree, row: usize, name: &str, kind: &Kind) -> Result<Tree, Problem> {
    let shown = tree::write(&cell);
    stored(cell, kind).ok_or_else(|| not(format!("row {row}: column '{name}' is {}, not {shown}", kind.wanted())))
}

/// A stored cell turned back to what the file holds for its column's kind:
/// typed where the format types its values (JSON), else text. Public for
/// tools that show a table's cells as their fields hold them (the IDE's
/// mapping preview).
pub fn back(cell: &Tree, kind: Option<&Kind>, typed: bool) -> Tree {
    match (kind, cell) {
        (_, Tree::Null) => Tree::Null,
        (Some(Kind::Bool), Tree::Number(n)) => Tree::Bool(n != "0"),
        (Some(Kind::Values(_)), Tree::Text(t)) if typed => tree::parse(t).unwrap_or_else(|| cell.clone()),
        (Some(Kind::Decimal), Tree::Text(t)) if typed => crate::numbers::shortest(t).map(Tree::Number).unwrap_or_else(|| cell.clone()),
        _ => cell.clone(),
    }
}

/// The columns an entity maps, from JSON: `[{"name":"site","kind":"text"}, ...]`.
pub fn fields_from_json(text: &str) -> Result<Vec<Field>, String> {
    if text.trim().is_empty() {
        return Ok(Vec::new());
    }
    let list = tree::parse(text).ok_or("the columns are no JSON")?;
    let kind = Kind::from_tree(&Tree::Object(vec![
        ("kind".into(), Tree::Text("record".into())),
        ("entity".into(), Tree::Text(String::new())),
        ("fields".into(), list),
        ("base".into(), Tree::Object(Vec::new())),
    ]))?;
    match kind {
        Kind::Record(shape) => Ok(shape.fields),
        _ => Err("the columns are a list of names and kinds".into()),
    }
}

/// A column of a nesting file's table: its name, where it is, its kind.
struct Column {
    name: String,
    path: Path,
    kind: Kind,
}

/// One table of a nesting file: where its records are, its columns, the tables nested in it.
struct Spec {
    name: String,
    entity: Option<String>,
    /// Relative to the record around it; for the source's own table, the mapping's `at`.
    at: Option<Path>,
    link: Option<String>,
    columns: Vec<Column>,
    inner: Vec<Spec>,
}

impl Spec {
    fn of(info: &FormatInfo, name: String, entity: Option<String>, at: Option<Path>, link: Option<String>, fields: &[Field], map: &Mapping) -> Result<Spec, Problem> {
        let mut columns = Vec::new();
        for field in fields.iter().filter(|f| !matches!(f.kind, Kind::Record(_) | Kind::Records(_))) {
            if link.as_deref() == Some(field.name.as_str()) {
                continue;
            }
            let path = match map.path(&field.name) {
                Some(p) => p.clone(),
                None => Path::field(&field.name, info.nodes, info.attributes).map_err(|why| match why {
                    Why::NoPath => not(format!("the column '{}' is no {} path", field.name, info.title)),
                    Why::Attribute => not(format!("the column '{}' names an attribute, which {} does not have", field.name, info.title)),
                })?,
            };
            if matches!(field.kind, Kind::Values(_)) && !matches!(path.steps.last(), Some(Step::Name(_))) {
                return Err(not(format!("the column '{}' is a list, kept in elements of its name", field.name)));
            }
            columns.push(Column { name: field.name.clone(), path, kind: field.kind.clone() });
        }
        let mut inner = Vec::new();
        for (key, nested) in &map.nested {
            let Some(Kind::Records(shape)) = fields.iter().find(|f| f.name == *key).map(|f| &f.kind) else {
                return Err(Problem::new(ProblemKind::BadOptions, format!("the nested '{key}' is no field of records among the fields given")));
            };
            let inner_name = if name.is_empty() { key.clone() } else { format!("{name}/{key}") };
            let at = nested.at.clone().unwrap_or_else(|| Path::name(key));
            let link = nested.parent.clone().unwrap_or_else(|| PARENT.to_string());
            inner.push(Spec::of(info, inner_name, Some(shape.entity.clone()), Some(at), Some(link), &shape.fields, &nested.map)?);
        }
        Ok(Spec { name, entity, at, link, columns, inner })
    }

    fn names(&self) -> Vec<String> {
        let mut names = vec![NODE.to_string()];
        names.extend(self.link.clone());
        names.extend(self.columns.iter().map(|c| c.name.clone()));
        names
    }

    /// It and the tables nested in it, in order.
    fn all(&self) -> Vec<&Spec> {
        let mut out = vec![self];
        for inner in &self.inner {
            out.extend(inner.all());
        }
        out
    }
}

/// The node of a record as its format writes one: an element's markup, else its JSON.
fn node_text(info: &FormatInfo, node: &Node) -> String {
    match info.nodes {
        Nodes::Elements => markup::element_text(node),
        _ => tree::write(&node.to_tree()),
    }
}

fn node_of(info: &FormatInfo, text: &str, name: &str) -> Option<Node> {
    match info.nodes {
        Nodes::Elements => markup::parse_element(text).ok(),
        _ => tree::parse(text).map(|t| Node::from_tree(name, &t)),
    }
}

/// The records `nodes` are: one list's items, else the nodes; each a record, else why not.
fn records<'a>(info: &FormatInfo, nodes: Vec<&'a Node>) -> Result<Vec<&'a Node>, Problem> {
    let nodes = match nodes.as_slice() {
        [one] if one.form == Form::List => one.nodes().collect(),
        _ => nodes,
    };
    if nodes.iter().any(|n| !matches!(n.form, Form::Element | Form::Object)) {
        return Err(not(format!("the {} is not a list of records", info.title)));
    }
    Ok(nodes)
}

/// Reads `input`, a file in `format`, as its table, or tables where the mapping nests.
pub fn read(format: &dyn Format, info: &FormatInfo, input: &[u8], options: &[(String, Tree)], map: &Mapping, fields: &[Field]) -> Result<String, Problem> {
    let rules = check::rules(format, info, options)?;
    if info.nodes == Nodes::Rows {
        return read_flat(format, info, input, options, map, fields).map(|t| t.to_json());
    }
    let spec = Spec::of(info, String::new(), None, map.at.clone(), None, fields, map)?;
    let specs = spec.all();
    let mut tables: Vec<Table> = specs.iter().map(|s| Table { columns: s.names(), rows: Vec::new() }).collect();
    // A new file, not written yet: no rows.
    if !input.is_empty() {
        let doc = format.parse(input, options, &[])?;
        if let Some(rules) = &rules {
            check::document(&doc, info, rules)?;
        }
        let found = shape::found(&doc, info, map.at.as_ref(), true)?;
        for node in records(info, found.nodes)? {
            read_row(info, &specs, 0, node, &found.up, None, &mut tables)?;
        }
    }
    Ok(several(map, &specs, tables))
}

/// The tables as JSON: one as it always was, several when the mapping nests.
fn several(map: &Mapping, specs: &[&Spec], tables: Vec<Table>) -> String {
    if map.nested.is_empty() {
        return tables[0].to_json();
    }
    let list = specs
        .iter()
        .zip(tables)
        .map(|(spec, table)| {
            let Tree::Object(mut pairs) = table.to_tree() else { unreachable!() };
            let mut head = vec![("name".to_string(), Tree::Text(spec.name.clone()))];
            head.extend(spec.entity.clone().map(|e| ("entity".to_string(), Tree::Text(e))));
            head.append(&mut pairs);
            Tree::Object(head)
        })
        .collect();
    tree::write(&Tree::Object(vec![("tables".into(), Tree::List(list))]))
}

fn read_row<'a>(info: &FormatInfo, specs: &[&Spec], at: usize, node: &'a Node, up: &[&'a Node], parent: Option<usize>, tables: &mut [Table]) -> Result<(), Problem> {
    let spec = specs[at];
    let number = tables[at].rows.len() + 1;
    let mut row = vec![Tree::Text(node_text(info, node))];
    if let Some(parent) = parent {
        row.push(Tree::Number(parent.to_string()));
    }
    for column in &spec.columns {
        row.push(fitted(cell(node, up, column), number, &column.name, &column.kind)?);
    }
    tables[at].rows.push(row);
    for inner in &spec.inner {
        let index = specs.iter().position(|s| std::ptr::eq(*s, inner)).expect("one of them");
        let Some(at) = &inner.at else { continue };
        if let Target::Nodes(found, _) = path::resolve(node, up, at) {
            // `..` from them is this record, not what the path went through.
            let mut around = up.to_vec();
            around.push(node);
            for inner_node in records(info, found)? {
                read_row(info, specs, index, inner_node, &around, Some(number), tables)?;
            }
        }
    }
    Ok(())
}

/// The value of a column in a record node, as the file holds it.
fn cell(node: &Node, up: &[&Node], column: &Column) -> Tree {
    let target = path::resolve(node, up, &column.path);
    if matches!(column.kind, Kind::Values(_)) {
        return match target {
            Target::Nodes(found, _) if found[0].form == Form::Element => list_of(&found),
            Target::Nodes(found, _) => found[0].to_tree(),
            _ => Tree::Null,
        };
    }
    let text = |t: String| if t.is_empty() { Tree::Null } else { Tree::Text(t) };
    match target {
        Target::Missing => Tree::Null,
        Target::Text(t) => text(t),
        Target::Own(n) => match &n.form {
            Form::Element => text(n.text()),
            Form::Value(v) => v.clone(),
            _ => Tree::Null,
        },
        Target::Nodes(found, _) => match &found[0].form {
            Form::Element => text(found[0].text()),
            Form::Value(v) => v.clone(),
            // A record or list inside is kept as its JSON text.
            _ => Tree::Text(tree::write(&found[0].to_tree())),
        },
    }
}

/// Whether an element holds items: elements that all have one name.
fn wrapper(e: &Node) -> bool {
    let mut names = e.nodes().map(|c| local(&c.name));
    names.next().is_some_and(|first| names.all(|n| n == first))
}

/// The items of a list: the elements, or the children of one element when
/// they all have one name; one empty element is an empty list.
fn list_of(found: &[&Node]) -> Tree {
    let items: Vec<&Node> = match found {
        [one] if wrapper(one) => one.nodes().collect(),
        [one] if !one.has_nodes() && one.text().trim().is_empty() => Vec::new(),
        _ => found.to_vec(),
    };
    Tree::List(items.iter().map(|i| Tree::Text(i.text())).collect())
}

/// A flat file's own columns and rows: every column the entity maps must be
/// there; a field the mapping names takes its column under its own name.
fn read_flat(format: &dyn Format, info: &FormatInfo, input: &[u8], options: &[(String, Tree)], map: &Mapping, fields: &[Field]) -> Result<Table, Problem> {
    let named: Vec<(String, &Field)> = fields.iter().map(|f| (column(f, map), f)).collect();
    if input.is_empty() {
        return Ok(Table { columns: fields.iter().map(|f| f.name.clone()).collect(), rows: Vec::new() });
    }
    let names: Vec<String> = named.iter().map(|(n, _)| n.clone()).collect();
    let doc = format.parse(input, options, &names)?;
    for (name, field) in &named {
        if !doc.columns.iter().any(|c| mapped(c, &[(name.clone(), *field)]).is_some()) {
            return Err(not(format!("the {} has no column '{name}'", info.title)));
        }
    }
    let kinds: Vec<Option<&Field>> = doc.columns.iter().map(|c| mapped(c, &named)).collect();
    let columns = doc.columns.iter().zip(&kinds).map(|(c, f)| match f {
        Some(f) if map.path(&f.name).is_some() => f.name.clone(),
        _ => c.clone(),
    });
    let columns: Vec<String> = columns.collect();
    let mut rows = Vec::new();
    for (i, row) in doc.top.nodes().enumerate() {
        let mut cells = Vec::with_capacity(columns.len());
        for (cell, field) in row.nodes().map(Node::to_tree).zip(&kinds) {
            cells.push(match field {
                Some(field) => fitted(cell, i + 1, &field.name, &field.kind)?,
                None => cell,
            });
        }
        rows.push(cells);
    }
    Ok(Table { columns, rows })
}

/// Writes `tables` (JSON, as `read` gives them) back into a file in `format`,
/// inside `original` where the format keeps more than the tables.
pub fn write(format: &dyn Format, info: &FormatInfo, original: &[u8], tables: &str, options: &[(String, Tree)], map: &Mapping, fields: &[Field]) -> Result<Vec<u8>, Problem> {
    let failed = |t: &str| Problem::new(ProblemKind::Failed, t.to_string());
    let given = tree::parse(tables).ok_or_else(|| failed("the table to write is no table"))?;
    if info.nodes == Nodes::Rows {
        let table = Table::from_tree(&given).ok_or_else(|| failed("the table to write is no table"))?;
        return write_flat(format, &table, options, map, fields);
    }
    let spec = Spec::of(info, String::new(), None, map.at.clone(), None, fields, map)?;
    let specs = spec.all();
    let tables: Vec<Table> = if map.nested.is_empty() {
        vec![Table::from_tree(&given).ok_or_else(|| failed("the table to write is no table"))?]
    } else {
        let Some(Tree::List(list)) = given.get("tables") else { return Err(failed("the tables to write are no tables")) };
        specs
            .iter()
            .map(|s| list.iter().find(|t| t.get("name").and_then(Tree::as_text) == Some(s.name.as_str())).and_then(Table::from_tree).unwrap_or(Table { columns: s.names(), rows: Vec::new() }))
            .collect()
    };
    let names = map.at.as_ref().map(Path::names).unwrap_or_default();
    let fresh = original.is_empty();
    let title = info.title;
    let mut doc = if fresh {
        match info.nodes {
            Nodes::Elements if names.is_empty() => return Err(not(format!("an {title} file source needs 'at' to name its elements"))),
            Nodes::Elements => Document { top: Node::element(""), columns: Vec::new(), kept: None },
            _ => Document { top: Node::new("", if names.is_empty() { Form::List } else { Form::Object }), columns: Vec::new(), kept: None },
        }
    } else {
        format.parse(original, options, &[])?
    };
    let last = names.last().copied();
    let mut nodes = Vec::with_capacity(tables[0].rows.len());
    for i in 0..tables[0].rows.len() {
        nodes.push(row_node(info, &specs, &tables, 0, i, last)?);
    }
    let (holder, slot) = holder(info, &mut doc.top, &names, fresh)?;
    if let Some(slot) = slot {
        let slot = slot.to_string();
        crate::doc::replace(holder, |n| is_named(&n.name, &slot), nodes);
    } else {
        let any_element = info.nodes == Nodes::Elements;
        crate::doc::replace(holder, |n| !any_element || n.form == Form::Element, nodes);
    }
    format.write(&doc, options)
}

/// The node that holds the records and the name they have there (none: every node it holds).
fn holder<'a, 'n>(info: &FormatInfo, top: &'a mut Node, names: &[&'n str], fresh: bool) -> Result<(&'a mut Node, Option<&'n str>), Problem> {
    let title = info.title;
    if info.nodes == Nodes::Elements {
        let Some((last, before)) = names.split_last() else {
            let root = top.nodes_mut().next().ok_or_else(|| not(format!("the {title} has no root element")))?;
            return Ok((root, None));
        };
        // A path of one name names the root itself, which the top holds.
        let first = before.first().unwrap_or(last);
        let root = top.nodes().next().map(|r| r.name.clone());
        match root {
            Some(root) if is_named(&root, first) => {}
            None if fresh => {
                if !before.is_empty() {
                    top.push(Node::element(first));
                }
            }
            _ => return Err(not(format!("the {title} has no '{first}'"))),
        }
        let mut here = top;
        for step in before {
            here = child(here, step, Form::Element);
        }
        return Ok((here, Some(*last)));
    }
    let Some((last, before)) = names.split_last() else {
        if top.form != Form::List {
            return Err(not(format!("the {title} is not a list of records")));
        }
        return Ok((top, None));
    };
    let mut here = top;
    for step in before {
        here = child(here, step, Form::Object);
    }
    let list = child(here, last, Form::List);
    if list.form != Form::List {
        return Err(not(format!("the {title} is not a list of records")));
    }
    Ok((list, None))
}

/// The child called `name`, made (of `form`) where missing.
fn child<'a>(here: &'a mut Node, name: &str, form: Form) -> &'a mut Node {
    let found = here.nodes().position(|n| is_named(&n.name, name));
    let at = match found {
        Some(at) => at,
        None => {
            here.push(Node::new(name, form));
            here.nodes().count() - 1
        }
    };
    here.nodes_mut().nth(at).expect("just found")
}

/// Row `i` of table `at` as its record's node, the rows nested in it inside.
fn row_node(info: &FormatInfo, specs: &[&Spec], tables: &[Table], at: usize, i: usize, name: Option<&str>) -> Result<Node, Problem> {
    let spec = specs[at];
    let table = &tables[at];
    let row = &table.rows[i];
    let title = info.title;
    let cell_of = |column: &str| table.columns.iter().position(|c| c == column).and_then(|p| row.get(p));
    let mut node = match cell_of(NODE) {
        Some(Tree::Text(t)) => node_of(info, t, name.unwrap_or("")).ok_or_else(|| not(format!("row {}: its node is no {title}", i + 1)))?,
        _ => match info.nodes {
            Nodes::Elements => Node::element(name.ok_or_else(|| not(format!("an {title} file source needs 'at' to name the element of a new record")))?),
            _ => Node::new(name.unwrap_or(""), Form::Object),
        },
    };
    for column in &spec.columns {
        let Some(cell) = cell_of(&column.name) else { continue };
        set(info, &mut node, column, cell, i)?;
    }
    for inner in &spec.inner {
        let index = specs.iter().position(|s| std::ptr::eq(*s, inner)).expect("one of them");
        let inner_table = &tables[index];
        let link = inner.link.as_deref().unwrap_or(PARENT);
        let Some(link_at) = inner_table.columns.iter().position(|c| c == link) else { continue };
        let Some(at) = inner.at.as_ref() else { continue };
        let names = at.names();
        let Some((last, before)) = names.split_last() else { continue };
        let mut nodes = Vec::new();
        for (j, inner_row) in inner_table.rows.iter().enumerate() {
            let place = match inner_row.get(link_at) {
                Some(Tree::Number(n)) => n.parse::<usize>().ok(),
                Some(Tree::Text(t)) => t.trim().parse::<usize>().ok(),
                _ => None,
            };
            if place == Some(i + 1) {
                nodes.push(row_node(info, specs, tables, index, j, Some(last))?);
            }
        }
        let form = if info.nodes == Nodes::Elements { Form::Element } else { Form::Object };
        let mut here = &mut node;
        for step in before {
            here = child(here, step, form.clone());
        }
        if info.nodes == Nodes::Elements {
            let last = last.to_string();
            crate::doc::replace(here, |n| is_named(&n.name, &last), nodes);
        } else {
            let list = child(here, last, Form::List);
            list.children = nodes.into_iter().map(Content::Node).collect();
        }
    }
    Ok(node)
}

/// Sets a column's value in a record node.
fn set(info: &FormatInfo, node: &mut Node, column: &Column, cell: &Tree, row: usize) -> Result<(), Problem> {
    let typed = info.nodes == Nodes::Values;
    if typed {
        let value = back(cell, Some(&column.kind), true);
        let as_text = cell.as_text().map(str::to_string);
        set_value(node, &column.path, value, as_text);
        return Ok(());
    }
    if matches!(column.kind, Kind::Values(_)) {
        let items = match cell {
            Tree::Null => None,
            Tree::Text(t) => match tree::parse(t) {
                Some(Tree::List(items)) => Some(items.iter().map(crate::csv::cell).collect()),
                _ => return Err(not(format!("row {}: column '{}' is a list, not {}", row + 1, column.name, tree::write(cell)))),
            },
            other => return Err(not(format!("row {}: column '{}' is a list, not {}", row + 1, column.name, tree::write(other)))),
        };
        set_list(node, &column.path, items);
        return Ok(());
    }
    let back = back(cell, Some(&column.kind), false);
    let value = (back != Tree::Null).then(|| crate::csv::cell(&back));
    set_element(node, &column.path, value.as_deref());
    Ok(())
}

/// The index of a child node by name: exact (or local), else plain.
fn child_at(e: &Node, name: &str, plainly: bool) -> Option<usize> {
    let at = |exact: bool| e.children.iter().position(|c| matches!(c, Content::Node(n) if if exact { is_named(&n.name, name) } else { plain(local(&n.name)) == plain(name) }));
    at(true).or_else(|| if plainly { at(false) } else { None })
}

fn attribute_at(e: &Node, name: &str) -> Option<usize> {
    e.attributes.iter().position(|(n, _)| local(n) == name).or_else(|| e.attributes.iter().position(|(n, _)| plain(local(n)) == plain(name)))
}

fn node_mut(e: &mut Node, at: usize) -> &mut Node {
    match &mut e.children[at] {
        Content::Node(n) => n,
        _ => unreachable!("a node's place"),
    }
}

/// Sets a value in a node of values (JSON): in its member, made where
/// missing; null removes the member; a member held as text stays text
/// (`as_text`, the cell as stored).
fn set_value(node: &mut Node, path: &Path, value: Tree, as_text: Option<String>) {
    if let [Step::Here] = path.steps.as_slice() {
        if matches!(node.form, Form::Value(_)) {
            node.form = Form::Value(value);
        }
        return;
    }
    let names: Vec<&str> = path.names();
    if names.iter().any(|n| n.is_empty()) {
        return;
    }
    let (last, before) = names.split_last().expect("a path has a step");
    let mut here = node;
    for step in before {
        let at = match child_at(here, step, false) {
            Some(at) => at,
            None if value == Tree::Null => return,
            None => {
                here.push(Node::new(step, Form::Object));
                here.children.len() - 1
            }
        };
        here = node_mut(here, at);
    }
    let at = child_at(here, last, path.single().is_some());
    match (at, value) {
        (Some(at), Tree::Null) => {
            here.children.remove(at);
        }
        (Some(at), value) => {
            let old = node_mut(here, at);
            // A number the file held as text stays text, as it was written.
            let value = match (&old.form, value) {
                (Form::Value(Tree::Text(_)), Tree::Number(n)) => Tree::Text(as_text.unwrap_or(n)),
                (_, value) => value,
            };
            let name = old.name.clone();
            *old = Node::from_tree(&name, &value);
        }
        (None, Tree::Null) => {}
        (None, value) => here.push(Node::from_tree(last, &value)),
    }
}

/// Sets a value in an element: a plain name in the child element of that
/// name, else the attribute of that name, else a new child element; a path
/// in the element or attribute it names, made where missing; null empties
/// an element's text or removes an attribute.
fn set_element(e: &mut Node, path: &Path, value: Option<&str>) {
    if let Some(name) = path.single() {
        if let Some(i) = child_at(e, name, true) {
            node_mut(e, i).set_text(value.unwrap_or(""));
        } else if let Some(i) = attribute_at(e, name) {
            match value {
                Some(v) => e.attributes[i].1 = v.to_string(),
                None => {
                    e.attributes.remove(i);
                }
            }
        } else if let Some(v) = value {
            let mut c = Node::element(name);
            c.set_text(v);
            e.push(c);
        }
        return;
    }
    let Some((last, before)) = path.steps.split_last() else { return };
    let mut here = e;
    for step in before {
        let Step::Name(n) = step else { continue };
        let at = match (child_at(here, n, false), value) {
            (Some(at), _) => at,
            // Nothing to empty where there is nothing.
            (None, None) => return,
            (None, Some(_)) => {
                here.push(Node::element(n));
                here.children.len() - 1
            }
        };
        here = node_mut(here, at);
    }
    match (last, value) {
        (Step::Attribute(a), Some(v)) => here.set_attribute(a, v),
        (Step::Attribute(a), None) => here.remove_attribute(a),
        (Step::Here, v) => here.set_text(v.unwrap_or("")),
        (Step::Name(n), v) => {
            let at = match (child_at(here, n, false), v) {
                (Some(at), _) => at,
                (None, None) => return,
                (None, Some(_)) => {
                    here.push(Node::element(n));
                    here.children.len() - 1
                }
            };
            node_mut(here, at).set_text(v.unwrap_or(""));
        }
        (Step::Up, _) => {}
    }
}

/// Sets a list column: its items as elements of its name in the old ones'
/// places (inside the old wrapper when there was one); null removes them;
/// an empty list is one empty element.
fn set_list(e: &mut Node, path: &Path, items: Option<Vec<String>>) {
    let names = path.names();
    let (name, steps, plainly) = match path.single() {
        Some(n) => (n, &names[..0], true),
        None => (names[names.len() - 1], &names[..names.len() - 1], false),
    };
    let mut here = e;
    for step in steps {
        let at = match (child_at(here, step, false), &items) {
            (Some(at), _) => at,
            (None, None) => return,
            (None, Some(_)) => {
                here.push(Node::element(step));
                here.children.len() - 1
            }
        };
        here = node_mut(here, at);
    }
    let has = |n: &Node, plainly: bool| if plainly { plain(local(&n.name)) == plain(name) } else { is_named(&n.name, name) };
    let plainly = plainly && !here.nodes().any(|n| has(n, false));
    let found: Vec<usize> = here.children.iter().enumerate().filter(|(_, c)| matches!(c, Content::Node(n) if has(n, plainly))).map(|(i, _)| i).collect();
    let element = |item_name: &str, text: &str| {
        let mut c = Node::element(item_name);
        c.set_text(text);
        c
    };
    if let ([one], Some(items)) = (found.as_slice(), &items) {
        let w = node_mut(here, *one);
        if wrapper(w) {
            let item = w.nodes().next().map(|c| c.name.clone()).unwrap_or_default();
            let new = items.iter().map(|t| element(&item, t)).collect();
            crate::doc::replace(w, |n| n.name == item, new);
            return;
        }
    }
    let named = found.first().map(|i| node_mut(here, *i).name.clone()).unwrap_or_else(|| name.to_string());
    let new = match items {
        None => Vec::new(),
        Some(items) if items.is_empty() => vec![Node::element(&named)],
        Some(items) => items.iter().map(|t| element(&named, t)).collect(),
    };
    crate::doc::replace(here, |n| has(n, plainly), new);
}

/// A flat file's table written whole: its columns in its order, a mapped
/// field's column under the name the mapping gives it.
fn write_flat(format: &dyn Format, table: &Table, options: &[(String, Tree)], map: &Mapping, fields: &[Field]) -> Result<Vec<u8>, Problem> {
    let named: Vec<(String, &Field)> = fields.iter().map(|f| (f.name.clone(), f)).collect();
    let kinds: Vec<Option<&Field>> = table.columns.iter().map(|c| mapped(c, &named)).collect();
    let columns: Vec<String> = table.columns.iter().zip(&kinds).map(|(c, f)| f.filter(|f| map.path(&f.name).is_some()).map(|f| column(f, map)).unwrap_or_else(|| c.clone())).collect();
    let mut top = Node::new("", Form::List);
    for row in &table.rows {
        let mut node = Node::new("", Form::Object);
        for ((cell, field), name) in row.iter().zip(&kinds).zip(&columns) {
            node.push(Node::value(name, back(cell, field.map(|f| &f.kind), false)));
        }
        top.push(node);
    }
    format.write(&Document { top, columns, kept: None }, options)
}
