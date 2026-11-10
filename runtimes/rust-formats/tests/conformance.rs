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
    assert_eq!(read_json("csv", b"x", "", kind).unwrap_err().text, "there is no format 'csv'");
    assert_eq!(read_json("json", b"\"x\"", r#"{"separator":";"}"#, kind).unwrap_err().text, "json has no option 'separator'; it has the options at");
    assert_eq!(read_json("json", b"\"x\"", r#"{"at":1}"#, kind).unwrap_err().text, "the option 'at' of json is text");
    assert_eq!(read_json("json", b"\"x\"", "", kind).unwrap(), "\"x\"");
    assert_eq!(read_json("json", b"[1]", "", kind).unwrap_err().text, "the JSON is text, not [1]");
}

#[test]
fn the_catalog_lists_json_with_its_options() {
    assert_eq!(clarity_formats::catalog_json(), r#"[{"name":"json","title":"JSON","text":true,"nests":true,"tables":"one","options":[{"name":"at","kind":"text","label":"formatOptionAt","reads":true,"writes":false}]}]"#);
}
