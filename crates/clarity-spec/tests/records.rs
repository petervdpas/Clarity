//! Records that belong to no table (DESIGN 5.12): `record Shape` with its
//! fields, made with `new`, read and set by field, in lists, and given out
//! by an engine as JSON objects of their fields. App code uses them too:
//! window data, functions and actions, tables and trees of them.

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
    assert_has(&messages(&SHAPES.replace("    op: text\n", "    op: text\n    owner: ref Shape\n"), Severity::Error), "a reference names a stored entity; Shape is a record");
}

/// Window data, functions and handlers holding records, and a tree of them
/// arranged by key.
const OUTLINE: &str = "module outline

entity Project
    name: text(80)

record Node
    key: text
    parent: text
    label: text
    depth: integer
    project: ref Project

function node(key: text, parent: text, label: text) -> Node
    let n = new Node
    n.key = key
    n.parent = parent
    n.label = label
    return n

function nodes(projects: list<Project>) -> list<Node>
    let all: list<Node> = []
    all = all + [node(\"top\", \"\", \"Landscape\")]
    for p in projects
        let n = node(\"p\", \"top\", p.name)
        n.project = p
        all = all + [n]
    return all

page Outline: window
    on open: loadNodes
    data projects: list<Project>
    data nodes: list<Node>
    data picked: Node
    tree name nav at 1,1 span 4 rows 10 bind nodes key key parent parent show label on select: chosen
    table name grid at 5,1 span 8 rows 10 bind nodes columns label, depth on open: opened
    textbox name labelBox at 5,11 span 4 bind picked.label

code Outline
    handler loadNodes
        projects = find p in Project
        nodes = nodes(projects)
        nav.selected = nodes[1]

    handler chosen
        picked = nav.selected
        picked.depth = picked.depth + 1
        nav.selected = none

    handler opened
        for n in nodes
            if n.label == grid.selected.label and n.project != none
                message n.project.name
";

#[test]
fn app_code_holds_records() {
    let errors = messages(OUTLINE, Severity::Error);
    assert!(errors.is_empty(), "{errors:#?}");
    let project = load_sources([("outline.clarity", OUTLINE)]);
    assert!(project.entity("Node").is_some_and(|n| n.record), "a project finds its records by name");
    // The designer shows a list of records with sample rows, as an entity's.
    let rows = &clarity_spec::sample::sample_data(&project)["Node"];
    assert!(!rows.is_empty() && rows[0].contains_key("label"), "{rows:?}");
}

#[test]
fn records_in_app_code_are_still_never_stored() {
    let with = |code: &str| messages(&OUTLINE.replace("        picked.depth = picked.depth + 1\n", &format!("        picked.depth = picked.depth + 1\n{code}")), Severity::Error);
    assert_has(&with("        save picked\n"), "Node is a record, which is never stored");
    assert_has(&with("        delete nav.selected\n"), "Node is a record, which is never stored");
    assert_has(&with("        let all = find n in Node\n"), "Node is a record, which is never stored");
    // A record is a value, never none; only a part's selected row may be none.
    assert_has(&with("        if picked != none\n            message \"x\"\n"), "Node is a record, which is never none");
    assert_has(&with("        picked = none\n"), "Node is a record, which is never none");
}

#[test]
fn a_table_of_records_is_not_edited_or_reordered() {
    let edited = OUTLINE.replace("columns label, depth on open", "columns label, depth edit label on open");
    assert_has(&messages(&edited, Severity::Error), "Node is a record, which is never stored, so its rows are not edited in a table");
    let moved = OUTLINE.replace("columns label, depth on open", "columns label, depth reorder by depth on open");
    assert_has(&messages(&moved, Severity::Error), "Node is a record, which is never stored, so the person cannot put its rows in their own order");
}

#[test]
fn a_record_refers_to_stored_records_only() {
    let to_record = OUTLINE.replace("    project: ref Project\n", "    project: ref Project\n    above: ref Node\n");
    assert_has(&messages(&to_record, Severity::Error), "a reference names a stored entity; Node is a record");
    // An engine has no stored records to refer to.
    let engine = format!("{OUTLINE}\nengine Counter\n    function first() -> Node\n        return new Node\n");
    assert_has(&messages(&engine, Severity::Error), "record Node refers to stored records");
}

#[test]
fn a_record_and_an_entity_have_names_of_their_own() {
    let both = format!("{SHAPES}\nentity Shape\n    name: text\n");
    assert_has(&messages(&both, Severity::Error), "Shape");
}
