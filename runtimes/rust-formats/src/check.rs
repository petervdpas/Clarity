//! A mapping's `check` (docs/FORMATS-PLAN.md, step 4): a file read is
//! checked against a schema before anything is fitted, written once for
//! every format with schema kinds. The options carry the schema's text
//! (`"check": "<xs:schema ...>"`; the generator reads the project's file),
//! the format reads it into the engine's one rule set (describe.rs's
//! `Described`: an XSD for XML, xsd.rs), and the whole document is walked
//! by it: a record's required fields are there, an element the schema
//! allows once is there once, and every value is of its kind (an integer,
//! a decimal within its digits, a date, a date and time, true or false),
//! one of an enumeration's values, or a text no longer than its longest.
//! What breaks it stops the read with where it is: the path to the record,
//! a record of a list with its place among its kind (`shop/order[2]`).

use crate::describe::{Column, Described};
use crate::doc::{local, Document, Form, Node};
use crate::format::Format;
use crate::tree::{self, Tree};
use crate::{catalog, FormatInfo, Nodes, Problem, ProblemKind};

/// The rules the options' `check` gives, read by `format`; none without one.
/// A format without schema kinds refuses it, as the checker does.
pub fn rules(format: &dyn Format, info: &FormatInfo, options: &[(String, Tree)]) -> Result<Option<Vec<Described>>, Problem> {
    let Some((_, given)) = options.iter().find(|(n, _)| n == "check") else { return Ok(None) };
    if info.schema_kinds.is_empty() {
        let with: Vec<String> = catalog().iter().filter(|f| !f.schema_kinds.is_empty()).map(|f| format!("{} ({})", f.name, f.schema_kinds.join(", "))).collect();
        return Err(Problem::new(ProblemKind::BadOptions, format!("{} has no schemas to check a file against; the formats with schemas are {}", info.title, with.join(", "))));
    }
    let Tree::Text(text) = given else {
        return Err(Problem::new(ProblemKind::BadOptions, format!("the option 'check' of {} is the text of a schema", info.name)));
    };
    format.schema(text).map(Some)
}

fn broken(text: String) -> Problem {
    Problem::new(ProblemKind::NotFitting, text)
}

/// Checks `doc` against `rules`: the first thing that breaks them, else nothing.
pub fn document(doc: &Document, info: &FormatInfo, rules: &[Described]) -> Result<(), Problem> {
    match info.nodes {
        // The root element is a table the schema declares by its name.
        Nodes::Elements => {
            let Some(root) = doc.top.nodes().next() else { return Ok(()) };
            let name = local(&root.name);
            match rules.iter().find(|t| t.path == name) {
                Some(table) => record(root, table, name, info, rules),
                None => Err(broken(format!("the {}'s root is <{}>, which the schema does not declare", info.title, root.name))),
            }
        }
        // The file's value is the schema's first table.
        _ => match rules.first() {
            Some(table) => record(&doc.top, table, "", info, rules),
            None => Ok(()),
        },
    }
}

/// Where a column is found in a record: attributes or nodes.
enum Found<'a> {
    Attribute(&'a str),
    Nodes(Vec<&'a Node>),
}

/// The nodes `node` holds of `name`, a list's items in its place.
fn named<'a>(node: &'a Node, name: &str) -> Vec<&'a Node> {
    let mut out = Vec::new();
    for child in node.nodes().filter(|c| local(&c.name) == name) {
        if child.form == Form::List {
            out.extend(child.nodes());
        } else {
            out.push(child);
        }
    }
    out
}

fn found<'a>(node: &'a Node, column: &Column, info: &FormatInfo) -> Found<'a> {
    if let Some(name) = column.column.strip_prefix('@') {
        return node.attribute(name).map_or(Found::Nodes(Vec::new()), Found::Attribute);
    }
    let nodes = named(node, &column.column);
    if nodes.is_empty() && info.attributes {
        if let Some(value) = node.attribute(&column.column) {
            return Found::Attribute(value);
        }
    }
    Found::Nodes(nodes)
}

