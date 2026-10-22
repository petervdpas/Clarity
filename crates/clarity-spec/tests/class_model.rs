//! The class diagram designer's model (DG1, DESIGN section 5.13): boxes for
//! entities, enums, engines and modules with their places, members and
//! namespaces, and relations as association, composition or aggregation,
//! read from Mermaid text and written back as plain Mermaid. Writing what
//! was read loses nothing.

use clarity_spec::diagrams::class_model::{self, ClassBox, ClassBoxKind, ClassDesign, ClassLink, ClassRelation};
use clarity_spec::diagrams::erd_model::ErdField;
use clarity_spec::diagrams::{self, Cardinality};

const SKETCH: &str = "classDiagram
    namespace shop {
        class Customer {
            +name: text(80), required
            +email: email, unique
        }
        class Order {
            +ordered_on: date
            +customer: ref Customer
        }
    }
    class OrderStatus {
        <<enumeration>>
        Pending
        Shipped
    }
    class Pricing {
        <<engine>>
        +price(order: integer) money
    }
    class shop {
        <<module>>
        +total(order: Order) money
    }
    Order \"1\" *-- \"*\" OrderLine : lines
    Order \"*\" --> \"1\" OrderStatus : status
    %% clarity: Customer at 40,60
    %% clarity: Pricing at 300,60
";

fn field(name: &str, type_text: &str, required: bool, unique: bool) -> ErdField {
    ErdField { name: name.into(), type_text: type_text.into(), required, unique, extra: String::new() }
}

#[test]
fn a_class_diagram_reads_into_boxes_of_each_kind_with_their_places_and_members() {
    let (design, problems) = class_model::read(SKETCH);
    assert!(problems.is_empty(), "{problems:?}");
    let shown: Vec<(&str, ClassBoxKind, Option<&str>, Option<i32>)> =
        design.boxes.iter().map(|b| (b.name.as_str(), b.kind, b.module.as_deref(), b.x)).collect();
    assert_eq!(
        shown,
        vec![
            ("Customer", ClassBoxKind::Entity, Some("shop"), Some(40)),
            ("Order", ClassBoxKind::Entity, Some("shop"), None),
            ("OrderStatus", ClassBoxKind::Enum, None, None),
            ("Pricing", ClassBoxKind::Engine, None, Some(300)),
            ("shop", ClassBoxKind::Module, None, None),
            ("OrderLine", ClassBoxKind::Entity, None, None),
        ]
    );
    assert_eq!(design.boxes[0].fields, vec![field("name", "text(80)", true, false), field("email", "email", false, true)]);
    assert_eq!(design.boxes[1].fields[1], field("customer", "ref Customer", false, false));
    assert_eq!(design.boxes[2].values, vec!["Pending".to_string(), "Shipped".to_string()]);
    assert_eq!(design.boxes[3].functions, vec!["price(order: integer) -> money".to_string()]);
    assert_eq!(design.boxes[4].functions, vec!["total(order: Order) -> money".to_string()]);
    assert_eq!(
        design.relations,
        vec![
            ClassRelation {
                left: "Order".into(),
                right: "OrderLine".into(),
                link: ClassLink::Composition,
                left_count: Cardinality::ExactlyOne,
                right_count: Cardinality::ZeroOrMore,
                label: Some("lines".into()),
            },
            ClassRelation {
                left: "Order".into(),
                right: "OrderStatus".into(),
                link: ClassLink::Association,
                left_count: Cardinality::ZeroOrMore,
                right_count: Cardinality::ExactlyOne,
                label: Some("status".into()),
            },
        ]
    );
}

#[test]
fn writing_what_was_read_loses_nothing() {
    let (design, _) = class_model::read(SKETCH);
    let text = class_model::write(&design);
    assert_eq!(class_model::read(&text).0, design, "{text}");
    let (diagram, problems) = diagrams::parse(&text);
    assert!(problems.is_empty(), "{problems:?}\n{text}");
    assert_eq!(diagram.kind, Some(diagrams::DiagramKind::Class));
}

#[test]
fn the_written_text_is_plain_mermaid_with_places_as_comments() {
    let design = ClassDesign {
        boxes: vec![
            ClassBox {
                name: "Customer".into(),
                kind: ClassBoxKind::Entity,
                module: Some("shop".into()),
                x: Some(40),
                y: Some(60),
                fields: vec![ErdField { extra: "min 1".into(), ..field("level", "integer", true, false) }],
                values: vec![],
                functions: vec![],
            },
            ClassBox {
                name: "Level".into(),
                kind: ClassBoxKind::Enum,
                module: None,
                x: None,
                y: None,
                fields: vec![],
                values: vec!["Gold".into()],
                functions: vec![],
            },
            ClassBox {
                name: "Invoice".into(),
                kind: ClassBoxKind::Entity,
                module: None,
                x: None,
                y: None,
                fields: vec![],
                values: vec![],
                functions: vec![],
            },
            ClassBox {
                name: "Tax".into(),
                kind: ClassBoxKind::Engine,
                module: None,
                x: None,
                y: None,
                fields: vec![],
                values: vec![],
                functions: vec!["rate(country: text) -> decimal(5, 2)".into(), "reset()".into()],
            },
        ],
        relations: vec![
            ClassRelation {
                left: "Invoice".into(),
                right: "Customer".into(),
                link: ClassLink::Association,
                left_count: Cardinality::ZeroOrMore,
                right_count: Cardinality::ExactlyOne,
                label: None,
            },
            ClassRelation {
                left: "Customer".into(),
                right: "Invoice".into(),
                link: ClassLink::Aggregation,
                left_count: Cardinality::ExactlyOne,
                right_count: Cardinality::ZeroOrMore,
                label: Some("invoices".into()),
            },
        ],
    };
    assert_eq!(
        class_model::write(&design),
        "classDiagram
    namespace shop {
        class Customer {
            +level: integer, required, min 1
        }
    }
    class Level {
        <<enumeration>>
        Gold
    }
    class Invoice
    class Tax {
        <<engine>>
        +rate(country: text) decimal(5, 2)
        +reset()
    }
    Invoice \"*\" --> \"1\" Customer
    Customer \"1\" o-- \"*\" Invoice : invoices
    %% clarity: Customer at 40,60
"
    );
}

#[test]
fn a_uses_arrow_from_a_module_reads_and_writes_back_without_a_problem() {
    let text = "classDiagram
    class shop {
        <<module>>
        +total(order: Order) money
    }
    class Pricing {
        <<engine>>
        +price(order: integer) money
    }
    class Order
    shop ..> Pricing
    shop ..> Order
";
    let (design, problems) = class_model::read(text);
    assert!(problems.is_empty(), "{problems:?}");
    let uses: Vec<(&str, &str, ClassLink)> = design.relations.iter().map(|r| (r.left.as_str(), r.right.as_str(), r.link)).collect();
    assert_eq!(uses, vec![("shop", "Pricing", ClassLink::Uses), ("shop", "Order", ClassLink::Uses)]);
    let written = class_model::write(&design);
    assert!(written.contains("    shop ..> Pricing\n"), "{written}");
    assert_eq!(class_model::read(&written).0, design);
    let (diagram, _) = diagrams::parse(&written);
    assert!(diagram.relations.is_empty(), "a uses arrow makes no field");
}
