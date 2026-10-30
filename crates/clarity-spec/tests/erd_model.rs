//! The ERD designer's model (DG1, DESIGN section 5.13): what the canvas
//! edits (entities with their place, fields, relations), read from Mermaid
//! text and written back as plain Mermaid. Places are `%% clarity:` lines,
//! which Mermaid ignores. Writing what was read loses nothing.

use clarity_spec::diagrams::erd_model::{self, Erd, ErdEntity, ErdField, ErdRelation};
use clarity_spec::diagrams::{self, Cardinality};

/// An entity's name with its fields: name, type and modifiers.
type EntityMeaning = (String, Vec<(String, String, clarity_spec::model::Modifiers)>);
/// A relation's ends, counts, line, list and label.
type RelationMeaning = (String, String, Cardinality, Cardinality, bool, bool, Option<String>);

const SKETCH: &str = "erDiagram
    Customer {
        varchar(80) name \"required\"
        email email UK
        integer level \"min 1, default 1\"
    }
    Order {
        date ordered_on \"required\"
        decimal total \"decimal(10, 2), computed\"
        Customer customer FK
        OrderLine[] lines \"owned\"
    }
    Customer ||--o{ Order : places
    Order ||--o{ OrderLine : lines
    %% clarity: Customer at 40,60
    %% clarity: Order at 320,60
";

#[test]
fn an_erd_reads_into_entities_with_their_places_fields_and_relations() {
    let (erd, problems) = erd_model::read(SKETCH);
    assert!(problems.is_empty(), "{problems:?}");
    let places: Vec<(&str, Option<i32>, Option<i32>)> = erd.entities.iter().map(|e| (e.name.as_str(), e.x, e.y)).collect();
    assert_eq!(places, vec![("Customer", Some(40), Some(60)), ("Order", Some(320), Some(60)), ("OrderLine", None, None)]);
    let customer = &erd.entities[0];
    assert_eq!(
        customer.fields,
        vec![
            ErdField { name: "name".into(), type_text: "text(80)".into(), required: true, unique: false, extra: String::new() },
            ErdField { name: "email".into(), type_text: "email".into(), required: false, unique: true, extra: String::new() },
            ErdField { name: "level".into(), type_text: "integer".into(), required: false, unique: false, extra: "min 1, default 1".into() },
        ]
    );
    let order = &erd.entities[1];
    let types: Vec<&str> = order.fields.iter().map(|f| f.type_text.as_str()).collect();
    assert_eq!(types, vec!["date", "decimal(10, 2)", "ref Customer", "list<OrderLine>"]);
    assert_eq!(order.fields[1].extra, "computed");
    assert_eq!(order.fields[3].extra, "owned");
    assert_eq!(
        erd.relations[0],
        ErdRelation {
            left: "Customer".into(),
            right: "Order".into(),
            left_count: Cardinality::ExactlyOne,
            right_count: Cardinality::ZeroOrMore,
            identifying: true,
            label: Some("places".into()),
            via: Vec::new(),
        }
    );
}

#[test]
fn writing_what_was_read_loses_nothing() {
    let (erd, _) = erd_model::read(SKETCH);
    let text = erd_model::write(&erd);
    assert_eq!(erd_model::read(&text).0, erd, "{text}");
    let (again, problems) = diagrams::parse(&text);
    assert!(problems.is_empty(), "{problems:?}");
    let (first, _) = diagrams::parse(SKETCH);
    // The same entities, fields and relations; only the lines they are on differ.
    let meaning = |d: &diagrams::Diagram| {
        let entities: Vec<EntityMeaning> = d
            .entities
            .iter()
            .map(|e| (e.name.clone(), e.fields.iter().map(|f| (f.name.clone(), f.type_text.clone(), f.modifiers.clone())).collect()))
            .collect();
        let relations: Vec<RelationMeaning> = d
            .relations
            .iter()
            .map(|r| (r.left.clone(), r.right.clone(), r.left_count, r.right_count, r.identifying, r.lists, r.label.clone()))
            .collect();
        (entities, relations)
    };
    assert_eq!(meaning(&again), meaning(&first), "the diagram means the same:\n{text}");
}

#[test]
fn the_written_text_is_plain_mermaid_with_places_as_comments() {
    let erd = Erd {
        entities: vec![
            ErdEntity {
                name: "Customer".into(),
                x: Some(40),
                y: Some(60),
                fields: vec![ErdField { name: "name".into(), type_text: "text(80)".into(), required: true, unique: true, extra: String::new() }],
            },
            ErdEntity { name: "Invoice".into(), x: None, y: None, fields: vec![] },
        ],
        relations: vec![ErdRelation {
            left: "Invoice".into(),
            right: "Customer".into(),
            left_count: Cardinality::ZeroOrMore,
            right_count: Cardinality::ExactlyOne,
            identifying: false,
            label: None,
            via: Vec::new(),
        }],
    };
    assert_eq!(
        erd_model::write(&erd),
        "erDiagram
    Customer {
        text(80) name UK \"required\"
    }
    Invoice {
    }
    Invoice }o..|| Customer : \"\"
    %% clarity: Customer at 40,60
"
    );
}

/// A line can bend: its points are a `%% clarity: Left -> Right : label via x,y x,y`
/// line, which Mermaid ignores, read onto the relation and written back.
#[test]
fn a_relation_keeps_the_points_it_bends_at() {
    let text = format!("{SKETCH}    %% clarity: Customer -> Order : places via 200,40 200,140\n");
    let (erd, problems) = erd_model::read(&text);
    assert!(problems.is_empty(), "{problems:?}");
    assert_eq!(erd.relations[0].via, [(200, 40), (200, 140)]);
    assert!(erd.relations[1].via.is_empty());
    let written = erd_model::write(&erd);
    assert!(written.contains("    %% clarity: Customer -> Order : places via 200,40 200,140\n"), "{written}");
    assert_eq!(erd_model::read(&written).0, erd);
}
