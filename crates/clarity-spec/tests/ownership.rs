//! Mapped names in migration steps, and the baseline step Take ownership
//! writes for a database that was external (D1e).

use clarity_spec::migrate::{self, Op};
use clarity_spec::{Project, Severity, SourceRef, load_in};

const ERP: &str = "module erp
source erp

entity Invoice
    table \"tblInvoice\"
    key \"InvoiceID\"
    number: text(20), unique, column \"InvNo\"
    amount: money
";

const BASELINE: &str = "migration \"Taken over\" baseline
    add entity Invoice
        table \"tblInvoice\"
        key \"InvoiceID\"
        number: text(20), unique, column \"InvNo\"
        amount: money
";

fn load(erp: &str, steps: &[(&str, &str)]) -> Project {
    load_in([("erp.clarity", erp)], steps.iter().copied(), &[SourceRef::sql("main"), SourceRef::sql("erp")])
}

fn pending(project: &Project) -> Vec<String> {
    migrate::pending(project, "erp", &[]).iter().map(migrate::op_text).collect()
}

#[test]
fn a_baseline_step_describes_the_mapped_tables() {
    let project = load(ERP, &[("migrations/erp/0001.clarity", BASELINE)]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let steps = &project.migrations.as_ref().unwrap()["erp"];
    assert!(steps[0].baseline);
    let schema = migrate::schema(&project, "erp");
    let invoice = schema.table("Invoice").unwrap();
    assert_eq!(invoice.table_name(), "tblInvoice");
    assert_eq!(invoice.key_column(), "InvoiceID");
    assert_eq!(invoice.column_name("number"), "InvNo");
    assert_eq!(invoice.column_name("amount"), "amount");

    // The step text is what Take ownership writes.
    let written = migrate::step_text_baseline("Taken over", &migrate::pending(&load(ERP, &[]), "erp", &[]));
    assert!(written.ends_with(BASELINE), "{written}");
}

#[test]
fn only_the_first_step_can_be_a_baseline() {
    let second = "migration \"Again\" baseline\n    add entity Note\n        text: text\n";
    let project = load(ERP, &[("migrations/erp/0001.clarity", BASELINE), ("migrations/erp/0002.clarity", second)]);
    assert!(
        project.diagnostics.iter().any(|d| d.severity == Severity::Error && d.message == "only the first step of a data source can be a baseline"),
        "{:#?}",
        project.diagnostics
    );
}

#[test]
fn changing_a_mapping_is_a_step() {
    let remapped = ERP
        .replace("table \"tblInvoice\"", "table \"invoices\"")
        .replace("column \"InvNo\"", "column \"InvoiceNumber\"")
        .replace("key \"InvoiceID\"", "key \"id\"")
        .replace("    amount: money\n", "    amount: money, column \"Total\"\n");
    let project = load(&remapped, &[("migrations/erp/0001.clarity", BASELINE)]);
    assert_eq!(
        pending(&project),
        [
            "map table Invoice to \"invoices\"",
            "map key Invoice to \"id\"",
            "map column Invoice.number to \"InvoiceNumber\"",
            "map column Invoice.amount to \"Total\"",
        ]
    );
    let steps = format!("migration \"Remap\"\n{}", pending(&project).iter().map(|l| format!("    {l}\n")).collect::<String>());
    let project = load(&remapped, &[("migrations/erp/0001.clarity", BASELINE), ("migrations/erp/0002.clarity", &steps)]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);

    // Back to the default name.
    let plain = remapped.replace("    amount: money, column \"Total\"\n", "    amount: money\n");
    let project = load(&plain, &[("migrations/erp/0001.clarity", BASELINE), ("migrations/erp/0002.clarity", &steps)]);
    assert_eq!(pending(&project), ["map column Invoice.amount to default"]);
}

#[test]
fn owned_entities_can_keep_mapped_names() {
    let project = load(ERP, &[("migrations/erp/0001.clarity", BASELINE)]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    // A renamed field keeps its column.
    let renamed = ERP.replace("    number: text(20)", "    invoice_number: text(20)");
    let project = load(&renamed, &[("migrations/erp/0001.clarity", BASELINE)]);
    let ops = migrate::pending(&project, "erp", &migrate::proposed_renames(&project, "erp"));
    assert!(matches!(ops.as_slice(), [Op::RenameField { .. }]), "{ops:#?}");
}
