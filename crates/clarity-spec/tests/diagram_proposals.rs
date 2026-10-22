//! A written diagram compared with the project (DG1, DESIGN section 5.13):
//! what it would add, change or leave out, as proposals; Apply turns the
//! ticked ones into ordinary edits, one batch per module, with their texts
//! keyed. Nothing outside the diagram is proposed for removal.

use clarity_spec::diagrams::{self, Change, Proposal};
use clarity_spec::{Project, Severity, load_sources};

const SALES: &str = "module sales

enum OrderStatus
    Pending label @orderStatusPending
    Approved label @orderStatusApproved

entity Customer
    name: text(80), required, label @customerNameLabel
    notes: text, label @customerNotesLabel
";

const BILLING: &str = "module billing

use sales

entity Invoice
    number: text, label @invoiceNumberLabel
";

const ERD: &str = "erDiagram
    Customer {
        varchar(80) name
        email email UK
    }
    Order {
        date ordered_on \"required\"
    }
    Customer ||--o{ Order : places
    Invoice }o..|| Order : bills
";

fn project() -> Project {
    load_sources([("sales.clarity", SALES), ("billing.clarity", BILLING)])
}

fn proposals(text: &str) -> Vec<Proposal> {
    let (diagram, problems) = diagrams::parse(text);
    assert!(problems.iter().all(|p| p.severity != Severity::Error), "{problems:#?}");
    diagrams::propose(&project(), &diagram)
}

fn summary(proposals: &[Proposal]) -> Vec<(Change, &str, bool, &str)> {
    proposals.iter().map(|p| (p.change, p.subject.as_str(), p.ticked, p.file.as_str())).collect()
}

#[test]
fn an_erd_proposes_what_it_adds_changes_and_leaves_out() {
    let proposals = proposals(ERD);
    assert_eq!(
        summary(&proposals),
        vec![
            (Change::Change, "Customer.name", true, "sales.clarity"),
            (Change::Add, "Customer.email", true, "sales.clarity"),
            (Change::Add, "Customer.orders", true, "sales.clarity"),
            (Change::Remove, "Customer.notes", false, "sales.clarity"),
            (Change::Add, "Order", true, "sales.clarity"),
            (Change::Add, "Invoice.order", true, "billing.clarity"),
        ]
    );
    assert_eq!(proposals[0].description, "name: text(80), no longer required");
    assert_eq!(proposals[1].description, "email: email, unique");
    assert_eq!(proposals[2].description, "orders: list<Order>, owned");
    assert_eq!(proposals[4].description, "entity Order in module sales: ordered_on, customer");
    assert_eq!(proposals[5].description, "order: ref Order, required", "exactly one at the end referred to: required");
    assert_eq!(proposals[1].span.line, 4, "a proposal points at its line of the diagram");
}

#[test]
fn applying_the_ticked_proposals_writes_each_module_once_with_keys() {
    let project = project();
    let (diagram, _) = diagrams::parse(ERD);
    let proposals = diagrams::propose(&project, &diagram);
    let ticked: Vec<usize> = proposals.iter().enumerate().filter(|(_, p)| p.ticked).map(|(i, _)| i).collect();
    let sources = [("sales.clarity".to_string(), SALES.to_string()), ("billing.clarity".to_string(), BILLING.to_string())];
    let written = diagrams::apply(&project, &sources, &proposals, &ticked).expect("the proposals apply");
    assert_eq!(written.iter().map(|w| w.path.as_str()).collect::<Vec<_>>(), vec!["sales.clarity", "billing.clarity"]);

    let sales = &written[0].text;
    assert!(sales.contains("    name: text(80), label @customerNameLabel\n"), "{sales}");
    assert!(sales.contains("    email: email, unique, label @"), "{sales}");
    assert!(sales.contains("    orders: list<Order>, owned, label @"), "{sales}");
    assert!(sales.contains("    notes: text"), "the removal was not ticked: {sales}");
    assert!(sales.contains("entity Order\n    ordered_on: date, required, label @"), "{sales}");
    assert!(sales.contains("    customer: ref Customer, required, label @"), "{sales}");
    assert!(!written[0].keys.is_empty(), "the new labels became keys");
    assert!(written[1].text.contains("    order: ref Order, required, label @"), "{}", written[1].text);

    let after = load_sources([("sales.clarity", sales.as_str()), ("billing.clarity", written[1].text.as_str())]);
    let errors: Vec<&str> = after.diagnostics.iter().filter(|d| d.severity == Severity::Error).map(|d| d.message.as_str()).collect();
    assert_eq!(errors, Vec::<&str>::new());
    let again = diagrams::propose(&after, &diagram);
    assert_eq!(summary(&again), vec![(Change::Remove, "Customer.notes", false, "sales.clarity")], "applied, only the removal is left");
}

#[test]
fn a_class_diagram_proposes_enums_and_their_values() {
    let text = "classDiagram
    class OrderStatus {
        <<enumeration>>
        Pending
        Shipped
    }
    class Kind {
        <<enumeration>>
        Private
        Business
    }
    class Customer {
        +kind: Kind
    }
";
    let proposals = proposals(text);
    assert_eq!(
        summary(&proposals),
        vec![
            (Change::Add, "OrderStatus.Shipped", true, "sales.clarity"),
            (Change::Remove, "OrderStatus.Approved", false, "sales.clarity"),
            (Change::Add, "Kind", true, "sales.clarity"),
            (Change::Add, "Customer.kind", true, "sales.clarity"),
            (Change::Remove, "Customer.name", false, "sales.clarity"),
            (Change::Remove, "Customer.notes", false, "sales.clarity"),
        ]
    );
    assert_eq!(proposals[2].description, "enum Kind in module sales: Private, Business");
}

#[test]
fn a_new_entity_goes_to_its_namespace_or_beside_what_it_relates_to() {
    let text = "classDiagram
    namespace billing {
        class Payment {
            +amount: money
        }
    }
    class Refund {
        +reason: text
    }
    Refund --> Invoice
";
    let proposals = proposals(text);
    let added: Vec<(&str, &str)> =
        proposals.iter().filter(|p| p.change == Change::Add).map(|p| (p.subject.as_str(), p.file.as_str())).collect();
    assert_eq!(added, vec![("Payment", "billing.clarity"), ("Refund", "billing.clarity")]);
}
