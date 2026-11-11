//! Mappings as every stack runs them (conformance/formats-mapping.json):
//! the same case read from JSON, XML and CSV gives one result, proving the
//! engine is one.

use clarity_formats::tree::{self, Tree};
use clarity_formats::{read_json, read_table, write_json, write_table};

fn cases(part: &str) -> Vec<Tree> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance/formats-mapping.json");
    let file = tree::parse(&std::fs::read_to_string(path).unwrap()).unwrap();
    match file.get(part) {
        Some(Tree::List(cases)) => cases.clone(),
        _ => panic!("formats-mapping.json has {part}"),
    }
}

fn text<'a>(case: &'a Tree, field: &str) -> &'a str {
    case.get(field).and_then(Tree::as_text).unwrap_or_else(|| panic!("a case has '{field}'"))
}

fn pairs<'a>(case: &'a Tree, field: &str) -> &'a [(String, Tree)] {
    match case.get(field) {
        Some(Tree::Object(pairs)) => pairs,
        _ => panic!("a case has '{field}'"),
    }
}

/// A table without its `#node` column, which each format writes its own way.
fn without_nodes(table: &Tree) -> Tree {
    let Some(Tree::List(columns)) = table.get("columns") else { panic!("no columns") };
    let at = columns.iter().position(|c| c.as_text() == Some("#node"));
    let mut out = table.clone();
    if let Some(at) = at {
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
    }
    out
}

#[test]
fn one_mapping_reads_alike_from_every_format() {
    for case in &cases("read") {
        let name = text(case, "name");
        let kind = tree::write(case.get("kind").unwrap());
        let options = tree::write(case.get("options").unwrap());
        for (format, input) in pairs(case, "inputs") {
            let got = read_json(format, input.as_text().unwrap().as_bytes(), &options, &kind);
            let error = case.get("error").or_else(|| case.get("errors").and_then(|e| e.get(format)));
            match (case.get("fitted"), error) {
                (Some(fitted), _) => assert_eq!(tree::parse(&got.unwrap_or_else(|p| panic!("{name} ({format}): {}", p.text))).unwrap(), *fitted, "{name} ({format})"),
                (_, Some(error)) => assert_eq!(got.map_err(|p| p.text), Err(error.as_text().unwrap().to_string()), "{name} ({format})"),
                _ => panic!("{name}: a case has fitted or error"),
            }
        }
    }
}

#[test]
fn one_mapping_writes_where_it_says_in_every_format() {
    for case in &cases("write") {
        let name = text(case, "name");
        let kind = tree::write(case.get("kind").unwrap());
        let options = tree::write(case.get("options").unwrap());
        for (format, written) in pairs(case, "written") {
            let got = write_json(format, text(case, "value"), &options, &kind).unwrap_or_else(|p| panic!("{name} ({format}): {}", p.text));
            assert_eq!(String::from_utf8(got).unwrap(), written.as_text().unwrap(), "{name} ({format})");
        }
    }
}

#[test]
fn one_mapping_gives_a_file_source_the_same_table_from_every_format() {
    for case in &cases("table") {
        let name = text(case, "name");
        let fields = tree::write(case.get("fields").unwrap());
        let options = tree::write(case.get("options").unwrap());
        for (format, input) in pairs(case, "inputs") {
            let input = input.as_text().unwrap().as_bytes();
            let got = read_table(format, input, &options, &fields).unwrap_or_else(|p| panic!("{name} ({format}): {}", p.text));
            assert_eq!(without_nodes(&tree::parse(&got).unwrap()), *case.get("table").unwrap(), "{name} ({format})");
            let written = write_table(format, input, &got, &options, &fields).unwrap_or_else(|p| panic!("{name} ({format}): {}", p.text));
            assert_eq!(read_table(format, &written, &options, &fields).unwrap(), got, "{name} ({format}): read back the same");
        }
    }
}

