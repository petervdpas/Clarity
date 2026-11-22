//! The groups of field types every generator decides by, said once in the
//! spec (`FieldType::kept_as_text`, `shown_as_text`).

use clarity_spec::model::FieldType;

fn named(source: &str) -> Vec<(String, FieldType)> {
    let project = clarity_spec::load_converted([("shop.clarity", source)]);
    let entity = project.entity("Everything").expect("the entity");
    entity.fields.iter().map(|f| (f.name.name.clone(), f.ty.clone())).collect()
}

#[test]
fn the_types_kept_as_text_are_the_nine_and_a_slot_never_shows_a_secret() {
    let fields = named("module shop\n\nenum Size\n    Small\n\nentity Other\n    name: text\n\nentity Everything\n    text: text\n    email: email\n    url: url\n    color: color\n    file: file\n    count: integer\n    price: decimal(10, 2)\n    total: money\n    done: bool\n    day: date\n    moment: datetime\n    took: duration\n    photo: image\n    upload: blob\n    size: Size\n    other: ref Other\n    tags: list<text>\n");
    let kept: Vec<&str> = fields.iter().filter(|(_, t)| t.kept_as_text()).map(|(n, _)| n.as_str()).collect();
    assert_eq!(kept, ["text", "email", "url", "color", "file"]);
    // Settings' own types: folder, language, theme and secret are text too.
    for ty in [FieldType::Folder, FieldType::Language, FieldType::Theme, FieldType::Secret] {
        assert!(ty.kept_as_text(), "{ty:?}");
    }
    assert!(!FieldType::Secret.shown_as_text());
    for ty in [FieldType::Text { max_length: None }, FieldType::Email, FieldType::Url, FieldType::Color, FieldType::File, FieldType::Folder, FieldType::Language, FieldType::Theme] {
        assert!(ty.shown_as_text(), "{ty:?}");
    }
}

#[test]
fn every_plain_type_is_a_type_window_data_holds() {
    for ty in clarity_spec::code::PLAIN_TYPES {
        let source = format!("module m\n\npage Main: window\n    data held: {ty}\n");
        let project = clarity_spec::load_converted([("m.clarity", source.as_str())]);
        let errors: Vec<&str> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.as_str()).collect();
        assert!(errors.is_empty(), "{ty}: {errors:#?}");
    }
}

#[test]
fn a_type_reads_back_as_the_cases_say() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance/type-text.json");
    let cases: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    for c in cases["cases"].as_array().unwrap() {
        let source = format!("module m\n\nenum Size\n    Small\n\nentity Customer\n    name: text\n\nentity Holder\n    held: {}\n", c["written"].as_str().unwrap());
        let project = clarity_spec::load_converted([("m.clarity", source.as_str())]);
        let errors: Vec<&str> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.as_str()).collect();
        assert!(errors.is_empty(), "{c}: {errors:#?}");
        let held = &project.entity("Holder").unwrap().field("held").unwrap().ty;
        assert_eq!(clarity_spec::dictionary::type_text(held), c["text"].as_str().unwrap(), "{c}");
    }
}
