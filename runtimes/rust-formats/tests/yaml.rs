//! YAML in code as every stack reads and writes it (conformance/formats-yaml.json).

use clarity_formats::{read_json, tree, write_json};

fn cases(part: &str) -> Vec<tree::Tree> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance/formats-yaml.json");
    let file = tree::parse(&std::fs::read_to_string(path).unwrap()).unwrap();
    match file.get(part) {
        Some(tree::Tree::List(cases)) => cases.clone(),
        _ => panic!("formats-yaml.json has {part}"),
    }
}

fn text<'a>(case: &'a tree::Tree, field: &str) -> &'a str {
    case.get(field).and_then(tree::Tree::as_text).unwrap_or_else(|| panic!("a case has '{field}'"))
}

fn options(case: &tree::Tree) -> String {
    case.get("options").map(tree::write).unwrap_or_default()
}

#[test]
fn yaml_is_read_as_every_stack_reads_it() {
    for case in &cases("read") {
        let name = text(case, "name");
        let got = read_json("yaml", text(case, "input").as_bytes(), &options(case), &tree::write(case.get("kind").unwrap()));
        match (case.get("fitted"), case.get("error")) {
            (Some(fitted), _) => assert_eq!(tree::parse(&got.unwrap_or_else(|p| panic!("{name}: {}", p.text))).unwrap(), *fitted, "{name}"),
            (_, Some(error)) => assert_eq!(got.map_err(|p| p.text), Err(error.as_text().unwrap().to_string()), "{name}"),
            _ => panic!("{name}: a case has fitted or error"),
        }
    }
}

#[test]
fn yaml_is_written_as_every_stack_writes_it() {
    for case in &cases("write") {
        let name = text(case, "name");
        let got = write_json("yaml", text(case, "value"), &options(case), &tree::write(case.get("kind").unwrap()));
        match (case.get("written"), case.get("error")) {
            (Some(written), _) => assert_eq!(String::from_utf8(got.unwrap_or_else(|p| panic!("{name}: {}", p.text))).unwrap(), written.as_text().unwrap(), "{name}"),
            (_, Some(error)) => assert_eq!(got.map_err(|p| p.text), Err(error.as_text().unwrap().to_string()), "{name}"),
            _ => panic!("{name}: a case has written or error"),
        }
    }
}

#[test]
fn what_clarity_writes_in_yaml_it_reads_back_the_same() {
    for case in &cases("roundTrip") {
        let name = text(case, "name");
        let kind = tree::write(case.get("kind").unwrap());
        let written = write_json("yaml", text(case, "value"), &options(case), &kind).unwrap_or_else(|p| panic!("{name}: {}", p.text));
        let read = read_json("yaml", &written, &options(case), &kind).unwrap_or_else(|p| panic!("{name}: {}", p.text));
        assert_eq!(tree::parse(&read).unwrap(), tree::parse(text(case, "value")).unwrap(), "{name}: {}", String::from_utf8_lossy(&written));
    }
}

#[test]
fn bytes_that_are_not_utf8_are_no_yaml() {
    let kind = r#"{"kind":"text"}"#;
    assert_eq!(read_json("yaml", b"caf\xE9", "", kind).unwrap_err().text, "the text is no YAML");
    assert_eq!(read_json("yaml", "café".as_bytes(), "", kind).unwrap(), "\"café\"");
}
