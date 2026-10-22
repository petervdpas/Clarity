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
    for name in ["shop", "records", "todo", "noughts", "calculator", "dial"] {
        let project = example(name);
        let view = views::data_model(&project);
        let (diagram, problems) = diagrams::parse(&view.text);
        let errors: Vec<&str> = problems.iter().filter(|p| p.severity == Severity::Error).map(|p| p.message.as_str()).collect();
        assert_eq!(errors, Vec::<&str>::new(), "{name}:\n{}", view.text);
        let proposals: Vec<String> = diagrams::propose(&project, &diagram).iter().map(|p| format!("{} {}", p.subject, p.description)).collect();
        assert_eq!(proposals, Vec::<String>::new(), "{name}:\n{}", view.text);
    }
}
