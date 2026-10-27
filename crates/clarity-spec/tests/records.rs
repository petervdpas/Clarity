//! Records that belong to no table (DESIGN 5.12): `record Shape` with its
//! fields, made with `new`, read and set by field, in lists, and given out
//! by an engine as JSON objects of their fields. For now an engine uses
//! them; code outside an engine does not yet.

use clarity_spec::{Severity, load_sources};

const SHAPES: &str = "module play

record Shape
    op: text
    x: integer
    y: integer
    r: integer
    color: text, default \"accent\"

engine Shapes
    function dot(x: integer) -> Shape
        let s = new Shape
        s.op = \"circle\"
        s.x = x
        s.r = 5
        return s

    function all() -> list<Shape>
        return [dot(1), dot(2)] + [dot(3)]

    function rightmost(shapes: list<Shape>) -> integer
        let most = 0
        for s in shapes
            if s.x > most
                most = s.x
        return most
";

fn messages(source: &str, severity: Severity) -> Vec<String> {
    load_sources([("play.clarity", source)]).diagnostics.into_iter().filter(|d| d.severity == severity).map(|d| d.message).collect()
}

fn assert_has(messages: &[String], needle: &str) {
    assert!(messages.iter().any(|m| m.contains(needle)), "missing {needle:?} in {messages:#?}");
}

#[test]
fn a_record_is_declared_with_its_fields() {
    let project = load_sources([("play.clarity", SHAPES)]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let file = &project.files[0];
    assert!(file.entities.is_empty(), "a record is no entity");
    let shape = &file.records[0];
    assert!(shape.record);
    assert_eq!(shape.fields.iter().map(|f| f.name.name.as_str()).collect::<Vec<_>>(), ["op", "x", "y", "r", "color"]);
}

#[test]
fn a_record_is_never_stored() {
    let errors = |code: &str| messages(&SHAPES.replace("        return s\n", &format!("{code}        return s\n")), Severity::Error);
    assert_has(&errors("        save s\n"), "Shape is a record, which is never stored");
    let find = SHAPES.replace("        let most = 0\n", "        let found = find t in Shape\n        let most = 0\n");
    assert_has(&messages(&find, Severity::Error), "Shape is a record, which is never stored");
    assert_has(&messages(&SHAPES.replace("    op: text\n", "    op: text\n    owner: ref Shape\n"), Severity::Error), "a record's fields are values");
}

#[test]
fn code_outside_an_engine_does_not_have_records_yet() {
    let function = format!("{SHAPES}\nfunction size() -> integer\n    let s = new Shape\n    return s.r\n");
    assert_has(&messages(&function, Severity::Error), "record Shape is for engines for now");
    let param = format!("{SHAPES}\nfunction size(s: Shape) -> integer\n    return s.r\n");
    assert_has(&messages(&param, Severity::Error), "record Shape is for engines for now");
}

#[test]
fn a_record_and_an_entity_have_names_of_their_own() {
    let both = format!("{SHAPES}\nentity Shape\n    name: text\n");
    assert_has(&messages(&both, Severity::Error), "Shape");
}
