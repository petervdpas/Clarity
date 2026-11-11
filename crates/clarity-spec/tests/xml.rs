//! XML in code (docs/FORMATS-PLAN.md, step 3): `read list<Order> from xml
//! text at "shop/orders/order"` and `xml of orders at "shop/orders/order"`.
//! XML nests, and `xml of` always takes `at`: no element name is guessed.

use clarity_spec::{Severity, load_converted};

const ORDERS: &str = "module orders

entity Customer
    name: text(100)

entity Order
    number: text(20)
    customer: Customer
    tags: list<text>

function ordersIn(text: text) -> list<Order>
    return read list<Order> from xml text at \"shop/orders/order\"

function orderIn(text: text) -> Order
    return read Order from xml text at \"order\"

function written(orders: list<Order>) -> text
    return xml of orders at \"shop/orders/order\" indent true namespace \"urn:x\"

page Orders: window
    data orders: list<Order>
    button \"Load\" name load at 1,1 span 2 on click: loadClicked

code Orders
    handler loadClicked
        orders = ordersIn(\"<shop/>\")
        message written(orders)
";

fn errors(source: &str) -> Vec<String> {
    load_converted([("orders.clarity", source)]).diagnostics.iter().filter(|d| d.severity == Severity::Error).map(|d| d.message.clone()).collect()
}

fn with(old: &str, new: &str) -> String {
    assert!(ORDERS.contains(old), "{old:?}");
    ORDERS.replacen(old, new, 1)
}

#[test]
fn orders_read_and_write_xml_with_options() {
    assert_eq!(errors(ORDERS), Vec::<String>::new());
}

#[test]
fn xml_of_needs_at() {
    let without = with("xml of orders at \"shop/orders/order\" indent true", "xml of orders indent true");
    let loaded = load_converted([("orders.clarity", without.as_str())]);
    let found: Vec<_> = loaded.diagnostics.iter().filter(|d| d.severity == Severity::Error).collect();
    assert_eq!(found.iter().map(|d| d.message.as_str()).collect::<Vec<_>>(), vec!["'xml of' needs 'at': the elements around the records and the record's own, like at \"orders/order\""]);
    let line = without.lines().position(|l| l.contains("xml of orders")).unwrap() + 1;
    assert_eq!(found[0].span.line as usize, line);
}

#[test]
fn an_option_is_of_its_kind() {
    assert_eq!(errors(&with("indent true", "indent \"yes\"")), vec!["the option 'indent' of xml is true or false, not text"]);
    assert_eq!(errors(&with("namespace \"urn:x\"", "namespace 3")), vec!["the option 'namespace' of xml is text, not integer"]);
}