#[test]
fn a_nested_mapping_gives_several_tables_and_takes_them_back() {
    for case in &cases("tables") {
        let name = text(case, "name");
        let fields = tree::write(case.get("fields").unwrap());
        let options = tree::write(case.get("options").unwrap());
        let Some(Tree::List(expected)) = case.get("tables") else { panic!("{name} has tables") };
        for (format, input) in pairs(case, "inputs") {
            let input = input.as_text().unwrap();
            let got = read_table(format, input.as_bytes(), &options, &fields).unwrap_or_else(|p| panic!("{name} ({format}): {}", p.text));
            let Some(Tree::List(tables)) = tree::parse(&got).unwrap().get("tables").cloned() else { panic!("{name} ({format}): {got}") };
            let tables: Vec<Tree> = tables.iter().map(without_nodes).collect();
            assert_eq!(tables, *expected, "{name} ({format})");
            let written = write_table(format, input.as_bytes(), &got, &options, &fields).unwrap_or_else(|p| panic!("{name} ({format}): {}", p.text));
            assert_eq!(String::from_utf8(written).unwrap(), input, "{name} ({format}): written back unchanged");
        }
    }
}

const NESTED_XML: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<shop>\n  <orders>\n    <order number=\"A-1\"><note>keep</note><lines><line product=\"Tea\"/><line product=\"Coffee\"/></lines></order>\n    <order number=\"A-2\"><lines/></order>\n  </orders>\n</shop>\n";
const NESTED_FIELDS: &str = r#"[{"name":"number","kind":"text"},{"name":"lines","kind":"records","entity":"Line","fields":[{"name":"product","kind":"text"}],"base":{}}]"#;
const NESTED_OPTIONS: &str = r#"{"at":"shop/orders/order","fields":{"number":"@number"},"nested":{"lines":{"at":"lines/line","fields":{"product":"@product"}}}}"#;

#[test]
fn nested_rows_go_back_into_the_record_they_link_to() {
    let read = read_table("xml", NESTED_XML.as_bytes(), NESTED_OPTIONS, NESTED_FIELDS).unwrap();
    let tables = tree::parse(&read).unwrap();
    let Some(Tree::List(list)) = tables.get("tables") else { panic!("{read}") };
    let lines = &list[1];
    assert_eq!(lines.get("columns"), Some(&Tree::List(vec![Tree::Text("#node".into()), Tree::Text("#parent".into()), Tree::Text("product".into())])));
    // Coffee moves to A-2 and becomes Milk, Tea goes, a new line joins A-1.
    let edited = tree::write(&Tree::Object(vec![(
        "tables".into(),
        Tree::List(vec![
            list[0].clone(),
            Tree::Object(vec![
                ("name".into(), Tree::Text("lines".into())),
                ("columns".into(), lines.get("columns").unwrap().clone()),
                (
                    "rows".into(),
                    Tree::List(vec![
                        Tree::List(vec![Tree::Text("<line product=\"Coffee\"/>".into()), Tree::Number("2".into()), Tree::Text("Milk".into())]),
                        Tree::List(vec![Tree::Null, Tree::Number("1".into()), Tree::Text("Juice".into())]),
                        Tree::List(vec![Tree::Null, Tree::Number("9".into()), Tree::Text("Lost".into())]),
                    ]),
                ),
            ]),
        ]),
    )]));
    let written = write_table("xml", NESTED_XML.as_bytes(), &edited, NESTED_OPTIONS, NESTED_FIELDS).unwrap();
    assert_eq!(
        String::from_utf8(written).unwrap(),
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<shop>\n  <orders>\n    <order number=\"A-1\"><note>keep</note><lines><line product=\"Juice\"/></lines></order>\n    <order number=\"A-2\"><lines><line product=\"Milk\"/></lines></order>\n  </orders>\n</shop>\n"
    );
}

#[test]
fn a_new_file_gets_its_records_and_the_records_inside_them() {
    let tables = r##"{"tables":[{"name":"","columns":["#node","number"],"rows":[[null,"A-1"]]},{"name":"lines","entity":"Line","columns":["#node","#parent","product"],"rows":[[null,1,"Tea"]]}]}"##;
    let written = write_table("json", b"", tables, r#"{"at":"orders","nested":{"lines":{}}}"#, NESTED_FIELDS).unwrap();
    assert_eq!(String::from_utf8(written).unwrap(), r#"{"orders":[{"number":"A-1","lines":[{"product":"Tea"}]}]}"#);
}

#[test]
fn a_nested_key_that_holds_no_records_is_refused() {
    let got = read_table("json", b"[]", r#"{"nested":{"number":{}}}"#, NESTED_FIELDS).unwrap_err();
    assert_eq!(got.text, "the nested 'number' is no field of records among the fields given");
    let got = read_json("csv", b"a", r#"{"nested":{}}"#, r#"{"kind":"text"}"#).unwrap_err();
    assert_eq!(got.text, "csv has no option 'nested'; it has the options separator, noHeader, quote, encoding, fields");
}
