//! Queries (D1a): `find c in Entity where ... order by ...`, the
//! `contains` operator, the `text` of unbound text inputs, and handlers
//! that run other handlers of their window.

use clarity_spec::code::{ExprKind, StmtKind};
use clarity_spec::parser::parse;
use clarity_spec::{Severity, load_sources};

const SHOP: &str = "module shop

enum Kind
    Private
    Business

entity Customer
    name: text(80)
    kind: Kind
    active: bool
    credit_limit: money
    since: date

entity Order
    number: text(12)
    customer: ref Customer
    total: money

function big_spenders(limit: money) -> list<Customer>
    return find c in Customer where c.credit_limit >= limit and c.active order by c.credit_limit desc, c.name

page Customers: window
    data customers: list<Customer>
    data customer: Customer
    data orders: list<Order>
    on return: window_returned
    textbox name name_filter at 1,1 span 4 on change: filter_changed
    table name grid at 1,2 span 12 rows 6 bind customers columns name

code Customers
    handler window_returned
        filter_changed()

    handler filter_changed
        customers = find c in Customer where c.name contains name_filter.text order by c.name
        orders = find o in Order where o.customer == customer and o.total > 100
        let business = find c in Customer where c.kind in [Business] and not c.active
        let all = find c in Customer
        if count(all) > 0 and \"Ada\" contains \"a\"
            name_filter.text = \"\"
";

fn problems(source: &str) -> Vec<(Severity, String)> {
    load_sources([("shop.clarity", source)])
        .diagnostics
        .into_iter()
        .map(|d| (d.severity, d.message))
        .collect()
}

fn errors(source: &str) -> Vec<String> {
    problems(source)
        .into_iter()
        .filter(|(s, _)| *s == Severity::Error)
        .map(|(_, m)| m)
        .collect()
}

fn assert_has(messages: &[String], needle: &str) {
    assert!(
        messages.iter().any(|m| m.contains(needle)),
        "missing {needle:?} in {messages:#?}"
    );
}

fn with(old: &str, new: &str) -> String {
    assert!(SHOP.contains(old), "{old:?} is not in the source");
    SHOP.replace(old, new)
}

#[test]
fn the_example_checks_clean() {
    assert_eq!(problems(SHOP), Vec::new());
}

#[test]
fn parses_a_query_with_its_condition_and_order() {
    let (file, diagnostics) = parse(SHOP, "shop.clarity");
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let body = &file.functions[0].body;
    let StmtKind::Return { value: Some(value) } = &body[0].kind else {
        panic!("{body:#?}")
    };
    let ExprKind::Find { var, entity, filter, order } = &value.kind else {
        panic!("{value:#?}")
    };
    assert_eq!(var.name, "c");
    assert_eq!(entity.name, "Customer");
    assert!(filter.is_some());
    assert_eq!(order.len(), 2);
    assert_eq!(order[0].field.name, "credit_limit");
    assert!(order[0].descending);
    assert_eq!(order[1].field.name, "name");
    assert!(!order[1].descending);
}

#[test]
fn a_query_needs_an_entity_and_its_own_fields() {
    let unknown = with("find c in Customer\n", "find c in Nobody\n");
    assert_has(&errors(&unknown), "unknown entity 'Nobody'");

    let field = with("where c.name contains", "where c.nickname contains");
    assert_has(&errors(&field), "Customer has no field 'nickname'");

    let order = with("order by c.name\n", "order by d.name\n");
    assert_has(&errors(&order), "order by a field of c, as in 'c.name'");
}

#[test]
fn a_query_compares_a_field_with_a_value() {
    let two_fields = with("c.credit_limit >= limit", "c.credit_limit >= c.credit_limit");
    assert_has(
        &errors(&two_fields),
        "a query compares a field of c with a value, as in 'c.name == x'",
    );

    let not_bool = with("where c.kind in [Business]", "where c.name");
    assert_has(&errors(&not_bool), "a condition must be bool, not text");
}

#[test]
fn contains_is_for_text() {
    let source = with("c.name contains name_filter.text", "c.credit_limit contains name_filter.text");
    assert_has(&errors(&source), "'contains' needs text, not money and text");
}

#[test]
fn unbound_text_inputs_have_text() {
    let bound = with(
        "textbox name name_filter at 1,1 span 4 on change",
        "textbox name name_filter at 1,1 span 4 bind customer.name on change",
    );
    assert_has(&errors(&bound), "textbox 'name_filter' is bound to customer.name; use customer.name");
}

#[test]
fn handlers_run_other_handlers_of_their_window() {
    let args = with("        filter_changed()\n", "        filter_changed(1)\n");
    assert_has(&errors(&args), "handler 'filter_changed' takes 0 values, not 1");

    let value = with("let all = find c in Customer\n", "let all = filter_changed()\n");
    assert_has(&errors(&value), "handler 'filter_changed' does not return a value");

    let unknown = with("        filter_changed()\n", "        nothing_here()\n");
    assert_has(&errors(&unknown), "unknown function 'nothing_here'");
}

#[test]
fn the_query_variable_is_only_inside_the_query() {
    let source = with("        let all = find c in Customer\n", "        let all = find c in Customer\n        let x = c.name\n");
    assert_has(&errors(&source), "unknown name 'c'");
}

/// X2: a condition or an order may go through references, to the fields of
/// the records they refer to; the database reads them in a subquery.
#[test]
fn a_query_goes_through_references() {
    let through = with(
        "orders = find o in Order where o.customer == customer and o.total > 100",
        "orders = find o in Order where o.customer.name contains name_filter.text and o.customer.active and o.customer.kind in [Business] and o.customer.since != none and o.customer.credit_limit > 10 order by o.customer.name desc, o.number",
    );
    assert_eq!(problems(&through), Vec::new());

    let (file, _) = parse(&through, "shop.clarity");
    let order = file.code[0].handlers[1].body[1].kind.clone();
    let StmtKind::Assign { value, .. } = order else { panic!("{order:#?}") };
    let ExprKind::Find { order, .. } = &value.kind else { panic!("{value:#?}") };
    assert_eq!(order[0].through.iter().map(|i| i.name.as_str()).collect::<Vec<_>>(), ["customer"]);
    assert_eq!(order[0].field.name, "name");
    assert!(order[0].descending);
    assert!(order[1].through.is_empty());
}

#[test]
fn a_query_goes_through_references_only() {
    let text = with("o.total > 100", "o.number.name == \"Ada\"");
    assert_has(&errors(&text), "'o.number' is text, not a reference; a query goes through references only");

    let order = with("order by c.name\n", "order by c.since.year\n");
    assert_has(&errors(&order), "'c.since' is a date, not a reference; a query goes through references only");

    let missing = with("o.total > 100", "o.customer.nickname == \"Ada\"");
    assert_has(&errors(&missing), "Customer has no field 'nickname'");
}
