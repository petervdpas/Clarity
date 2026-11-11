//! The cases every stack shares (conformance/formats-json-*.json), run on
//! the one implementation they all call: what is read is fitted word for
//! word, what is written comes out byte for byte.

use clarity_formats::{read_json, tree, write_json};

fn cases(name: &str) -> Vec<tree::Tree> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance").join(name);
    let text = std::fs::read_to_string(&path).expect("the conformance file reads");
    match tree::parse(&text).expect("it is JSON").get("cases") {
        Some(tree::Tree::List(cases)) => cases.clone(),
        _ => panic!("{name} has cases"),
    }
}

fn text<'a>(case: &'a tree::Tree, field: &str) -> &'a str {
    case.get(field).and_then(tree::Tree::as_text).unwrap_or_else(|| panic!("a case has '{field}'"))
}

#[test]
fn json_is_read_and_fitted_as_every_stack_reads_it() {
    let cases = cases("formats-json-read.json");
    assert!(cases.len() >= 29);
    for case in &cases {
        let name = text(case, "name");
        let kind = tree::write(case.get("kind").unwrap());
        let options = case.get("options").map(tree::write).unwrap_or_default();
        let got = read_json("json", text(case, "input").as_bytes(), &options, &kind);
        match (case.get("fitted"), case.get("error")) {
            (Some(fitted), _) => {
                let got = got.unwrap_or_else(|p| panic!("{name}: {}", p.text));
                assert_eq!(tree::parse(&got).unwrap(), *fitted, "{name}");
            }
            (_, Some(error)) => assert_eq!(got.map_err(|p| p.text), Err(error.as_text().unwrap().to_string()), "{name}"),
            _ => panic!("{name}: a case has fitted or error"),
        }
    }
}

#[test]
fn json_is_written_byte_for_byte_as_every_stack_writes_it() {
    for case in &cases("formats-json-write.json") {
        let name = text(case, "name");
        let kind = tree::write(case.get("kind").unwrap());
        let got = write_json("json", text(case, "value"), "", &kind).unwrap_or_else(|p| panic!("{name}: {}", p.text));
        assert_eq!(String::from_utf8(got).unwrap(), text(case, "written"), "{name}");
    }
}