/// Checks one record, at `at`, by its table, then the records inside it.
fn record(node: &Node, table: &Described, at: &str, info: &FormatInfo, rules: &[Described]) -> Result<(), Problem> {
    let place = |text: String| broken(if at.is_empty() { text } else { format!("{at}: {text}") });
    for column in &table.columns {
        let name = &column.name;
        let nodes = match found(node, column, info) {
            Found::Attribute(value) => {
                value_of(value, column).map_err(|why| place(format!("'{name}' {why}")))?;
                continue;
            }
            Found::Nodes(nodes) => nodes,
        };
        if nodes.is_empty() {
            if column.required {
                return Err(place(format!("'{name}' is missing, which the schema requires")));
            }
            continue;
        }
        if nodes.len() > 1 && !column.list {
            return Err(place(format!("'{name}' is there {} times, which the schema allows once", nodes.len())));
        }
        if column.kind == "record" {
            inner(&nodes, table, column, at, info, rules)?;
            continue;
        }
        for item in nodes {
            for value in values(item, column.list) {
                value_of(&value, column).map_err(|why| place(format!("'{name}' {why}")))?;
            }
        }
    }
    Ok(())
}

/// The records a record column holds, each checked by its own table. The
/// steps from the column to them are the table's path past the column's
/// (`lines/line`: the items of one `lines`); a table that refers back (a
/// type that holds itself) is reached by its last step.
fn inner(nodes: &[&Node], table: &Described, column: &Column, at: &str, info: &FormatInfo, rules: &[Described]) -> Result<(), Problem> {
    let Some(path) = &column.table else { return Ok(()) };
    let Some(rules_of) = rules.iter().find(|t| t.path == *path) else { return Ok(()) };
    let own = format!("{}/{}", table.path, column.column);
    let steps: Vec<&str> = match path.strip_prefix(&format!("{own}/")) {
        Some(rest) => rest.split('/').collect(),
        None if *path == own => Vec::new(),
        None => match path.rsplit('/').next() {
            Some(last) if last != column.column => vec![last],
            _ => Vec::new(),
        },
    };
    let base = if at.is_empty() { column.column.clone() } else { format!("{at}/{}", column.column) };
    // The column's own nodes are the records, or hold them past more steps.
    let mut level: Vec<(String, &Node)> = places(&base, nodes, column.list && steps.is_empty());
    for (i, step) in steps.iter().enumerate() {
        let last = i + 1 == steps.len();
        let mut next = Vec::new();
        for (here, node) in &level {
            let found = named(node, step);
            next.extend(places(&format!("{here}/{step}"), &found, last));
        }
        level = next;
    }
    for (here, node) in level {
        record(node, rules_of, &here, info, rules)?;
    }
    Ok(())
}

/// Nodes with where each is: of a list, with its place among its kind.
fn places<'a>(path: &str, nodes: &[&'a Node], numbered: bool) -> Vec<(String, &'a Node)> {
    nodes.iter().enumerate().map(|(i, n)| (if numbered { format!("{path}[{}]", i + 1) } else { path.to_string() }, *n)).collect()
}

/// The values a node gives a value column: its own, or for a list held in
/// one node (`<tags><tag>a</tag></tags>`), its items'.
fn values(node: &Node, list: bool) -> Vec<String> {
    match &node.form {
        Form::Value(Tree::Text(t)) => vec![t.clone()],
        Form::Value(Tree::Null) => Vec::new(),
        Form::Value(v) => vec![tree::write(v)],
        Form::Element if list && node.has_nodes() && node.text().trim().is_empty() => node.nodes().map(Node::text).collect(),
        Form::Element => vec![node.text()],
        Form::Object | Form::List => Vec::new(),
    }
}

/// Whether a value is what its column says, else what it breaks.
fn value_of(value: &str, column: &Column) -> Result<(), String> {
    let says = |what: &str| Err(format!("is {}, which the schema says {what}", tree::write(&Tree::Text(value.to_string()))));
    let trimmed = value.trim();
    match column.kind.as_str() {
        "integer" if !is_integer(trimmed) => says("is an integer"),
        "decimal" => match column.digits {
            Some((total, scale)) => match digits(trimmed) {
                None => says("is a decimal"),
                Some((t, s)) if t > total || s > scale => says(&format!("has at most {total} digits, {scale} after the point")),
                Some(_) => Ok(()),
            },
            None if digits(trimmed).is_none() && !is_double(trimmed) => says("is a decimal"),
            None => Ok(()),
        },
        "date" if !is_date(trimmed) => says("is a date"),
        "datetime" if !is_date_time(trimmed) => says("is a date and time"),
        "bool" if !matches!(trimmed, "true" | "false" | "1" | "0") => says("is true or false"),
        "enum" if !column.values.iter().any(|v| v == value) => says(&format!("is {}", one_of(&column.values))),
        "text" => match column.size {
            Some(size) if value.chars().count() > size as usize => Err(format!("is {} characters long, which the schema says is at most {size}", value.chars().count())),
            _ => Ok(()),
        },
        _ => Ok(()),
    }
}

