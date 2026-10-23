use clarity_spec::model::{FieldType, Literal};
use clarity_spec::parser::parse;

fn parse_ok(source: &str) -> clarity_spec::model::SpecFile {
    let (file, diagnostics) = parse(source, "test.clarity");
    assert!(
        diagnostics.is_empty(),
        "unexpected diagnostics: {diagnostics:#?}"
    );
    file
}

fn messages(source: &str) -> Vec<String> {
    let (_, diagnostics) = parse(source, "test.clarity");
    diagnostics.into_iter().map(|d| d.message).collect()
}

#[test]
fn parses_module_and_uses() {
    let file = parse_ok("module sales\nuse customers\nuse products\n");
    assert_eq!(file.module.unwrap().name, "sales");
    let uses: Vec<_> = file.uses.iter().map(|u| u.name.as_str()).collect();
    assert_eq!(uses, ["customers", "products"]);
}

#[test]
fn requires_module_declaration() {
    assert!(messages("entity A\n    x: text\n")[0].contains("module"));
    assert!(messages("")[0].contains("module"));
}

#[test]
fn module_must_come_first() {
    let msgs = messages("use x\nmodule a\n");
    assert!(msgs.iter().any(|m| m.contains("first declaration")));
}

#[test]
fn parses_enum_with_labels() {
    let file =
        parse_ok("module m\nenum Kind\n    Private label \"Private person\"\n    Business\n");
    let variants = &file.enums[0].variants;
    assert_eq!(variants[0].label, "Private person");
    assert_eq!(variants[1].label, "Business");
}

#[test]
fn parses_field_types() {
    let file = parse_ok(
        "module m
entity E
    a: text
    b: text(20)
    c: decimal(10, 2)
    d: money
    e: ref Other
    f: list<Line>
    g: Status
    h: datetime
",
    );
    let types: Vec<_> = file.entities[0].fields.iter().map(|f| &f.ty).collect();
    assert_eq!(types[0], &FieldType::Text { max_length: None });
    assert_eq!(
        types[1],
        &FieldType::Text {
            max_length: Some(20)
        }
    );
    assert_eq!(
        types[2],
        &FieldType::Decimal {
            precision: 10,
            scale: 2
        }
    );
    assert_eq!(types[3], &FieldType::Money);
    assert!(matches!(types[4], FieldType::Ref { entity } if entity.name == "Other"));
    assert!(matches!(types[5], FieldType::List { item } if item.name == "Line"));
    assert!(matches!(types[6], FieldType::Enum { name } if name.name == "Status"));
    assert_eq!(types[7], &FieldType::DateTime);
}

#[test]
fn parses_modifiers_and_resolves_label() {
    let file = parse_ok(
        "module m
entity E
    credit_limit: money, required, min 0, max 1000.50, default 0, help \"Max credit\"
    email: email, unique, label \"E-mail\"
    active: bool, default true
",
    );
    let fields = &file.entities[0].fields;
    let m = &fields[0].modifiers;
    assert!(m.required);
    assert_eq!(m.min, Some(Literal::Number("0".into())));
    assert_eq!(m.max, Some(Literal::Number("1000.50".into())));
    assert_eq!(m.help.as_deref(), Some("Max credit"));
    assert_eq!(fields[0].label, "Credit limit");
    assert_eq!(fields[1].label, "E-mail");
    assert_eq!(fields[2].modifiers.default, Some(Literal::Bool(true)));
}

#[test]
fn reports_bad_fields_and_keeps_going() {
    let msgs = messages(
        "module m
entity E
    a text
    b: text, sorted
    c: text, required, required
    d: decimal
    e: text
",
    );
    assert_eq!(msgs.len(), 4, "{msgs:#?}");
    assert!(msgs[0].contains("':'"));
    assert!(msgs[1].contains("unknown modifier 'sorted'"));
    assert!(msgs[2].contains("duplicate modifier 'required'"));
    assert!(msgs[3].contains("precision"));
}

#[test]
fn block_spans_cover_children() {
    let source = "module m\nentity E\n    a: text\n    b: text\n";
    let file = parse_ok(source);
    let span = file.entities[0].span;
    assert_eq!(
        &source[span.start as usize..span.end as usize],
        "entity E\n    a: text\n    b: text"
    );
}

#[test]
fn windows_are_pages_not_screens() {
    let msgs = messages("module m\nscreen F: form Customer\n");
    assert!(msgs[0].contains("expected 'module', 'use', 'source', 'enum', 'entity', 'page', 'app', 'theme', 'code', 'function', 'action', 'engine', 'settings', 'cli', 'command', 'part' or 'library'"), "{msgs:#?}");
}

#[test]
fn reports_inconsistent_dedent() {
    let msgs = messages("module m\nentity E\n        a: text\n    b: text\n");
    assert!(msgs.iter().any(|m| m.contains("indentation")), "{msgs:#?}");
}

#[test]
fn reports_children_where_none_are_allowed() {
    let msgs = messages("module m\nentity E\n    a: text\n        b: text\n");
    assert!(msgs[0].contains("unexpected indented block"));
}

#[test]
fn diagnostics_carry_path_and_position() {
    let (_, diagnostics) = parse("module m\nentity E\n    a text\n", "spec/m.clarity");
    assert_eq!(diagnostics[0].path, "spec/m.clarity");
    assert_eq!(diagnostics[0].span.line, 3);
    assert_eq!(diagnostics[0].span.col, 7);
}
