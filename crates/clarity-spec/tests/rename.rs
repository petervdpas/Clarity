use clarity_spec::load_sources;
use clarity_spec::rename::{RenameKind, rename};

const CUSTOMERS: &str = "module customers

enum Kind
    Private
    Business

entity Customer
    name: text
    kind: Kind, default Private
    best_friend: ref Customer
";

const SALES: &str = "module sales

use customers

entity Order
    customer: ref Customer
    lines: list<Customer>

page Card: window
    title \"Customer\"
    data customer: Customer
    data customers: list<Customer>
    textbox at 1,1 bind customer.name
";

const OTHER: &str = "module other

// No 'use customers': its Customer is another one.
entity Customer
    x: text
";

fn sources() -> Vec<(&'static str, &'static str)> {
    vec![("customers.clarity", CUSTOMERS), ("sales.clarity", SALES), ("other.clarity", OTHER)]
}

fn renamed(kind: RenameKind, old: &str, new: &str) -> Result<Vec<(String, String)>, String> {
    let project = load_sources(sources());
    let source = |path: &str| sources().into_iter().find(|(p, _)| *p == path).map(|(_, t)| t.to_string());
    rename(&project, source, kind, old, new).map(|files| files.into_iter().map(|f| (f.path, f.text)).collect())
}

#[test]
fn renames_an_entity_and_every_reference() {
    // `other` declares its own Customer, which makes the name ambiguous.
    assert!(renamed(RenameKind::Entity, "Customer", "Client").unwrap_err().contains("more than one module"));

    let project = load_sources([("customers.clarity", CUSTOMERS), ("sales.clarity", SALES)]);
    let source = |path: &str| match path {
        "customers.clarity" => Some(CUSTOMERS.to_string()),
        "sales.clarity" => Some(SALES.to_string()),
        _ => None,
    };
    let files = rename(&project, source, RenameKind::Entity, "Customer", "Client").unwrap();
    let text = |path: &str| files.iter().find(|f| f.path == path).unwrap().text.clone();
    let customers = text("customers.clarity");
    assert!(customers.contains("entity Client\n"));
    assert!(customers.contains("best_friend: ref Client\n"));
    let sales = text("sales.clarity");
    assert!(sales.contains("customer: ref Client\n    lines: list<Client>\n"), "{sales}");
    assert!(sales.contains("data customer: Client\n    data customers: list<Client>\n"), "{sales}");
    // The window title and bindings are untouched.
    assert!(sales.contains("title \"Customer\"") && sales.contains("bind customer.name"));
    let after = load_sources([("customers.clarity", customers.as_str()), ("sales.clarity", sales.as_str())]);
    assert!(after.diagnostics.is_empty(), "{:#?}", after.diagnostics);
}

#[test]
fn renames_an_enum_in_field_types() {
    let files = renamed(RenameKind::Enum, "Kind", "CustomerKind").unwrap();
    assert_eq!(files.len(), 1);
    assert!(files[0].1.contains("enum CustomerKind\n"));
    assert!(files[0].1.contains("kind: CustomerKind, default Private\n"));
}

#[test]
fn refuses_bad_or_taken_names() {
    assert!(renamed(RenameKind::Enum, "Kind", "two words").unwrap_err().contains("not a valid enum name"));
    assert!(renamed(RenameKind::Enum, "Kind", "Customer").unwrap_err().contains("already declared"));
    assert!(renamed(RenameKind::Enum, "Nope", "X").unwrap_err().contains("no enum 'Nope'"));
    assert!(renamed(RenameKind::Enum, "Kind", "Kind").unwrap().is_empty());
}
