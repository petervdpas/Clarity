//! Entities in a web API (DESIGN 5.23): a module with `source shop`, where
//! shop is a `rest` data source, holds the records a service sends and
//! takes. They are not stored, so no migration covers them and `find`,
//! `save` and `delete` do not reach them; actions read and send them with
//! `http ... from shop`, which calls the source's address with its auth,
//! and `query "name": value`, which adds an encoded query parameter.

use clarity_spec::code::{ExprKind, StmtKind};
use clarity_spec::{Project, Severity, SourceRef, load_in};

const SHOP: &str = "module shop
source shop

entity Order
    number: text(20)
    total: money

action list_orders(status: text) -> list<Order>
    effects net
    let reply = http get \"/orders\" from shop query \"status\": status query \"limit\": \"50\"
    if not reply.ok
        refuse \"shop answered {reply.status}: {reply.text}\"
    return read list<Order> from json reply.text

action create_order(order: Order) -> Order
    effects net
    let reply = http post \"/orders\" from shop with json of order header \"Content-Type\": \"application/json\"
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
fn entities_live_in_a_web_api() {
    assert_eq!(errors(&[("shop.clarity", SHOP), ("sales.clarity", SALES)]), Vec::<String>::new());
}

#[test]
fn they_are_not_stored() {
    let project = project(&[("shop.clarity", SHOP), ("sales.clarity", SALES)]);
    let stored: Vec<&str> = project.stored_entities().map(|e| e.name.name.as_str()).collect();
    assert_eq!(stored, ["Customer"]);
    assert_eq!(project.entities_in("shop").count(), 1);
}

#[test]
fn a_database_query_does_not_reach_them() {
    let with_find = format!("{SHOP}\naction stored() -> list<Order>\n    effects db.read\n    return find o in Order\n");
    let errs = errors(&[("shop.clarity", &with_find)]);
    assert!(errs.iter().any(|e| e == "Order lives in the web API shop, not in a database: its actions read and send it"), "{errs:?}");
    let with_save = format!("{SHOP}\naction keep(o: Order)\n    effects db.write\n    save o\n");
    let errs = errors(&[("shop.clarity", &with_save)]);
    assert!(errs.iter().any(|e| e == "Order lives in the web API shop, not in a database: its actions read and send it"), "{errs:?}");
}

#[test]
fn a_call_from_a_source_names_a_web_api() {
    let (file, diagnostics) = clarity_spec::parser::parse(SHOP, "shop.clarity");
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let StmtKind::Let { value, .. } = &file.functions[0].body[0].kind else { panic!() };
    let ExprKind::Http { from, query, .. } = &value.kind else { panic!("{value:#?}") };
    assert_eq!(from.as_ref().map(|f| f.name.as_str()), Some("shop"));
    assert_eq!(query.iter().map(|q| q.name.name.as_str()).collect::<Vec<_>>(), ["status", "limit"]);
    let errs = errors(&[("shop.clarity", &SHOP.replace("from shop query", "from main query"))]);
    assert!(errs.iter().any(|e| e == "'main' is no web API: 'from' names a rest data source of the project"), "{errs:?}");
    let errs = errors(&[("shop.clarity", &SHOP.replace("from shop query", "from nowhere query"))]);
    assert!(errs.iter().any(|e| e.starts_with("unknown data source 'nowhere'")), "{errs:?}");
    let errs = errors(&[("shop.clarity", &SHOP.replace("query \"limit\": \"50\"", "query \"limit\": 50"))]);
    assert!(errs.iter().any(|e| e == "the query parameter 'limit' is text, not integer"), "{errs:?}");
}

/// `every page` reads a long list a page at a time, as the source's `paging`
/// line says (DESIGN 5.23, paging); only `get` reads pages, from a source
/// that pages.
#[test]
fn every_page_reads_a_source_that_pages() {
    use clarity_spec::{Paging, PagingKind};
    let paged = |paging: Option<Paging>| {
        let shop = SourceRef { paging, ..SourceRef::web("shop") };
        move |text: &str| -> Vec<String> {
            load_in([("shop.clarity", text)], [], &[SourceRef::sql("main"), shop.clone()])
                .diagnostics
                .iter()
                .filter(|d| d.severity == Severity::Error)
                .map(|d| d.message.clone())
                .collect()
        }
    };
    let pages = Paging { kind: PagingKind::Page, param: "page".into(), size_param: "per_page".into(), size: 100, next: String::new(), items: "data".into() };
    let every = SHOP.replace("from shop query \"status\": status query \"limit\": \"50\"", "from shop every page query \"status\": status");
    let (file, diagnostics) = clarity_spec::parser::parse(&every, "shop.clarity");
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let StmtKind::Let { value, .. } = &file.functions[0].body[0].kind else { panic!() };
    let ExprKind::Http { pages: every_page, .. } = &value.kind else { panic!("{value:#?}") };
    assert!(*every_page);
    assert_eq!(paged(Some(pages.clone()))(&every), Vec::<String>::new());
    assert_eq!(paged(None)(&every), ["data source 'shop' does not page: say how under its datasource line, like: paging page \"page\" size \"per_page\" 100"]);
    let posted = every.replace("http get \"/orders\"", "http post \"/orders\"").replace("every page query \"status\": status", "every page with \"x\"");
    assert!(paged(Some(pages.clone()))(&posted).contains(&"only http get reads every page".to_string()), "{:?}", paged(Some(pages.clone()))(&posted));
    let unsourced = every.replace("http get \"/orders\" from shop every page", "http get \"https://shop.example.com/orders\" every page");
    assert!(paged(Some(pages))(&unsourced).contains(&"every page reads a web API data source: write 'from' and its name before it".to_string()));
}
