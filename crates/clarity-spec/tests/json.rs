//! JSON in code (DESIGN 5.23): `json of value` writes a record, a list or a
//! value as Clarity's JSON, and `read Order from json text` (or
//! `read list<Order> from json text at "data"`) reads records from it.

use clarity_spec::code::{ExprKind, StmtKind, TypeRefKind};
use clarity_spec::parser::parse;
use clarity_spec::{Severity, load_sources};

const SHOP: &str = "module shop

entity Order
    number: text(20)
    total: money

function orders_in(text: text) -> list<Order>
    return read list<Order> from json text at \"data.orders\"

action send(order: Order) -> integer
    effects net
    let reply = http post \"https://api.example.com/orders\" with json of order header \"Content-Type\": \"application/json\"
    let saved = read Order from json reply.text
    return reply.status

page Orders: window
    data orders: list<Order>
    button \"Load\" name load at 1,1 span 2 on click: loadClicked

code Orders
    handler loadClicked
        let reply = http get \"https://api.example.com/orders\"
        orders = orders_in(reply.text)
        message json of orders
";

fn errors(source: &str) -> Vec<String> {
    load_sources([("shop.clarity", source)]).diagnostics.iter().filter(|d| d.severity == Severity::Error).map(|d| d.message.clone()).collect()
}

fn with(old: &str, new: &str) -> String {
    assert!(SHOP.contains(old), "{old:?}");
    SHOP.replacen(old, new, 1)
}

#[test]
fn a_shop_reads_and_writes_json() {
    assert_eq!(errors(SHOP), Vec::<String>::new());
}

#[test]
fn parses_reading_and_writing() {
    let (file, diagnostics) = parse(SHOP, "shop.clarity");
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let StmtKind::Return { value: Some(value) } = &file.functions[0].body[0].kind else { panic!() };
    let ExprKind::ReadJson { ty, text, at } = &value.kind else { panic!("{value:#?}") };
    assert_eq!(ty.kind, TypeRefKind::List { item: "Order".into() });
    assert!(matches!(&text.kind, ExprKind::Name { name } if name == "text"));
    assert!(matches!(at.as_deref().map(|a| &a.kind), Some(ExprKind::Str { value }) if value == "data.orders"));
    let StmtKind::Let { value, .. } = &file.functions[1].body[0].kind else { panic!() };
    let ExprKind::Http { body: Some(body), .. } = &value.kind else { panic!() };
    assert!(matches!(&body.kind, ExprKind::JsonOf { .. }));
}

#[test]
fn json_reads_records_and_lists_of_them() {
    let errs = errors(&with("read list<Order> from json text at", "read integer from json text at"));
    assert!(errs.iter().any(|e| e.contains("'read ... from json' reads a record or a list of records, not integer")), "{errs:?}");
    let errs = errors(&with("read Order from json reply.text", "read Order from json reply.status"));
    assert!(errs.iter().any(|e| e.contains("the JSON to read is text, not integer")), "{errs:?}");
    let errs = errors(&with("at \"data.orders\"", "at 3"));
    assert!(errs.iter().any(|e| e.contains("where to start in the JSON is text, not integer")), "{errs:?}");
    // What it reads has the type it names.
    let errs = errors(&with("return read list<Order> from json text at \"data.orders\"", "return read Order from json text"));
    assert!(!errs.is_empty(), "one Order is no list<Order>");
}

#[test]
fn json_of_takes_records_lists_and_values() {
    let errs = errors(&with("message json of orders", "message json of reply"));
    assert!(errs.iter().any(|e| e.contains("'json of' takes a record, a list or a value, not http reply")), "{errs:?}");
    assert_eq!(errors(&with("message json of orders", "message json of 42")), Vec::<String>::new());
}

#[test]
fn an_engine_does_not_read_json() {
    let source = format!(
        "{SHOP}\nengine Board\n    function take(t: text) -> integer\n        let o = read Order from json t\n        return 1\n"
    );
    let errs = errors(&source);
    assert!(errs.iter().any(|e| e.contains("an engine does not read JSON")), "{errs:?}");
}
