//! A mapping's `check` as every stack runs it (conformance/formats-check.json):
//! the schema's text comes with the options, the format reads it into the
//! engine's rules, and a file that breaks them stops the read with where.

use clarity_formats::tree::{self, Tree};
use clarity_formats::{read_json, read_table};

fn cases(part: &str) -> Vec<Tree> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance/formats-check.json");
    let file = tree::parse(&std::fs::read_to_string(path).unwrap()).unwrap();
    match file.get(part) {
        Some(Tree::List(cases)) => cases.clone(),
        _ => panic!("formats-check.json has {part}"),
    }
}

fn text<'a>(case: &'a Tree, field: &str) -> &'a str {
    case.get(field).and_then(Tree::as_text).unwrap_or_else(|| panic!("a case has '{field}'"))
}

#[test]
fn a_file_is_read_only_when_its_schema_allows_it() {
    let cases = cases("read");
    assert!(cases.len() >= 20);
    for case in &cases {
        let name = text(case, "name");
        let kind = tree::write(case.get("kind").unwrap());
        let options = tree::write(case.get("options").unwrap());
        let got = read_json(text(case, "format"), text(case, "input").as_bytes(), &options, &kind);
        match (case.get("fitted"), case.get("error")) {
            (Some(fitted), _) => assert_eq!(tree::parse(&got.unwrap_or_else(|p| panic!("{name}: {}", p.text))).unwrap(), *fitted, "{name}"),
            (_, Some(error)) => assert_eq!(got.map_err(|p| p.text), Err(error.as_text().unwrap().to_string()), "{name}"),
            _ => panic!("{name}: a case has fitted or error"),
        }
    }
}

/// A table without its `#node` column, which each format writes its own way.
fn without_nodes(table: &Tree) -> Tree {
    let Some(Tree::List(columns)) = table.get("columns") else { panic!("no columns") };
    let Some(at) = columns.iter().position(|c| c.as_text() == Some("#node")) else { return table.clone() };
    let mut out = table.clone();
    let mut columns = columns.clone();
    columns.remove(at);
    out.set("columns", Tree::List(columns));
    if let Some(Tree::List(rows)) = table.get("rows") {
        let rows = rows
            .iter()
            .map(|r| {
                let Tree::List(mut cells) = r.clone() else { panic!("a row") };
                cells.remove(at);
                Tree::List(cells)
            })
            .collect();
        out.set("rows", Tree::List(rows));
    }
    out
}

#[test]
fn a_file_source_is_checked_as_code_is() {
    for case in &cases("table") {
        let name = text(case, "name");
        let fields = tree::write(case.get("fields").unwrap());
        let options = tree::write(case.get("options").unwrap());
        let got = read_table(text(case, "format"), text(case, "input").as_bytes(), &options, &fields);
        match (case.get("table"), case.get("error")) {
            (Some(table), _) => assert_eq!(without_nodes(&tree::parse(&got.unwrap_or_else(|p| panic!("{name}: {}", p.text))).unwrap()), *table, "{name}"),
            (_, Some(error)) => assert_eq!(got.map_err(|p| p.text), Err(error.as_text().unwrap().to_string()), "{name}"),
            _ => panic!("{name}: a case has table or error"),
        }
    }
}
