//! An XML file as a file source's table (conformance/formats-xml-table.json),
//! run on the one implementation every stack calls.

use clarity_formats::{read_table, tree, write_table};

fn cases(part: &str) -> Vec<tree::Tree> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance/formats-xml-table.json");
    let file = tree::parse(&std::fs::read_to_string(path).unwrap()).unwrap();
    match file.get(part) {
        Some(tree::Tree::List(cases)) => cases.clone(),
        _ => panic!("formats-xml-table.json has {part}"),
    }
}

fn text<'a>(case: &'a tree::Tree, field: &str) -> &'a str {
    case.get(field).and_then(tree::Tree::as_text).unwrap_or_else(|| panic!("a case has '{field}'"))
}

#[test]
fn an_xml_file_is_read_as_a_table_as_every_stack_reads_it() {
    for case in &cases("read") {
        let name = text(case, "name");
        let options = case.get("options").map(tree::write).unwrap_or_default();
        let fields = tree::write(case.get("fields").unwrap());
        let got = read_table("xml", text(case, "input").as_bytes(), &options, &fields);
        match (case.get("table"), case.get("error")) {
            (Some(table), _) => assert_eq!(tree::parse(&got.unwrap_or_else(|p| panic!("{name}: {}", p.text))).unwrap(), *table, "{name}"),
            (_, Some(error)) => assert_eq!(got.map_err(|p| p.text), Err(error.as_text().unwrap().to_string()), "{name}"),
            _ => panic!("{name}"),
        }
    }
}

#[test]
fn a_table_is_written_back_into_the_whole_document_as_every_stack_writes_it() {
    for case in &cases("write") {
        let name = text(case, "name");
        let options = case.get("options").map(tree::write).unwrap_or_default();
        let fields = tree::write(case.get("fields").unwrap());
        let original = case.get("original").and_then(|o| o.as_text()).unwrap_or("");
        let got = write_table("xml", original.as_bytes(), &tree::write(case.get("table").unwrap()), &options, &fields).unwrap_or_else(|p| panic!("{name}: {}", p.text));
        assert_eq!(String::from_utf8(got).unwrap(), text(case, "written"), "{name}");
    }
}

#[test]
fn a_table_read_and_written_again_is_the_same_document() {
    let original = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!-- prices -->\n<shop>\n  <item code=\"1\" extra=\"keep\"><name>Tea</name><x:note xmlns:x=\"urn:x\">a</x:note></item>\n  <item code=\"2\"><name>Coffee</name></item>\n</shop>\n";
    let fields = r#"[{"name":"@code","kind":"integer"},{"name":"name","kind":"text"}]"#;
    let options = r#"{"at":"shop/item"}"#;
    let table = read_table("xml", original.as_bytes(), options, fields).unwrap();
    let written = write_table("xml", original.as_bytes(), &table, options, fields).unwrap();
    assert_eq!(String::from_utf8(written).unwrap(), original);
}

#[test]
fn the_declared_encoding_is_read_and_written_again() {
    let original = b"<?xml version=\"1.0\" encoding=\"Windows-1252\"?><list><a name=\"caf\xE9\"/></list>";
    let fields = r#"[{"name":"name","kind":"text"}]"#;
    let table = read_table("xml", original, "", fields).unwrap();
    assert_eq!(table, r##"{"columns":["#node","name"],"rows":[["<a name=\"café\"/>","café"]]}"##);
    let table = table.replace(r#""café"]"#, r#""thé €"]"#);
    let written = write_table("xml", original, &table, "", fields).unwrap();
    assert_eq!(written, b"<?xml version=\"1.0\" encoding=\"Windows-1252\"?><list><a name=\"th\xE9 \x80\"/></list>");

    let utf16: Vec<u8> = [0xFF, 0xFE].into_iter().chain("<?xml version=\"1.0\" encoding=\"UTF-16\"?><list><a>\u{263A}</a></list>".encode_utf16().flat_map(u16::to_le_bytes)).collect();
    let fields = r#"[{"name":".","kind":"text"}]"#;
    let table = read_table("xml", &utf16, "", fields).unwrap();
    assert_eq!(table, r##"{"columns":["#node","."],"rows":[["<a>☺</a>","☺"]]}"##);
    assert_eq!(write_table("xml", &utf16, &table, "", fields).unwrap(), utf16);

    assert_eq!(read_table("xml", b"<?xml version=\"1.0\" encoding=\"ebcdic\"?><a/>", "", fields).unwrap_err().text, "xml reads utf-8, utf-16, windows-1252 or iso-8859-1, not \"ebcdic\"");
}

#[test]
fn a_new_record_needs_at_to_name_its_element() {
    let fields = r#"[{"name":"name","kind":"text"}]"#;
    let table = r##"{"columns":["#node","name"],"rows":[[null,"x"]]}"##;
    assert_eq!(write_table("xml", b"<list/>", table, "", fields).unwrap_err().text, "an XML file source needs 'at' to name the element of a new record");
    assert_eq!(write_table("xml", b"", table, "", fields).unwrap_err().text, "an XML file source needs 'at' to name its elements");
}
