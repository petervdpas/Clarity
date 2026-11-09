//! Renaming a field of an entity or a record: its declaration and every
//! place that names it, found through the parsed model: code (members,
//! `find` order, `print` columns, text slots) and the windows' `.cxaml`
//! (bindings, columns, charts, trees, searches). A field of the same name in
//! another entity is left alone.

use clarity_spec::rename::rename_field;
use clarity_spec::{converted, load_sources, Severity};

const CUSTOMERS: &str = "module customers

entity Customer
    name: text
    city: text
    total: integer

entity Supplier
    name: text

entity Order
    customer: ref Customer
    status: text
    total: integer
";

const SALES: &str = "module sales

use customers

page Card: window
    title \"Customer\"
    data customer: Customer
    data customers: list<Customer>
    data order: Order
    data orders: list<Order>
    data supplier: Supplier
    textbox name nameBox at 1,1 span 4 bind customer.name
    textbox name supplierBox at 5,1 span 4 bind supplier.name
    table name grid at 1,2 span 12 rows 4 bind customers columns name, city edit name on open: gridOpened
    dropdown name customerBox at 1,6 span 6 bind order.customer search show name, city
    chart bar name byStatus at 1,7 span 6 rows 4 bind orders by status sum total
    button \"Find\" name findButton at 7,6 span 2 on click: findClicked

code Card
    handler gridOpened
        customers = find c in Customer where c.name != \"\" order by c.name

    handler findClicked
        customer.name = supplier.name
        let found = find o in Order where o.customer.name == \"x\" order by o.customer.name
        message \"Found {customer.name} of {supplier.name}\"

command customer list
    help \"Lists the customers\"
    let found = find c in Customer order by c.name
    print found columns name, city
";

fn sources() -> Vec<(String, String)> {
    converted([("customers.clarity", CUSTOMERS), ("sales.clarity", SALES)])
}

fn load(sources: &[(String, String)]) -> clarity_spec::Project {
    load_sources(sources.iter().map(|(p, t)| (p.as_str(), t.as_str())))
}

fn errors(project: &clarity_spec::Project) -> Vec<String> {
    project.diagnostics.iter().filter(|d| d.severity == Severity::Error).map(|d| d.message.clone()).collect()
}

/// The sources after renaming `entity.old` to `new`.
fn renamed(all: &[(String, String)], entity: &str, old: &str, new: &str) -> Result<Vec<(String, String)>, String> {
    let project = load(all);
    let source = |path: &str| all.iter().find(|(p, _)| p == path).map(|(_, t)| t.clone());
    let changed = rename_field(&project, source, entity, old, new)?;
    let mut out = all.to_vec();
    for file in changed {
        match out.iter_mut().find(|(p, _)| *p == file.path) {
            Some(slot) => slot.1 = file.text,
            None => out.push((file.path, file.text)),
        }
    }
    Ok(out)
}

fn text<'a>(all: &'a [(String, String)], end: &str) -> &'a str {
    all.iter().find(|(p, _)| p.ends_with(end)).map(|(_, t)| t.as_str()).unwrap_or_else(|| panic!("no {end}"))
}

#[test]
fn the_project_checks_clean_before() {
    assert_eq!(errors(&load(&sources())), Vec::<String>::new());
}

#[test]
fn renames_a_field_everywhere_it_is_named_and_nowhere_else() {
    let after = renamed(&sources(), "Customer", "name", "fullName").unwrap();
    assert_eq!(errors(&load(&after)), Vec::<String>::new(), "the renamed project checks clean");

    let model = text(&after, "customers.clarity");
    assert!(model.contains("entity Customer\n    fullName: text\n"), "{model}");
    assert!(model.contains("entity Supplier\n    name: text\n"), "another entity's field stays: {model}");

    let code = text(&after, "sales.clarity");
    assert!(code.contains("where c.fullName != \"\" order by c.fullName"), "{code}");
    assert!(code.contains("customer.fullName = supplier.name"), "{code}");
    assert!(code.contains("o.customer.fullName == \"x\" order by o.customer.fullName"), "through a reference: {code}");
    assert!(code.contains("\"Found {customer.fullName} of {supplier.name}\""), "text slots: {code}");
    assert!(code.contains("let found = find c in Customer order by c.fullName\n    print found columns fullName, city"), "{code}");

    let card = text(&after, "Card.cxaml");
    assert!(card.contains("bind=\"customer.fullName\""), "{card}");
    assert!(card.contains("bind=\"supplier.name\""), "{card}");
    assert!(card.contains("columns=\"fullName, city\""), "{card}");
    assert!(card.contains("edit=\"fullName\""), "{card}");
    assert!(card.contains("show=\"fullName, city\""), "a search shows the referenced entity's fields: {card}");
}

