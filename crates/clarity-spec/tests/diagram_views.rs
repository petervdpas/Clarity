//! Views generated from the project (DG1, DESIGN section 5.13): the Data
//! model as an ERD in Mermaid, with each line linked to what it shows.
//! A view read back proposes nothing: it is the project, drawn.

use clarity_spec::diagrams::{self, views};
use clarity_spec::{Project, Severity, load_sources};

const SALES: &str = "module sales

enum OrderStatus
    Pending label @orderStatusPending

entity Order
    number: text(12), unique, required, label @orderNumberLabel
    customer: ref Customer, required, label @orderCustomerLabel
    status: OrderStatus, default Pending, label @orderStatusLabel
    total: decimal(10, 2), computed, label @orderTotalLabel
    lines: list<OrderLine>, owned, label @orderLinesLabel

entity OrderLine
    order: ref Order, required, label @orderLineOrderLabel

entity Customer
    name: text(80), label @customerNameLabel
";

#[test]
fn the_data_model_is_an_erd_of_every_entity() {
    let project = load_sources([("sales.clarity", SALES)]);
    let view = views::data_model(&project);
    assert_eq!(
        view.text,
        "erDiagram
    Order {
        text(12) number UK \"required\"
        Customer customer FK \"required\"
        OrderStatus status
        decimal total \"decimal(10, 2), computed\"
        OrderLine[] lines \"owned\"
    }
    OrderLine {
        Order order FK \"required\"
    }
    Customer {
        text(80) name
    }
    Order }o..|| Customer : customer
    Order ||--o{ OrderLine : lines
"
    );
    let order = view.links.iter().find(|l| l.line == 2).unwrap();
    assert_eq!((order.path.as_str(), order.span.line), ("sales.clarity", 6));
    let number = view.links.iter().find(|l| l.line == 3).unwrap();
    assert_eq!(number.span.line, 7);
    let lines = view.links.iter().find(|l| l.line == 16).unwrap();
    assert_eq!(lines.span.line, 11, "a relationship links to the list that makes it");
}

fn example(name: &str) -> Project {
    let folder = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples").join(name);
    let mut texts = Vec::new();
    for entry in std::fs::read_dir(&folder).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "clarity") {
            texts.push((path.display().to_string(), std::fs::read_to_string(&path).unwrap()));
        }
    }
    load_sources(texts.iter().map(|(p, t)| (p.as_str(), t.as_str())))
}

#[test]
fn every_examples_data_model_reads_back_as_the_project() {
    for name in ["shop", "records", "todo", "noughts", "calculator", "dial", "ledger", "stock", "orbs"] {
        let project = example(name);
        let view = views::data_model(&project);
        let (diagram, problems) = diagrams::parse(&view.text);
        let errors: Vec<&str> = problems.iter().filter(|p| p.severity == Severity::Error).map(|p| p.message.as_str()).collect();
        assert_eq!(errors, Vec::<&str>::new(), "{name}:\n{}", view.text);
        let proposals: Vec<String> = diagrams::propose(&project, &diagram).iter().map(|p| format!("{} {}", p.subject, p.description)).collect();
        assert_eq!(proposals, Vec::<String>::new(), "{name}:\n{}", view.text);
    }
}

#[test]
fn the_data_model_keeps_the_places_of_its_boxes_for_the_entities_it_draws() {
    let project = load_sources([("sales.clarity", SALES)]);
    let places = vec![("Customer".to_string(), 300, 40), ("Gone".to_string(), 1, 2), ("Order".to_string(), 40, 60)];
    let view = views::data_model_placed(&project, None, &places);
    let plain = views::data_model(&project);
    assert!(view.text.starts_with(&plain.text), "the places come after the drawing");
    assert_eq!(&view.text[plain.text.len()..], "    %% clarity: Order at 40,60\n    %% clarity: Customer at 300,40\n");
    assert_eq!(view.links, plain.links);

    let (erd, problems) = diagrams::erd_model::read(&view.text);
    assert!(problems.is_empty(), "{problems:?}");
    let order = erd.entities.iter().find(|e| e.name == "Order").unwrap();
    assert_eq!((order.x, order.y), (Some(40), Some(60)));
    assert_eq!(diagrams::erd_model::places(&view.text).len(), 2);
    let (diagram, _) = diagrams::parse(&view.text);
    assert!(diagrams::propose(&project, &diagram).is_empty());
}

