use clarity_spec::{converted, load_sources};
use clarity_spec::rename::{RenameKind, rename};

const CUSTOMERS: &str = "module customers

enum Kind
    Private
    Business

entity Customer
    name: text
    kind: Kind, default Private
    bestFriend: ref Customer
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
    textbox name textbox1 at 1,1 bind customer.name
";

const OTHER: &str = "module other

// No 'use customers': its Customer is another one.
entity Customer
    x: text
";

/// The sources as a project holds them: the window Card in design/Card.cxaml.
fn sources() -> Vec<(String, String)> {
    converted([("customers.clarity", CUSTOMERS), ("sales.clarity", SALES), ("other.clarity", OTHER)])
}

fn load(sources: &[(String, String)]) -> clarity_spec::Project {
    load_sources(sources.iter().map(|(p, t)| (p.as_str(), t.as_str())))
}

fn renamed(kind: RenameKind, old: &str, new: &str) -> Result<Vec<(String, String)>, String> {
    let all = sources();
    let project = load(&all);
    let source = |path: &str| all.iter().find(|(p, _)| p == path).map(|(_, t)| t.clone());
    rename(&project, source, kind, old, new).map(|files| files.into_iter().map(|f| (f.path, f.text)).collect())
}

#[test]
fn renames_an_entity_and_every_reference() {
    // `other` declares its own Customer, which makes the name ambiguous.
    assert!(renamed(RenameKind::Entity, "Customer", "Client").unwrap_err().contains("more than one module"));

    let all = converted([("customers.clarity", CUSTOMERS), ("sales.clarity", SALES)]);
    let project = load(&all);
    let source = |path: &str| all.iter().find(|(p, _)| p == path).map(|(_, t)| t.clone());
    let files = rename(&project, source, RenameKind::Entity, "Customer", "Client").unwrap();
    let text = |path: &str| files.iter().find(|f| f.path == path).unwrap().text.clone();
    let customers = text("customers.clarity");
    assert!(customers.contains("entity Client\n"));
    assert!(customers.contains("bestFriend: ref Client\n"));
    let sales = text("sales.clarity");
    assert!(sales.contains("customer: ref Client\n    lines: list<Client>\n"), "{sales}");
    // The window's data, in its own file.
    let card = text("design/Card.cxaml");
    assert!(card.contains("<Data name=\"customer\" type=\"Client\" />") && card.contains("<Data name=\"customers\" type=\"list&lt;Client&gt;\" />"), "{card}");
    // The window title and bindings are untouched.
    assert!(card.contains("title=\"Customer\"") && card.contains("bind=\"customer.name\""), "{card}");
    let after: Vec<(String, String)> = all.iter().map(|(p, t)| (p.clone(), files.iter().find(|f| f.path == *p).map_or(t.clone(), |f| f.text.clone()))).collect();
    let after = load(&after);
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

/// A window's plain value of an enum, in the window's own file, takes the new name.
#[test]
fn renames_an_enum_in_a_windows_data() {
    let all = converted([
        ("customers.clarity", CUSTOMERS),
        ("pick.clarity", "module pick\n\nuse customers\n\npage Picker: window\n    data chosen: Kind\n"),
    ]);
    let project = load(&all);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let source = |path: &str| all.iter().find(|(p, _)| p == path).map(|(_, t)| t.clone());
    let files = rename(&project, source, RenameKind::Enum, "Kind", "CustomerKind").unwrap();
    let picker = files.iter().find(|f| f.path == "design/Picker.cxaml").expect("the window's file").text.clone();
    assert!(picker.contains("<Data name=\"chosen\" type=\"CustomerKind\" />"), "{picker}");
}
