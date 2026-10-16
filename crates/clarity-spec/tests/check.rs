use clarity_spec::{Severity, load_dir, load_sources};

fn diagnostics(sources: &[(&str, &str)]) -> Vec<(Severity, String)> {
    load_sources(sources.iter().copied())
        .diagnostics
        .into_iter()
        .map(|d| (d.severity, d.message))
        .collect()
}

fn errors(sources: &[(&str, &str)]) -> Vec<String> {
    diagnostics(sources)
        .into_iter()
        .filter(|(s, _)| *s == Severity::Error)
        .map(|(_, m)| m)
        .collect()
}

const CUSTOMERS: &str = "module customers
enum Kind
    Private
    Business
entity Customer
    name: text(80), required
    kind: Kind, default Private
    notes: list<Note>, owned
entity Note
    text: text
";

#[test]
fn example_project_is_clean() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/shop");
    let project = load_dir(std::path::Path::new(dir)).unwrap();
    assert_eq!(project.files.len(), 4);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
}

#[test]
fn resolves_types_within_a_module() {
    let errs = errors(&[(
        "c.clarity",
        "module customers
enum Kind
    A
entity Order
    x: text
entity Customer
    kind: Kind
    orders: list<Order>, owned
    missing: Nope
    wrong: ref Kind
",
    )]);
    assert_eq!(errs.len(), 2, "{errs:#?}");
    assert!(errs[0].contains("unknown enum 'Nope'"));
    assert!(errs[1].contains("'Kind' is an enum"));
}

#[test]
fn requires_use_for_other_modules() {
    let sales = "module sales
entity Order
    customer: ref Customer
";
    let errs = errors(&[("c.clarity", CUSTOMERS), ("s.clarity", sales)]);
    assert!(
        errs.iter().any(|e| e.contains("add 'use customers'")),
        "{errs:#?}"
    );
}

#[test]
fn resolves_through_use() {
    let sales = "module sales
use customers
entity Order
    customer: ref Customer
    kind: Kind
";
    let errs = errors(&[("c.clarity", CUSTOMERS), ("s.clarity", sales)]);
    assert!(errs.is_empty(), "{errs:#?}");
}

#[test]
fn reports_ambiguous_names() {
    let a = "module a\nentity Item\n    x: text\n";
    let b = "module b\nentity Item\n    y: text\n";
    let c = "module c\nuse a\nuse b\nentity Box\n    item: ref Item\n";
    let errs = errors(&[("a.clarity", a), ("b.clarity", b), ("c.clarity", c)]);
    assert_eq!(errs.len(), 1);
    assert!(errs[0].contains("ambiguous"));
}

#[test]
fn reports_duplicates() {
    let errs = errors(&[
        (
            "a.clarity",
            "module a
enum E
    X
    X
entity A
    f: text
    f: text
entity A
    g: text
",
        ),
        ("b.clarity", "module a\n"),
    ]);
    assert!(errs.iter().any(|e| e.contains("duplicate variant 'X'")));
    assert!(errs.iter().any(|e| e.contains("duplicate field 'f'")));
    assert!(errs.iter().any(|e| e.contains("'A' is already declared")));
    assert!(
        errs.iter()
            .any(|e| e.contains("module 'a' is already declared"))
    );
}

#[test]
fn reports_unknown_and_self_use() {
    let errs = errors(&[("a.clarity", "module a\nuse a\nuse nowhere\n")]);
    assert!(errs.iter().any(|e| e.contains("cannot use itself")));
    assert!(errs.iter().any(|e| e.contains("unknown module 'nowhere'")));
}

#[test]
fn checks_modifiers_against_types() {
    let errs = errors(&[(
        "a.clarity",
        "module a
enum Kind
    A
entity E
    a: text, owned
    b: integer, pattern \"x\"
    c: money, min 10, max 5
    d: bool, default 3
    e: Kind, default Z
    f: text, min 3
    g: money, computed, default 0
    h: decimal(2, 4)
    i: text(0)
",
    )]);
    let expected = [
        "'owned' only applies to lists",
        "'pattern' only applies to text",
        "'min' is greater than 'max'",
        "default value does not match",
        "'Z' is not a variant of 'Kind'",
        "'min' needs a number",
        "computed field cannot have a default",
        "decimal precision",
        "text length",
    ];
    for needle in expected {
        assert!(
            errs.iter().any(|e| e.contains(needle)),
            "missing '{needle}' in {errs:#?}"
        );
    }
    assert_eq!(errs.len(), expected.len(), "{errs:#?}");
}

#[test]
fn preview_samples_follow_the_spec() {
    let project = load_sources([(
        "c.clarity",
        "module c
enum Kind
    A
    B
entity Customer
    name: text(5), required
    kind: Kind, default B
    limit: money, min 0, max 100, default 5
entity Order
    customer: ref Customer
",
    )]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let preview = clarity_spec::Preview::new(project);
    let customers = &preview.samples["Customer"];
    assert_eq!(customers.len(), clarity_spec::sample::ROWS_PER_ENTITY);
    assert_eq!(customers[0]["kind"], "B", "first row uses the default");
    assert_eq!(
        customers[0]["limit"], "5.00",
        "money defaults get two decimals"
    );
    for row in customers {
        assert!(row["name"].as_str().unwrap().chars().count() <= 5);
        let limit: f64 = row["limit"].as_str().unwrap().parse().unwrap();
        assert!((0.0..=100.0).contains(&limit), "{limit} out of bounds");
    }
    let displays = &preview.displays["Customer"];
    assert_eq!(preview.samples["Order"][2]["customer"], displays[2]);
}

#[test]
fn finds_errors_an_edit_would_add() {
    let before = load_sources([("a.clarity", "module a\nentity E\n    x: text\n")]);
    let same = load_sources([("a.clarity", "module a\n\nentity E\n    x: text\n")]);
    assert!(clarity_spec::new_error(&before, &same).is_none());
    let worse = load_sources([("a.clarity", "module a\nentity E\n    x: Nope\n")]);
    let error = clarity_spec::new_error(&before, &worse).expect("new error");
    assert!(error.message.contains("unknown enum 'Nope'"));
}