#[test]
fn every_examples_class_diagram_reads_back_as_the_project() {
    for name in ["shop", "records", "todo", "noughts", "calculator", "dial", "ledger", "stock", "orbs"] {
        let project = example(name);
        let text = diagrams::class_model::write(&views::classes_of(&project, None));
        let (diagram, problems) = diagrams::parse(&text);
        let errors: Vec<&str> = problems.iter().filter(|p| p.severity == Severity::Error).map(|p| p.message.as_str()).collect();
        assert_eq!(errors, Vec::<&str>::new(), "{name}:\n{text}");
        let proposals: Vec<String> = diagrams::propose(&project, &diagram).iter().map(|p| format!("{} {}", p.subject, p.description)).collect();
        assert_eq!(proposals, Vec::<String>::new(), "{name}:\n{text}");
    }
}

#[test]
fn the_class_diagram_draws_each_module_with_its_classes_and_refs_and_lists_as_lines() {
    let project = load_sources([("sales.clarity", SALES)]);
    let design = views::classes_of(&project, None);
    let boxes: Vec<(&str, diagrams::class_model::ClassBoxKind, Option<&str>)> =
        design.boxes.iter().map(|b| (b.name.as_str(), b.kind, b.module.as_deref())).collect();
    use diagrams::class_model::ClassBoxKind::{Entity, Enum};
    assert_eq!(
        boxes,
        vec![("OrderStatus", Enum, Some("sales")), ("Order", Entity, Some("sales")), ("OrderLine", Entity, Some("sales")), ("Customer", Entity, Some("sales"))]
    );
    let order_fields: Vec<&str> = design.boxes[1].fields.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(order_fields, vec!["number", "status", "total"], "refs and lists to drawn entities are lines");
    let links: Vec<(&str, &str, diagrams::class_model::ClassLink, Option<&str>)> =
        design.relations.iter().map(|r| (r.left.as_str(), r.right.as_str(), r.link, r.label.as_deref())).collect();
    use diagrams::class_model::ClassLink::{Association, Composition};
    assert_eq!(links, vec![("Order", "Customer", Association, Some("customer")), ("Order", "OrderLine", Composition, Some("lines"))]);

    // Drawing part of the model keeps a ref to an entity left out as a field.
    let only = views::classes_of(&project, Some(&["OrderLine".to_string(), "Customer".to_string()]));
    let names: Vec<&str> = only.boxes.iter().map(|b| b.name.as_str()).collect();
    assert_eq!(names, vec!["OrderLine", "Customer"]);
    assert_eq!(only.boxes[0].fields[0].type_text, "ref Order");
    assert!(only.relations.is_empty());
}

#[test]
fn a_diagram_is_written_again_from_the_project_keeping_its_places() {
    let project = load_sources([("sales.clarity", SALES)]);
    let erd = "erDiagram\n    Customer {\n    }\n    Stale {\n    }\n    %% clarity: Customer at 300,40\n";
    let all = views::generate(&project, erd, false).unwrap();
    assert_eq!(all, views::data_model_placed(&project, None, &[("Customer".into(), 300, 40)]).text);
    let drawn = views::generate(&project, erd, true).unwrap();
    assert_eq!(drawn, "erDiagram\n    Customer {\n        text(80) name\n    }\n    %% clarity: Customer at 300,40\n", "only the entities it draws that the project has");

    let classes = "classDiagram\n    class Order\n    %% clarity: Order at 10,20\n";
    let text = views::generate(&project, classes, true).unwrap();
    let (design, _) = diagrams::class_model::read(&text);
    assert_eq!(design.boxes.iter().map(|b| (b.name.as_str(), b.x)).collect::<Vec<_>>(), vec![("Order", Some(10))]);
    assert!(views::generate(&project, "gantt\n", false).is_none(), "a kind Clarity only draws is not written from the project");
}

#[test]
fn a_module_uses_the_engines_it_calls_and_the_records_its_functions_take_find_and_make() {
    let project = example("records");
    let design = views::classes_of(&project, None);
    let uses: Vec<(&str, &str)> = design
        .relations
        .iter()
        .filter(|r| r.link == diagrams::class_model::ClassLink::Uses)
        .map(|r| (r.left.as_str(), r.right.as_str()))
        .collect();
    assert!(uses.contains(&("music", "Images")), "{uses:?}");
    assert!(uses.contains(&("music", "Track")), "total_length takes a list of tracks: {uses:?}");
    assert!(uses.contains(&("loans", "Loan")), "{uses:?}");
    assert!(uses.iter().all(|(from, _)| ["music", "loans", "records"].contains(from)), "only modules use: {uses:?}");
}