fn one_of(values: &[String]) -> String {
    match values {
        [] => "an enum value".into(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} or {last}", rest.join(", ")),
    }
}

fn all_digits(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit())
}

fn unsigned(text: &str) -> &str {
    text.strip_prefix(['-', '+']).unwrap_or(text)
}

/// An XSD integer: a sign, then digits.
fn is_integer(text: &str) -> bool {
    all_digits(unsigned(text))
}

/// An XSD decimal's digits in all and after the point (leading and
/// trailing zeros not counted), or none when it is no decimal.
fn digits(text: &str) -> Option<(u32, u32)> {
    let (whole, fraction) = unsigned(text).split_once('.').unwrap_or((unsigned(text), ""));
    if whole.is_empty() && fraction.is_empty() || !whole.bytes().chain(fraction.bytes()).all(|b| b.is_ascii_digit()) {
        return None;
    }
    let whole = whole.trim_start_matches('0');
    let fraction = fraction.trim_end_matches('0');
    let total = (whole.len() + fraction.len()).max(1);
    Some((total as u32, fraction.len() as u32))
}

/// An XSD double or float: a decimal with an exponent, or INF, -INF, NaN.
fn is_double(text: &str) -> bool {
    if matches!(text, "INF" | "+INF" | "-INF" | "NaN") {
        return true;
    }
    match text.split_once(['e', 'E']) {
        Some((mantissa, exponent)) => digits(mantissa).is_some() && is_integer(exponent),
        None => false,
    }
}

fn two(text: &str, max: u32) -> bool {
    text.len() == 2 && all_digits(text) && text.parse::<u32>().is_ok_and(|n| n <= max)
}

/// A time zone as XSD writes one: none, `Z`, or `+01:00`.
fn is_zone(text: &str) -> bool {
    match text.strip_prefix(['+', '-']) {
        _ if text.is_empty() || text == "Z" => true,
        Some(zone) => zone.split_once(':').is_some_and(|(h, m)| two(h, 14) && two(m, 59)),
        None => false,
    }
}

/// An XSD date: `2026-01-31`, with a time zone or none.
fn is_date(text: &str) -> bool {
    let b = text.as_bytes();
    text.is_ascii() && b.len() >= 10 && b[4] == b'-' && b[7] == b'-' && all_digits(&text[..4]) && two(&text[5..7], 12) && &text[5..7] != "00" && two(&text[8..10], 31) && &text[8..10] != "00" && is_zone(&text[10..])
}

/// An XSD date and time: `2026-01-31T10:00:00`, seconds with a fraction or
/// none, with a time zone or none.
fn is_date_time(text: &str) -> bool {
    let Some((date, time)) = text.split_once('T') else { return false };
    if !is_date(date) || date.len() != 10 || !time.is_ascii() || time.len() < 8 {
        return false;
    }
    let (clock, rest) = time.split_at(8);
    let parts: Vec<&str> = clock.split(':').collect();
    let [h, m, s] = parts.as_slice() else { return false };
    if !(two(h, 24) && two(m, 59) && two(s, 60)) {
        return false;
    }
    let rest = match rest.strip_prefix('.') {
        Some(fraction) => {
            let end = fraction.find(|c: char| !c.is_ascii_digit()).unwrap_or(fraction.len());
            if end == 0 {
                return false;
            }
            &fraction[end..]
        }
        None => rest,
    };
    is_zone(rest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xsd_values_are_read_as_xsd_writes_them() {
        assert!(is_integer("+007") && is_integer("-3") && !is_integer("1.0") && !is_integer(""));
        assert_eq!(digits("0012.500"), Some((3, 1)));
        assert_eq!(digits(".5"), Some((1, 1)));
        assert_eq!(digits("."), None);
        assert!(is_double("1.5E3") && is_double("-INF") && !is_double("1e"));
        assert!(is_date("2026-01-31") && is_date("2026-01-31Z") && is_date("2026-01-31+01:00") && !is_date("2026-13-01") && !is_date("31-01-2026"));
        assert!(is_date_time("2026-01-31T10:00:00") && is_date_time("2026-01-31T10:00:00.25Z") && !is_date_time("2026-01-31T10:00") && !is_date_time("2026-01-31 10:00:00"));
    }
}
