//! XML in code as every stack reads and writes it (conformance/formats-xml.json),
//! and the encodings, which a JSON file cannot carry as bytes.

use clarity_formats::{read_json, tree, write_json};

fn cases(part: &str) -> Vec<tree::Tree> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance/formats-xml.json");
    let file = tree::parse(&std::fs::read_to_string(path).unwrap()).unwrap();
    match file.get(part) {
        Some(tree::Tree::List(cases)) => cases.clone(),
        _ => panic!("formats-xml.json has {part}"),
    }
}

fn text<'a>(case: &'a tree::Tree, field: &str) -> &'a str {
    case.get(field).and_then(tree::Tree::as_text).unwrap_or_else(|| panic!("a case has '{field}'"))
}

fn options(case: &tree::Tree) -> String {
    case.get("options").map(tree::write).unwrap_or_default()
}

const ORDER: &str = r#"{"kind":"record","entity":"Order","fields":[{"name":"note","kind":"text"}],"base":{"id":0,"note":""}}"#;

#[test]
fn xml_is_read_as_every_stack_reads_it() {
    for case in &cases("read") {
        let name = text(case, "name");
        let got = read_json("xml", text(case, "input").as_bytes(), &options(case), &tree::write(case.get("kind").unwrap()));
        match (case.get("fitted"), case.get("error")) {
            (Some(fitted), _) => assert_eq!(tree::parse(&got.unwrap_or_else(|p| panic!("{name}: {}", p.text))).unwrap(), *fitted, "{name}"),
            (_, Some(error)) => assert_eq!(got.map_err(|p| p.text), Err(error.as_text().unwrap().to_string()), "{name}"),
            _ => panic!("{name}: a case has fitted or error"),
        }
    }
}

#[test]
fn xml_is_written_as_every_stack_writes_it() {
    for case in &cases("write") {
        let name = text(case, "name");
        let got = write_json("xml", text(case, "value"), &options(case), &tree::write(case.get("kind").unwrap()));
        match (case.get("written"), case.get("error")) {
            (Some(written), _) => assert_eq!(String::from_utf8(got.unwrap_or_else(|p| panic!("{name}: {}", p.text))).unwrap(), written.as_text().unwrap(), "{name}"),
            (_, Some(error)) => assert_eq!(got.map_err(|p| p.text), Err(error.as_text().unwrap().to_string()), "{name}"),
            _ => panic!("{name}: a case has written or error"),
        }
    }
}

#[test]
fn what_clarity_writes_it_reads_back_the_same() {
    for case in &cases("roundTrip") {
        let name = text(case, "name");
        let kind = tree::write(case.get("kind").unwrap());
        let written = write_json("xml", text(case, "value"), &options(case), &kind).unwrap_or_else(|p| panic!("{name}: {}", p.text));
        let at = tree::write(&tree::Tree::Object(vec![("at".into(), case.get("options").unwrap().get("at").unwrap().clone())]));
        let read = read_json("xml", &written, &at, &kind).unwrap_or_else(|p| panic!("{name}: {}", p.text));
        assert_eq!(tree::parse(&read).unwrap(), tree::parse(text(case, "value")).unwrap(), "{name}: {}", String::from_utf8_lossy(&written));
    }
}

#[test]
fn bytes_are_read_in_the_encoding_the_text_gives() {
    let read = |bytes: &[u8]| read_json("xml", bytes, "", ORDER).map_err(|p| p.text);
    assert_eq!(read(b"<?xml version='1.0' encoding='windows-1252'?><order><note>caf\xE9 \x80</note></order>"), Ok(r#"{"id":0,"note":"café €"}"#.into()));
    assert_eq!(read(b"<?xml version=\"1.0\" encoding=\"ISO-8859-1\"?><order note=\"\xE9\x80\"/>"), Ok("{\"id\":0,\"note\":\"é\u{80}\"}".into()));
    assert_eq!(read(b"\xEF\xBB\xBF<order note=\"\xC3\xA9\"/>"), Ok(r#"{"id":0,"note":"é"}"#.into()), "a UTF-8 byte order mark is dropped");
    let utf16 = |big: bool| -> Vec<u8> {
        let text = "<?xml version=\"1.0\" encoding=\"UTF-16\"?><order note=\"é\"/>";
        let mut bytes = if big { vec![0xFE, 0xFF] } else { vec![0xFF, 0xFE] };
        for unit in text.encode_utf16() {
            bytes.extend(if big { unit.to_be_bytes() } else { unit.to_le_bytes() });
        }
        bytes
    };
    assert_eq!(read(&utf16(false)), Ok(r#"{"id":0,"note":"é"}"#.into()));
    assert_eq!(read(&utf16(true)), Ok(r#"{"id":0,"note":"é"}"#.into()));
    assert_eq!(read(b"<order note=\"\xE9\"/>"), Err("the text is no XML".into()), "without a declaration it is UTF-8");
}

#[test]
fn bytes_are_written_in_the_encoding_named() {
    let options = r#"{"at":"order","encoding":"windows-1252"}"#;
    let written = write_json("xml", r#"{"note":"café € ā"}"#, options, ORDER).unwrap();
    assert_eq!(written, b"<?xml version=\"1.0\" encoding=\"windows-1252\"?><order><note>caf\xE9 \x80 &#257;</note></order>");
    assert_eq!(read_json("xml", &written, r#"{"at":"order"}"#, ORDER).unwrap(), r#"{"id":0,"note":"café € ā"}"#);
}