#[test]
fn renames_the_fields_a_chart_groups_and_sums_by() {
    let after = renamed(&sources(), "Order", "total", "amount").unwrap();
    assert_eq!(errors(&load(&after)), Vec::<String>::new());
    let card = text(&after, "Card.cxaml");
    assert!(card.contains("measure=\"{&quot;field&quot;:&quot;amount&quot;,&quot;kind&quot;:&quot;sum&quot;}\""), "{card}");
    let by = renamed(&sources(), "Order", "status", "state").unwrap();
    assert_eq!(errors(&load(&by)), Vec::<String>::new());
    assert!(text(&by, "Card.cxaml").contains("by=\"state\""));
    let model = text(&after, "customers.clarity");
    assert!(model.contains("    total: integer\n\nentity Supplier"), "Customer's total stays: {model}");
}

#[test]
fn refuses_what_it_cannot_rename() {
    let all = sources();
    assert!(renamed(&all, "Customer", "name", "city").unwrap_err().contains("already has a field 'city'"));
    assert!(renamed(&all, "Customer", "name", "not a name").unwrap_err().contains("not a valid field name"));
    assert!(renamed(&all, "Customer", "nothing", "x").unwrap_err().contains("has no field 'nothing'"));
    assert!(renamed(&all, "Nobody", "name", "x").unwrap_err().contains("no entity or record 'Nobody'"));
    assert_eq!(renamed(&all, "Customer", "name", "name").unwrap(), all, "the same name changes nothing");
}

/// An entity on an existing table keeps its column: the rename writes it.
#[test]
fn a_field_of_an_existing_table_keeps_its_column() {
    let all = converted([("shop.clarity", "module shop\n\nentity Invoice\n    table \"tblInvoice\"\n    customerName: text\n")]);
    let after = renamed(&all, "Invoice", "customerName", "buyer").unwrap();
    assert!(text(&after, "shop.clarity").contains("    buyer: text, column \"customer_name\"\n"), "{after:?}");
}

const TREE: &str = "module shop

entity Project
    name: text(80), label @projectNameLabel

entity Version
    project: ref Project, label @versionProjectLabel
    number: integer, label @versionNumberLabel

page Main: window
    content name main at 1,1 span 12 rows 10
        view Explorer

page Explorer: view
    tree name modelTree at 1,1 span 4 rows 10
        level Project show name order by name
            level Version under project show @versionNode order by number desc
";

const EN: &str = "translation en
    projectNameLabel \"Name\"
    versionProjectLabel \"Project\"
    versionNumberLabel \"Number\"
    versionNode \"Version {number}\"
";

/// A tree level names fields of its entity; shown by a key, the key's slots
/// are the record's fields by name, so every language's value follows.
#[test]
fn renames_the_fields_a_tree_level_hangs_under_shows_and_orders_by() {
    let mut all = converted([("shop.clarity", TREE)]);
    all.push(("translations/en.clarity".into(), EN.into()));
    let load = |all: &[(String, String)]| {
        let modules: Vec<(String, String)> = all.iter().filter(|(p, _)| !p.starts_with("translations/")).cloned().collect();
        let mut project = load(&modules);
        project.locale.language = Some("en".into());
        clarity_spec::translations::attach(&mut project, all.iter().filter(|(p, _)| p.starts_with("translations/")).map(|(p, t)| (p.as_str(), t.as_str())));
        project
    };
    let project = load(&all);
    assert_eq!(errors(&project), Vec::<String>::new());
    let source = |path: &str| all.iter().find(|(p, _)| p == path).map(|(_, t)| t.clone());
    let changed = rename_field(&project, source, "Version", "number", "release").unwrap();
    let mut after = all.clone();
    for file in changed {
        after.iter_mut().find(|(p, _)| *p == file.path).expect("a file of the project").1 = file.text;
    }
    let explorer = text(&after, "Explorer.cxaml");
    assert!(explorer.contains("order=\"release\""), "{explorer}");
    assert!(text(&after, "translations/en.clarity").contains("versionNode \"Version {release}\""));
    let under = rename_field(&project, |path: &str| all.iter().find(|(p, _)| p == path).map(|(_, t)| t.clone()), "Version", "project", "owner").unwrap();
    let explorer = &under.iter().find(|f| f.path.ends_with("Explorer.cxaml")).unwrap().text;
    assert!(explorer.contains("under=\"owner\""), "{explorer}");
    let project_name = rename_field(&project, |path: &str| all.iter().find(|(p, _)| p == path).map(|(_, t)| t.clone()), "Project", "name", "title").unwrap();
    let explorer = &project_name.iter().find(|f| f.path.ends_with("Explorer.cxaml")).unwrap().text;
    assert!(explorer.contains("show=\"title\" order=\"title\""), "{explorer}");
    assert_eq!(errors(&load(&after)), Vec::<String>::new(), "the renamed project checks clean");
}
