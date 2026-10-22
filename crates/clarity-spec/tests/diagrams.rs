//! Diagrams (DG1, DESIGN section 5.13): Mermaid text read into what Clarity
//! maps it to. An ERD's entities become entities, its attributes fields in
//! Clarity's types, and its relationships refs and lists.

use clarity_spec::Severity;
use clarity_spec::diagrams::{self, Cardinality, DiagramKind};

const SHOP: &str = "erDiagram
    %% the shop's data
    Customer {
        string name \"required\"
        varchar(80) email UK
        int id PK
    }
    Order {
        varchar(12) number UK \"required\"
        date ordered_on
        decimal total \"decimal(10, 2), computed\"
    }
    Customer ||--o{ Order : places
    Order }o..|| Product : \"lists\"
";

fn errors(problems: &[clarity_spec::Diagnostic]) -> Vec<String> {
    problems.iter().filter(|p| p.severity == Severity::Error).map(|p| p.message.clone()).collect()
}

#[test]
fn the_kinds_are_listed_with_their_mermaid_header() {
    let kinds = diagrams::kinds();
    let headers: Vec<(&DiagramKind, &str)> = kinds.iter().map(|k| (&k.kind, k.header.as_str())).collect();
    assert_eq!(
        headers,
        vec![
            (&DiagramKind::Erd, "erDiagram"),
            (&DiagramKind::Class, "classDiagram"),
            (&DiagramKind::State, "stateDiagram-v2"),
            (&DiagramKind::Sequence, "sequenceDiagram"),
            (&DiagramKind::Activity, "flowchart"),
            (&DiagramKind::Architecture, "architecture-beta"),
        ]
    );
}

#[test]
fn the_kind_is_read_from_the_header_after_frontmatter_and_comments() {
    assert_eq!(diagrams::kind_of("---\ntitle: Shop\n---\n%% a note\n\nerDiagram\n"), Some(DiagramKind::Erd));
    assert_eq!(diagrams::kind_of("classDiagram\n    class A"), Some(DiagramKind::Class));
    assert_eq!(diagrams::kind_of("stateDiagram\n"), Some(DiagramKind::State));
    assert_eq!(diagrams::kind_of("graph TD\n"), Some(DiagramKind::Activity));
    assert_eq!(diagrams::kind_of("gantt\n"), None);
}

#[test]
fn an_erd_gives_entities_with_fields_in_clarity_types() {
    let (diagram, problems) = diagrams::parse(SHOP);
    assert_eq!(errors(&problems), Vec::<String>::new());
    assert_eq!(diagram.kind, Some(DiagramKind::Erd));
    let names: Vec<&str> = diagram.entities.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, vec!["Customer", "Order", "Product"]);

    let customer = &diagram.entities[0];
    let fields: Vec<(&str, &str)> = customer.fields.iter().map(|f| (f.name.as_str(), f.type_text.as_str())).collect();
    assert_eq!(fields, vec![("name", "text"), ("email", "text(80)")], "the key named id is Clarity's own");
    assert!(customer.fields[0].modifiers.required);
    assert!(customer.fields[1].modifiers.unique);
    assert_eq!((customer.span.line, customer.fields[1].span.line), (3, 5));

    let order = &diagram.entities[1];
    let total = order.fields.iter().find(|f| f.name == "total").unwrap();
    assert_eq!(total.type_text, "decimal(10, 2)", "the comment gives the type Mermaid cannot write");
    assert!(total.modifiers.computed);
    let number = &order.fields[0];
    assert!(number.modifiers.unique && number.modifiers.required);

    assert!(diagram.entities[2].fields.is_empty(), "an entity named only in a relationship has no fields");
}

#[test]
fn relationships_keep_both_ends_and_whether_they_identify() {
    let (diagram, _) = diagrams::parse(SHOP);
    let places = &diagram.relations[0];
    assert_eq!((places.left.as_str(), places.right.as_str()), ("Customer", "Order"));
    assert_eq!((places.left_count, places.right_count), (Cardinality::ExactlyOne, Cardinality::ZeroOrMore));
    assert!(places.identifying);
    assert_eq!(places.label.as_deref(), Some("places"));
    assert_eq!(places.span.line, 13);

    let lists = &diagram.relations[1];
    assert_eq!((lists.left_count, lists.right_count), (Cardinality::ZeroOrMore, Cardinality::ExactlyOne));
    assert!(!lists.identifying);
    assert_eq!(lists.label.as_deref(), Some("lists"));
    assert!(lists.left_count.is_many() && !lists.right_count.is_many());
}

#[test]
fn problems_are_placed_on_their_line_in_the_checkers_words() {
    let text = "erDiagram\n    Customer {\n        blob photo\n        decimal price\n    }\n    Customer ||~~o{ Order : places\n";
    let (diagram, problems) = diagrams::parse(text);
    let placed: Vec<(u32, Severity, &str)> = problems.iter().map(|p| (p.span.line, p.severity, p.message.as_str())).collect();
    assert_eq!(placed.len(), 3, "{placed:?}");
    assert_eq!((placed[0].0, placed[0].1), (3, Severity::Error));
    assert!(placed[0].2.contains("'blob'"), "{}", placed[0].2);
    assert_eq!((placed[1].0, placed[1].1), (4, Severity::Error));
    assert!(placed[1].2.contains("decimal(10, 2)"), "{}", placed[1].2);
    assert_eq!((placed[2].0, placed[2].1), (6, Severity::Error));
    assert_eq!(diagram.entities[0].fields.len(), 0, "a field in error is left out");
}

#[test]
fn foreign_type_words_are_read_as_clarity_words() {
    for (written, read) in [
        ("string", "text"),
        ("varchar", "text"),
        ("varchar(40)", "text(40)"),
        ("int", "integer"),
        ("long", "integer"),
        ("boolean", "bool"),
        ("timestamp", "datetime"),
        ("money", "money"),
        ("email", "email"),
        ("text(20)", "text(20)"),
    ] {
        let (diagram, problems) = diagrams::parse(&format!("erDiagram\n    A {{\n        {written} x\n    }}\n"));
        assert_eq!(errors(&problems), Vec::<String>::new(), "{written}");
        assert_eq!(diagram.entities[0].fields[0].type_text, read, "{written}");
    }
}

#[test]
fn other_kinds_are_only_drawn() {
    let (diagram, problems) = diagrams::parse("gantt\n    title A plan\n");
    assert_eq!(diagram.kind, None);
    assert!(problems.is_empty());
}
