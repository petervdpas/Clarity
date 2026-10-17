//! External data sources (D1e): databases Clarity does not own. Their
//! entities map onto existing tables and columns, they never get migration
//! steps, and a read-only source refuses `save` and `delete`.

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
    table \"tblInvoice\"
    key \"InvoiceID\"
    number: text(20), column \"InvNo\"
    customer: ref Customer, column \"CustID\"
    amount: money

action pay(invoice: Invoice)
    effects db.write
    invoice.amount = 0
    save invoice
";

const MAIN_STEP: &str = "migration \"Customers\"\n    add entity Customer\n        name: text(80)\n";

fn sources(readonly: bool) -> Vec<SourceRef> {
    let erp = SourceRef::sql("erp").external();
    vec![SourceRef::sql("main"), if readonly { erp.readonly() } else { erp }]
}

fn load(shop: &str, erp: &str, readonly: bool, steps: &[(&str, &str)]) -> Project {
    let mut all = vec![("migrations/main/0001.clarity", MAIN_STEP)];
    all.extend_from_slice(steps);
    load_in([("shop.clarity", shop), ("erp.clarity", erp)], all, &sources(readonly))
}

fn messages(project: &Project, severity: Severity) -> Vec<String> {
    project
        .diagnostics
        .iter()
        .filter(|d| d.severity == severity)
        .map(|d| d.message.clone())
        .collect()
}

#[test]
fn an_entity_maps_onto_an_existing_table() {
    let (file, diagnostics) = parse(ERP, "erp.clarity");
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let invoice = &file.entities[0];
    assert_eq!(invoice.table.as_deref(), Some("tblInvoice"));
    assert_eq!(invoice.key.as_deref(), Some("InvoiceID"));
    assert_eq!(invoice.field("number").unwrap().modifiers.column.as_deref(), Some("InvNo"));
    assert_eq!(invoice.column_of("number"), "InvNo");
    assert_eq!(invoice.column_of("amount"), "amount");
    assert_eq!(invoice.key_column(), "InvoiceID");
    assert_eq!(invoice.table_name(), "tblInvoice");
}

#[test]
fn external_entities_need_no_migration() {
    let project = load(SHOP, ERP, false, &[]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);

    let steps = load(SHOP, ERP, false, &[("migrations/erp/0001.clarity", MAIN_STEP)]);
    assert!(
        messages(&steps, Severity::Error)
            .iter()
            .any(|m| m == "data source 'erp' is external; Clarity does not change its schema, so it has no migration steps"),
        "{:#?}",
        steps.diagnostics
    );
}

#[test]
fn two_fields_cannot_share_a_column() {
    let shared = ERP.replace("amount: money", "amount: money, column \"InvNo\"");
    let project = load(SHOP, &shared, false, &[]);
    assert!(
        messages(&project, Severity::Error).iter().any(|m| m == "Invoice: fields 'number' and 'amount' both use column 'InvNo'"),
        "{:#?}",
        project.diagnostics
    );
    let key = ERP.replace("amount: money", "amount: money, column \"InvoiceID\"");
    let project = load(SHOP, &key, false, &[]);
    assert!(
        messages(&project, Severity::Error).iter().any(|m| m == "Invoice: field 'amount' uses column 'InvoiceID', which is the key"),
        "{:#?}",
        project.diagnostics
    );
}

#[test]
fn a_read_only_source_keeps_its_records() {
    let project = load(SHOP, ERP, true, &[]);
    assert!(
        messages(&project, Severity::Error)
            .iter()
            .any(|m| m == "Invoice is in data source erp, which is read-only; its records cannot be saved"),
        "{:#?}",
        project.diagnostics
    );
    let deleting = ERP.replace("    save invoice\n", "    delete invoice\n");
    let project = load(SHOP, &deleting, true, &[]);
    assert!(
        messages(&project, Severity::Error)
            .iter()
            .any(|m| m == "Invoice is in data source erp, which is read-only; its records cannot be deleted"),
        "{:#?}",
        project.diagnostics
    );
}