#[test]
fn an_option_or_format_that_is_not_there_is_refused() {
    let kind = r#"{"kind":"text"}"#;
    assert_eq!(read_json("toml", b"x", "", kind).unwrap_err().text, "there is no format 'toml'");
    assert_eq!(read_json("csv", b"a", r#"{"separator":";;"}"#, r#"{"kind":"records","entity":"A","fields":[],"base":{}}"#).unwrap_err().text, "the option 'separator' of csv is one character, not \";;\"");
    assert_eq!(read_json("csv", b"a", r#"{"encoding":"ebcdic"}"#, r#"{"kind":"records","entity":"A","fields":[],"base":{}}"#).unwrap_err().text, "csv reads and writes utf-8, windows-1252 or iso-8859-1, not \"ebcdic\"");
    assert_eq!(read_json("json", b"\"x\"", r#"{"separator":";"}"#, kind).unwrap_err().text, "json has no option 'separator'; it has the options at, fields, nested");
    assert_eq!(read_json("json", b"\"x\"", r#"{"at":1}"#, kind).unwrap_err().text, "the option 'at' of json is text");
    assert_eq!(read_json("json", b"\"x\"", "", kind).unwrap(), "\"x\"");
    assert_eq!(read_json("json", b"[1]", "", kind).unwrap_err().text, "the JSON is text, not [1]");
}

#[test]
fn the_catalog_lists_its_formats_with_their_options() {
    let catalog = clarity_formats::catalog_json();
    assert!(catalog.starts_with(r#"[{"name":"json","title":"JSON","text":true,"nests":true,"tables":"one","options":[{"name":"at","kind":"text","label":"formatOptionAt","reads":true,"writes":true,"requiredWriting":false}],"nodes":"values","attributes":false,"mapping":["fields","nested"]},{"name":"csv","title":"CSV","text":true,"nests":false"#), "{catalog}");
    assert!(catalog.contains(r#""nodes":"elements","attributes":true,"mapping":["fields","nested","check"]},"#), "{catalog}");
    assert!(catalog.ends_with(r#"{"name":"yaml","title":"YAML","text":true,"nests":true,"tables":"one","options":[{"name":"at","kind":"text","label":"formatOptionAt","reads":true,"writes":true,"requiredWriting":false}],"nodes":"values","attributes":false,"mapping":["fields","nested"]}]"#), "{catalog}");
    assert!(catalog.contains(r#"{"name":"noHeader","kind":"bool","label":"formatOptionNoHeader","reads":true,"writes":true,"requiredWriting":false}"#), "{catalog}");
}

fn csv_cases(part: &str) -> Vec<tree::Tree> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance/formats-csv.json");
    let file = tree::parse(&std::fs::read_to_string(path).unwrap()).unwrap();
    match file.get(part) {
        Some(tree::Tree::List(cases)) => cases.clone(),
        _ => panic!("formats-csv.json has {part}"),
    }
}

#[test]
fn csv_is_read_and_written_as_every_stack_does() {
    for case in &csv_cases("read") {
        let name = text(case, "name");
        let kind = tree::write(case.get("kind").unwrap());
        let options = case.get("options").map(tree::write).unwrap_or_default();
        let got = read_json("csv", text(case, "input").as_bytes(), &options, &kind);
        match (case.get("fitted"), case.get("error")) {
            (Some(fitted), _) => assert_eq!(tree::parse(&got.unwrap_or_else(|p| panic!("{name}: {}", p.text))).unwrap(), *fitted, "{name}"),
            (_, Some(error)) => assert_eq!(got.map_err(|p| p.text), Err(error.as_text().unwrap().to_string()), "{name}"),
            _ => panic!("{name}"),
        }
    }
    for case in &csv_cases("write") {
        let name = text(case, "name");
        let kind = tree::write(case.get("kind").unwrap());
        let options = case.get("options").map(tree::write).unwrap_or_default();
        let got = write_json("csv", text(case, "value"), &options, &kind).unwrap_or_else(|p| panic!("{name}: {}", p.text));
        assert_eq!(String::from_utf8(got).unwrap(), text(case, "written"), "{name}");
    }
}

fn table_cases(part: &str) -> Vec<tree::Tree> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance/formats-table.json");
    let file = tree::parse(&std::fs::read_to_string(path).unwrap()).unwrap();
    match file.get(part) {
        Some(tree::Tree::List(cases)) => cases.clone(),
        _ => panic!("formats-table.json has {part}"),
    }
}

#[test]
fn a_file_is_read_and_written_as_a_table_as_every_stack_does() {
    for case in &table_cases("read") {
        let name = text(case, "name");
        let options = case.get("options").map(tree::write).unwrap_or_default();
        let fields = tree::write(case.get("fields").unwrap());
        let got = clarity_formats::read_table(text(case, "format"), text(case, "input").as_bytes(), &options, &fields);
        match (case.get("table"), case.get("error")) {
            (Some(table), _) => assert_eq!(tree::parse(&got.unwrap_or_else(|p| panic!("{name}: {}", p.text))).unwrap(), *table, "{name}"),
            (_, Some(error)) => assert_eq!(got.map_err(|p| p.text), Err(error.as_text().unwrap().to_string()), "{name}"),
            _ => panic!("{name}"),
        }
    }
    for case in &table_cases("write") {
        let name = text(case, "name");
        let options = case.get("options").map(tree::write).unwrap_or_default();
        let fields = tree::write(case.get("fields").unwrap());
        let original = case.get("original").and_then(|o| o.as_text()).unwrap_or("");
        let got = clarity_formats::write_table(text(case, "format"), original.as_bytes(), &tree::write(case.get("table").unwrap()), &options, &fields).unwrap_or_else(|p| panic!("{name}: {}", p.text));
        assert_eq!(String::from_utf8(got).unwrap(), text(case, "written"), "{name}");
    }
}
