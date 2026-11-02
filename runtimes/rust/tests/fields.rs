// What a screen knows of a program's fields and types (DESIGN 7.9): the
// facts of each entity's field, keyed "Entity.field", found by a path in a
// window's data as the C# host finds them, and the type at such a path.

use clarity_runtime::fields::{Facts, FieldFacts, FieldFormat, FormatKind};

/// A window `Orders` with one order, a list of orders and a status, as a generated program carries it.
static FACTS: Facts = Facts {
    windows: &[("Orders", &[("order", "Order"), ("orders", "list<Order>"), ("status", "Status"), ("note", "text")])],
    records: &[("Order", &[("total", "money"), ("status", "Status"), ("customer", "ref<Customer>"), ("lines", "list<Line>")]), ("Line", &[("count", "integer")])],
    fields: &[
        ("Order.total", FieldFacts { label: "@totalLabel", format: Some(FieldFormat { kind: FormatKind::Money, scale: None, format: None }), required: true, min: Some("0"), max: None, refers: None, parent: None, color: false }),
        ("Order.customer", FieldFacts { label: "Customer", format: None, required: false, min: None, max: None, refers: Some("Customer"), parent: None, color: false }),
        ("Line.count", FieldFacts { label: "Count", format: Some(FieldFormat { kind: FormatKind::Integer, scale: None, format: None }), required: false, min: None, max: None, refers: None, parent: None, color: false }),
    ],
    enums: &[("Status", &["open", "paid"])],
};

#[test]
fn a_field_is_found_by_its_record_and_name() {
    let total = FACTS.field("Orders", "order.total").expect("order.total");
    assert_eq!(total.label, "@totalLabel");
    assert!(total.required);
    assert_eq!(total.min, Some("0"));
    assert_eq!(total.format.map(|f| f.kind), Some(FormatKind::Money));
    assert_eq!(FACTS.field("Orders", "order.customer").and_then(|f| f.refers), Some("Customer"));
    assert_eq!(FACTS.field_of("Line.count").map(|f| f.label), Some("Count"));
}

#[test]
fn a_row_of_a_list_is_its_record_by_place_or_not() {
    for path in ["orders[0].total", "orders.total", "orders[12].total"] {
        assert_eq!(FACTS.field("Orders", path).map(|f| f.label), Some("@totalLabel"), "{path}");
    }
    assert_eq!(FACTS.field("Orders", "order.lines[1].count").map(|f| f.label), Some("Count"));
}

#[test]
fn a_value_that_is_no_field_has_no_facts() {
    for path in ["note", "status", "order", "order.nothing", "nothing.total"] {
        assert_eq!(FACTS.field("Orders", path), None, "{path}");
    }
    assert_eq!(FACTS.field("Elsewhere", "order.total"), None);
}

#[test]
fn the_type_at_a_path() {
    assert_eq!(FACTS.type_of("Orders", "order"), Some("Order"));
    assert_eq!(FACTS.type_of("Orders", "orders"), Some("list<Order>"));
    assert_eq!(FACTS.type_of("Orders", "orders[3]"), Some("Order"));
    assert_eq!(FACTS.type_of("Orders", "orders[3].status"), Some("Status"));
    assert_eq!(FACTS.type_of("Orders", "order.status"), Some("Status"));
    assert_eq!(FACTS.type_of("Orders", "status"), Some("Status"));
    assert_eq!(FACTS.type_of("Orders", "note"), Some("text"));
    assert_eq!(FACTS.type_of("Orders", "order.lines"), Some("list<Line>"));
    assert_eq!(FACTS.type_of("Orders", "nothing"), None);
}

#[test]
fn an_enum_offers_its_values() {
    assert_eq!(FACTS.enum_values("Status"), &["open", "paid"]);
    assert!(FACTS.enum_values("Order").is_empty());
}

#[test]
fn a_program_without_facts_knows_none() {
    assert_eq!(Facts::EMPTY.field("Orders", "order.total"), None);
    assert_eq!(Facts::EMPTY.type_of("Orders", "order"), None);
}
