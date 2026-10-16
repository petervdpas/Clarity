use clarity_spec::dictionary::FieldSpec;
use clarity_spec::edit::{Edit, apply};
use clarity_spec::model::{Literal, Modifiers};
use clarity_spec::parser::parse;

const FILE: &str = "module shop

use customers

enum Status
    Open
    Closed label \"Done\"

entity Order
    number: text(12), required // the order number
    total: money

page P: window
    title \"P\"
";

fn at(source: &str, needle: &str) -> u32 {
    let mut offset = 0;
    for line in source.split('\n') {
        let indent = line.len() - line.trim_start().len();
        if line[indent..].starts_with(needle) {
            return (offset + indent) as u32;
        }
        offset += line.len() + 1;
    }
    panic!("no line starts with {needle:?}")
}

fn edit(source: &str, edit: Edit) -> String {
    let text = apply(source, &edit).unwrap_or_else(|e| panic!("{e}")).text;
    let (_, diags) = parse(&text, "x.clarity");
    assert!(diags.is_empty(), "{diags:#?}\n{text}");
    text
}

fn error(source: &str, edit: Edit) -> String {
    apply(source, &edit).expect_err("edit should fail").0
}

fn spec(name: &str, type_text: &str) -> FieldSpec {
    FieldSpec {
        name: name.into(),
        type_text: type_text.into(),
        modifiers: Modifiers::default(),
    }
}

#[test]
fn adds_entities_and_enums_after_the_data() {
    let text = edit(FILE, Edit::AddEntity { name: "Customer".into() });
    assert!(
        text.contains("    total: money\n\nentity Customer\n    name: text(80)\n\npage P"),
        "{text}"
    );
    let text = edit(&text, Edit::AddEnum { name: "Kind".into() });
    assert!(text.contains("entity Customer\n    name: text(80)\n\nenum Kind\n    First\n"), "{text}");
    assert!(error(FILE, Edit::AddEntity { name: "Order".into() }).contains("already declared"));
}

#[test]
fn adds_updates_and_reorders_fields() {
    let order = at(FILE, "entity Order");
    let mut s = spec("placed_on", "date");
    s.modifiers.required = true;
    let text = edit(FILE, Edit::AddField { entity: order, spec: s });
    assert!(text.contains("    total: money\n    placed_on: date, required\n"), "{text}");

    let mut s = spec("number", "text(20)");
    s.modifiers.unique = true;
    s.modifiers.label = Some("Number".into());
    s.modifiers.default = Some(Literal::Str("N-1".into()));
    let text = edit(&text, Edit::UpdateField { item: at(&text, "number:"), spec: s });
    assert!(
        text.contains("    number: text(20), unique, default \"N-1\", label \"Number\" // the order number\n"),
        "{text}"
    );

    let text = edit(&text, Edit::MoveSibling { item: at(&text, "total:"), delta: -1 });
    assert!(text.contains("entity Order\n    total: money\n    number:"), "{text}");
}

#[test]
fn rejects_bad_fields() {
    let order = at(FILE, "entity Order");
    assert!(error(FILE, Edit::AddField { entity: order, spec: spec("total", "money") }).contains("already has a field"));
    assert!(error(FILE, Edit::AddField { entity: order, spec: spec("x", "decimal") }).contains("precision"));
    assert!(error(FILE, Edit::AddField { entity: order, spec: spec("two words", "text") }).contains("not a valid field name"));
}

#[test]
fn edits_variants() {
    let status = at(FILE, "enum Status");
    let text = edit(FILE, Edit::AddVariant { owner: status, name: "Shipped".into(), label: None });
    assert!(text.contains("    Closed label \"Done\"\n    Shipped\n"), "{text}");
    let text = edit(&text, Edit::UpdateVariant { item: at(&text, "Open"), name: "New".into(), label: Some("Newly opened".into()) });
    assert!(text.contains("    New label \"Newly opened\"\n"), "{text}");
}

#[test]
fn removes_fields_and_declarations() {
    let text = edit(FILE, Edit::Remove { item: at(FILE, "total:") });
    assert!(!text.contains("total"));
    let text = edit(&text, Edit::Remove { item: at(&text, "enum Status") });
    assert!(!text.contains("Status"), "{text}");
}

#[test]
fn sets_the_uses_of_a_module() {
    let text = edit(FILE, Edit::SetUses { uses: vec!["customers".into(), "sales".into()] });
    assert!(text.starts_with("module shop\n\nuse customers\nuse sales\n\nenum Status"), "{text:?}");
    let text = edit(&text, Edit::SetUses { uses: vec![] });
    assert!(text.starts_with("module shop\n\nenum Status"), "{text:?}");
    let text = edit(&text, Edit::SetUses { uses: vec!["customers".into()] });
    assert!(text.starts_with("module shop\n\nuse customers\n\nenum Status"), "{text:?}");
}
