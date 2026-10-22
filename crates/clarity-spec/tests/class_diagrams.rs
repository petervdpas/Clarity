//! Class diagrams (DG1, DESIGN section 5.13): classes become entities,
//! `<<enumeration>>` enums, `<<engine>>` engines with their functions and
//! `<<module>>` a module's functions; associations, compositions and
//! aggregations become relations, as an ERD's do.

use clarity_spec::Severity;
use clarity_spec::diagrams::{self, Cardinality, DiagramKind};

const SHOP: &str = "classDiagram
    direction LR
    namespace sales {
        class Order {
            +String number
            +ordered_on: date, required
            +total: decimal(10, 2), computed
        }
        class OrderLine {
            +int quantity
        }
    }
    class Customer {
        +name: text(80), required
    }
    Customer : +boolean active
    class OrderStatus {
        <<enumeration>>
        Pending
        Approved
    }
    class Kind
    <<enumeration>> Kind
    Kind : Private
    Kind : Business
    class Board {
        <<engine>>
        +play(square: integer) text
        +winner() integer
    }
    class sales {
        <<module>>
        +order_total(lines: List~OrderLine~) money
        +within_credit(order: Order, limit: decimal) bool
    }
    Customer \"1\" --> \"*\" Order : places
    Order *-- OrderLine
    OrderLine --* Order
    Customer o-- Kind
    Order --> Customer
";

fn errors(problems: &[clarity_spec::Diagnostic]) -> Vec<String> {
    problems.iter().filter(|p| p.severity == Severity::Error).map(|p| p.message.clone()).collect()
}

#[test]
fn classes_become_entities_in_their_namespaces_module() {
    let (diagram, problems) = diagrams::parse(SHOP);
    assert_eq!(errors(&problems), Vec::<String>::new());
    assert_eq!(diagram.kind, Some(DiagramKind::Class));
    let entities: Vec<(&str, Option<&str>)> = diagram.entities.iter().map(|e| (e.name.as_str(), e.module.as_deref())).collect();
    assert_eq!(entities, vec![("Order", Some("sales")), ("OrderLine", Some("sales")), ("Customer", None)]);

    let order = &diagram.entities[0];
    let fields: Vec<(&str, &str)> = order.fields.iter().map(|f| (f.name.as_str(), f.type_text.as_str())).collect();
    assert_eq!(fields, vec![("number", "text"), ("ordered_on", "date"), ("total", "decimal(10, 2)")]);
    assert!(order.fields[1].modifiers.required && order.fields[2].modifiers.computed);
    assert_eq!(order.fields[0].span.line, 5);

    let customer = &diagram.entities[2];
    let fields: Vec<(&str, &str)> = customer.fields.iter().map(|f| (f.name.as_str(), f.type_text.as_str())).collect();
    assert_eq!(fields, vec![("name", "text(80)"), ("active", "bool")], "a member written beside the class joins it");
}

#[test]
fn an_enumeration_is_an_enum_with_its_values() {
    let (diagram, _) = diagrams::parse(SHOP);
    let enums: Vec<(&str, Vec<&str>)> =
        diagram.enums.iter().map(|e| (e.name.as_str(), e.values.iter().map(|v| v.name.as_str()).collect())).collect();
    assert_eq!(enums, vec![("OrderStatus", vec!["Pending", "Approved"]), ("Kind", vec!["Private", "Business"])]);
    assert_eq!(diagram.enums[0].values[1].span.line, 20);
}

#[test]
fn an_engine_and_a_module_have_their_functions() {
    let (diagram, _) = diagrams::parse(SHOP);
    let board = &diagram.engines[0];
    assert_eq!(board.name, "Board");
    let functions: Vec<String> = board.functions.iter().map(|f| f.signature()).collect();
    assert_eq!(functions, vec!["play(square: integer) -> text", "winner() -> integer"]);

    let sales = &diagram.modules[0];
    assert_eq!(sales.name, "sales");
    let functions: Vec<String> = sales.functions.iter().map(|f| f.signature()).collect();
    assert_eq!(functions, vec!["order_total(lines: list<OrderLine>) -> money", "within_credit(order: Order, limit: decimal) -> bool"]);
}

#[test]
fn relations_read_as_an_erds_with_the_whole_on_the_left() {
    let (diagram, _) = diagrams::parse(SHOP);
    let relations: Vec<(&str, &str, Cardinality, Cardinality, bool)> = diagram
        .relations
        .iter()
        .map(|r| (r.left.as_str(), r.right.as_str(), r.left_count, r.right_count, r.identifying))
        .collect();
    assert_eq!(
        relations,
        vec![
            ("Customer", "Order", Cardinality::ExactlyOne, Cardinality::ZeroOrMore, false),
            ("Order", "OrderLine", Cardinality::ExactlyOne, Cardinality::ZeroOrMore, true),
            ("Order", "OrderLine", Cardinality::ExactlyOne, Cardinality::ZeroOrMore, true),
            ("Customer", "Kind", Cardinality::ExactlyOne, Cardinality::ZeroOrMore, false),
            ("Order", "Customer", Cardinality::ZeroOrMore, Cardinality::ExactlyOne, false),
        ]
    );
    assert_eq!(diagram.relations[0].label.as_deref(), Some("places"));
}

#[test]
fn what_clarity_does_not_map_is_drawn_with_a_warning() {
    let text = "classDiagram
    class Animal
    class Dog
    Animal <|-- Dog
    Dog ..> Animal
    class Shape {
        <<interface>>
    }
    class Customer {
        +String name
        +greet() text
        +blob photo
        +nickname
    }
    this is no class line
";
    let (diagram, problems) = diagrams::parse(text);
    let placed: Vec<(u32, Severity)> = problems.iter().map(|p| (p.span.line, p.severity)).collect();
    assert_eq!(
        placed,
        vec![
            (4, Severity::Warning),
            (5, Severity::Warning),
            (7, Severity::Warning),
            (11, Severity::Warning),
            (12, Severity::Error),
            (13, Severity::Error),
            (15, Severity::Error),
        ],
        "{problems:#?}"
    );
    assert!(problems[0].message.contains("inheritance"), "{}", problems[0].message);
    assert!(problems[3].message.contains("module"), "{}", problems[3].message);
    assert!(diagram.relations.is_empty());
    let names: Vec<&str> = diagram.entities.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, vec!["Animal", "Dog", "Customer"], "a class of a kind Clarity does not map is left out");
}
