//! Data sources per module (D1c): `source erp` places a module's entities
//! in a data source of the project; each owned SQL source has its own
//! migration steps.

use clarity_spec::migrate;
use clarity_spec::parser::parse;
use clarity_spec::{Project, Severity, SourceRef, load_in};

const SHOP: &str = "module shop

entity Customer
    name: text(80)
";

const ERP: &str = "module erp
use shop
source erp

entity Invoice
    number: text(20)
    customer: ref Customer
";

fn sources() -> Vec<SourceRef> {
    vec![SourceRef::sql("main"), SourceRef::sql("erp"), SourceRef::web("payments")]
}

fn load(erp: &str, steps: &[(&str, &str)]) -> Project {
    load_in([("shop.clarity", SHOP), ("erp.clarity", erp)], steps.iter().copied(), &sources())
}

fn errors(project: &Project) -> Vec<String> {
    project
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .map(|d| d.message.clone())
        .collect()
}

fn pending(project: &Project, source: &str) -> Vec<String> {
    migrate::pending(project, source, &[]).iter().map(migrate::op_text).collect()
}

const MAIN_STEP: &str = "migration \"Customers\"\n    add entity Customer\n        name: text(80)\n";
const ERP_STEP: &str =
    "migration \"Invoices\"\n    add entity Invoice\n        number: text(20)\n        customer: ref Customer\n";

#[test]
fn a_module_names_its_data_source() {
    let (file, diagnostics) = parse(ERP, "erp.clarity");
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    assert_eq!(file.source.as_ref().map(|s| s.name.as_str()), Some("erp"));

    let project = load(ERP, &[("migrations/main/0001.clarity", MAIN_STEP), ("migrations/erp/0001.clarity", ERP_STEP)]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    assert_eq!(project.source_of("Invoice"), Some("erp"));
    // Without a source line, a module uses the first data source.
    assert_eq!(project.source_of("Customer"), Some("main"));
}

#[test]
fn the_source_must_be_a_sql_source_of_the_project() {
    let unknown = load(&ERP.replace("source erp", "source warehouse"), &[]);
    assert!(
        errors(&unknown).iter().any(|e| e == "unknown data source 'warehouse'; the project has main, erp and payments"),
        "{:#?}",
        unknown.diagnostics
    );
    let web = load(&ERP.replace("source erp", "source payments"), &[]);
    assert!(
        errors(&web).iter().any(|e| e == "'payments' is a web API; entities cannot live in a web API yet"),
        "{:#?}",
        web.diagnostics
    );
    let twice = load(&ERP.replace("source erp", "source erp\nsource main"), &[]);
    assert!(errors(&twice).iter().any(|e| e.contains("a module has one 'source'")), "{:#?}", twice.diagnostics);
}

#[test]
fn each_source_has_its_own_steps() {
    let project = load(ERP, &[("migrations/main/0001.clarity", MAIN_STEP)]);
    assert!(pending(&project, "main").is_empty());
    assert_eq!(pending(&project, "erp"), ["add entity Invoice\n    number: text(20)\n    customer: ref Customer"]);
    let warnings: Vec<&str> = project
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Warning)
        .map(|d| d.message.as_str())
        .collect();
    assert_eq!(warnings, ["entity 'Invoice' is not in a migration yet (data source erp)"]);
}

#[test]
fn moving_a_module_to_another_source_is_a_removal_and_an_addition() {
    let moved = ERP.replace("source erp", "source main");
    let project = load(&moved, &[("migrations/main/0001.clarity", MAIN_STEP), ("migrations/erp/0001.clarity", ERP_STEP)]);
    assert_eq!(pending(&project, "erp"), ["remove entity Invoice"]);
    assert_eq!(pending(&project, "main")[0].lines().next(), Some("add entity Invoice"));
}

#[test]
fn steps_for_a_source_the_project_does_not_have_are_an_error() {
    let project = load(ERP, &[("migrations/old/0001.clarity", MAIN_STEP)]);
    assert!(
        errors(&project).iter().any(|e| e == "the project has no data source 'old' for these steps"),
        "{:#?}",
        project.diagnostics
    );
}

/// A query runs in one database: it cannot go through a reference into
/// another data source.
#[test]
fn a_query_stays_in_its_own_database() {
    let erp = format!("{ERP}\nfunction of(name: text) -> list<Invoice>\n    return find i in Invoice where i.customer.name == name order by i.customer.name\n");
    let project = load(&erp, &[]);
    let errors = errors(&project);
    assert!(
        errors.iter().filter(|e| *e == "'i.customer' refers to Customer in data source 'main'; a query reads one database").count() == 2,
        "{errors:#?}"
    );
}
