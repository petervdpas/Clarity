//! Records inside records (DESIGN 5.25): an entity of a web API holds
//! another record, `owner: User`, and records, `lines: list<Line>`, as a
//! service's JSON nests them. Code reads them as any record and list:
//! `template.owner.name`, `for line in order.lines`. A stored entity refers
//! to another with `ref`, and keeps its lists in their own table.

use clarity_spec::model::FieldType;
use clarity_spec::{Project, Severity, SourceRef, load_in};

const SHOP: &str = "module shop
source shop

entity User
    name: text
    age: integer

entity Line
    product: text
    quantity: integer

entity Order
    title: text
    owner: User
    lines: list<Line>

function owner_name(order: Order) -> text
    return order.owner.name

function quantity(order: Order) -> integer
    let total = 0
    for line in order.lines
        total = total + line.quantity
    return total

action fetch_order() -> Order
    effects net
    let reply = http get \"/order\" from shop
    return read Order from json reply.text
";

const SALES: &str = "module sales

entity Customer
    name: text(80)
";

fn project(files: &[(&str, &str)]) -> Project {
    load_in(files.iter().copied(), [], &[SourceRef::sql("main"), SourceRef::web("shop")])
}

fn errors(files: &[(&str, &str)]) -> Vec<String> {
    project(files).diagnostics.iter().filter(|d| d.severity == Severity::Error).map(|d| d.message.clone()).collect()
}

#[test]
fn a_web_entity_holds_records_and_code_reads_them() {
    assert_eq!(errors(&[("shop.clarity", SHOP), ("sales.clarity", SALES)]), Vec::<String>::new());
    let project = project(&[("shop.clarity", SHOP)]);
    let order = project.entity("Order").unwrap();
    assert!(matches!(&order.fields[1].ty, FieldType::Record { entity } if entity.name == "User"));
    assert!(matches!(&order.fields[2].ty, FieldType::List { item } if item.name == "Line"));
}

#[test]
fn a_stored_entity_refers_to_records_instead() {
    let sales = "module sales\n\nentity Customer\n    name: text(80)\n\nentity Invoice\n    customer: Customer\n";
    assert_eq!(
        errors(&[("sales.clarity", sales)]),
        ["field 'customer' of Invoice holds a Customer inside the record, which only an entity of a web API does; a stored entity refers to one with ref Customer"]
    );
}

#[test]
fn what_a_web_entity_holds_lives_in_the_web_api_too() {
    let holds_stored = SHOP.replace("source shop\n", "source shop\nuse sales\n").replace("    owner: User\n", "    owner: User\n    buyer: Customer\n");
    assert_eq!(
        errors(&[("shop.clarity", &holds_stored), ("sales.clarity", "module sales\n\nentity Customer\n    name: text(80)\n")]),
        ["field 'buyer' of Order holds a Customer, which is stored; a record inside a web API's record is of the web API too"]
    );
}

#[test]
fn a_record_cannot_hold_itself() {
    let circle = SHOP.replace("    age: integer\n", "    age: integer\n    favorite: Order\n");
    assert_eq!(errors(&[("shop.clarity", &circle)]), ["record User holds itself: User.favorite holds an Order, which holds a User; hold a list instead, which may be empty"]);
}
