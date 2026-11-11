//! A save never overwrites someone else's change (docs/APP-CAPABILITIES-PLAN.md
//! step 5), the language and migration side: the step that adds the version
//! column, which entities are versioned and in which column, `version` and
//! `savedSince` on entities, and `save ... anyway` and `reload` in code.

use clarity_spec::code::StmtKind;
use clarity_spec::model::Locale;
use clarity_spec::{Project, Severity, SourceRef, load_converted, load_in, migrate, translations};

const SHOP: &str = "module shop

entity Customer
    name: text(80)

entity Order
    number: text(12)
    customer: ref Customer
";

const FIRST: &str = "migration \"Customers and orders\"
    add entity Customer
        name: text(80)
    add entity Order
        number: text(12)
        customer: ref Customer
";

fn load(steps: &[(&str, &str)]) -> Project {
    load_in([("shop.clarity", SHOP)], steps.iter().copied(), &[])
}

fn pending(project: &Project) -> Vec<String> {
    migrate::pending(project, "main", &[]).iter().map(migrate::op_text).collect()
}

fn messages(project: &Project, severity: Severity) -> Vec<String> {
    project.diagnostics.iter().filter(|d| d.severity == severity).map(|d| d.message.clone()).collect()
}

fn assert_has(messages: &[String], needle: &str) {
    assert!(messages.iter().any(|m| m.contains(needle)), "missing {needle:?} in {messages:#?}");
}

#[test]
fn an_existing_table_gets_its_version_from_a_step() {
    let project = load(&[("0001.clarity", FIRST)]);
    assert_eq!(pending(&project), ["add version Customer", "add version Order"]);
    assert_has(
        &messages(&project, Severity::Warning),
        "Customer has no version column yet, so saves of it are not checked: write the next migration step, which adds it (add version Customer)",
    );
    assert_eq!(project.version_column("Customer"), None);

    let second = "migration \"Versions\"\n    add version Customer\n    add version Order\n";
    let project = load(&[("0001.clarity", FIRST), ("0002.clarity", second)]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    assert!(pending(&project).is_empty());
    assert!(migrate::schema(&project, "main").table("Customer").unwrap().version);
    assert_eq!(project.version_column("Customer").as_deref(), Some("row_version"));
}

#[test]
fn a_new_entity_has_its_version_from_its_first_step() {
    let project = load(&[]);
    let first = migrate::step_text("Customers and orders", &migrate::pending(&project, "main", &[]));
    assert!(first.contains("    add entity Customer\n        version\n        name: text(80)\n"), "{first}");
    let project = load(&[("0001.clarity", first.as_str())]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    assert!(pending(&project).is_empty());
    assert_eq!(project.version_column("Order").as_deref(), Some("row_version"));
}

#[test]
fn a_version_is_added_once() {
    let twice = "migration \"Again\"\n    add version Customer\n";
    let first = FIRST.replace("    add entity Customer\n", "    add entity Customer\n        version\n");
    let project = load(&[("0001.clarity", first.as_str()), ("0002.clarity", twice)]);
    assert_has(&messages(&project, Severity::Error), "Customer has its version column already");
    let unknown = "migration \"Nobody\"\n    add version Invoice\n";
    let project = load(&[("0001.clarity", FIRST), ("0002.clarity", unknown)]);
    assert_has(&messages(&project, Severity::Error), "there is no entity 'Invoice'");
}

#[test]
fn a_project_loaded_without_its_steps_is_versioned_from_the_dictionary() {
    let project = load_converted([("shop.clarity", SHOP)]);
    assert_eq!(project.version_column("Customer").as_deref(), Some("row_version"));
}

const ERP: &str = "module erp
source erp

entity Invoice
    table \"tblInvoice\"
    key \"InvoiceID\"
    version \"RowVersion\"
    number: text(20)

entity Supplier
    version none
    name: text(80)

entity Payment
    amount: money
";

fn external(erp: &str, readonly: bool) -> Project {
    let erp_source = SourceRef { readonly, ..SourceRef::sql("erp").external() };
    load_in([("erp.clarity", erp)], [], &[SourceRef::sql("main"), erp_source])
}

#[test]
fn an_external_entity_names_its_version_column() {
    let project = external(ERP, false);
    assert!(messages(&project, Severity::Error).is_empty(), "{:#?}", project.diagnostics);
    let invoice = project.entity("Invoice").unwrap();
    assert_eq!(invoice.version.as_deref(), Some("RowVersion"));
    assert!(project.entity("Supplier").unwrap().version_none);
    assert_eq!(project.version_column("Invoice").as_deref(), Some("RowVersion"));
    assert_eq!(project.version_column("Supplier"), None);
    assert_eq!(project.version_column("Payment"), None);
    // Without either line the saves are not checked, which is said.
    let warnings = messages(&project, Severity::Warning);
    assert_eq!(
        warnings,
        ["Payment has no version column, so saves of it are not checked: name its column with version \"Column\", or write version none"]
    );
    // Its tables are not Clarity's: no step adds a version there.
    assert!(migrate::pending(&project, "erp", &[]).iter().all(|op| !migrate::op_text(op).contains("version")));
}

#[test]
fn a_readonly_external_entity_needs_no_version() {
    let project = external(ERP, true);
    assert!(messages(&project, Severity::Warning).is_empty(), "{:#?}", project.diagnostics);
}

#[test]
fn version_lines_are_checked() {
    let owned = "module shop\n\nentity Customer\n    version \"RowVersion\"\n    name: text(80)\n";
    let project = load_in([("shop.clarity", owned)], [], &[]);
    assert_has(
        &messages(&project, Severity::Error),
        "Customer is stored in a database Clarity owns, which keeps its version in row_version itself; remove the version line",
    );
    let twice = ERP.replace("    version none\n", "    version none\n    version \"Stamp\"\n");
    assert_has(&messages(&external(&twice, false), Severity::Error), "duplicate 'version'");
    let file = "module feed\nsource feed\n\nentity Line\n    version \"Stamp\"\n    name: text(80)\n";
    let project = load_in([("feed.clarity", file)], [], &[SourceRef::sql("main"), SourceRef::sql("feed").external().file()]);
    assert_has(&messages(&project, Severity::Error), "Line is kept in a file, which is its own version; remove the version line");
    assert!(messages(&project, Severity::Warning).is_empty(), "{:#?}", project.diagnostics);
    let web = "module pay\nsource pay\n\nentity Charge\n    version none\n    amount: money\n";
    let project = load_in([("pay.clarity", web)], [], &[SourceRef::sql("main"), SourceRef::web("pay")]);
    assert_has(&messages(&project, Severity::Error), "Charge comes from a web API, which has no version column; remove the version line");
    let record = "module shop\n\nrecord Dot\n    version none\n    x: integer\n";
    assert_has(&messages(&load_in([("shop.clarity", record)], [], &[]), Severity::Error), "a record is never stored, so it has no 'version'");
}

const NAMED: &str = "module shop

entity Person
    name: text(80), label @personName

entity Customer
    name: text(80), label @customerName
    changedBy: ref Person, label @customerChangedBy
    savedSince @customerChangedSince
";

fn translated(source: &str, en: &str) -> Project {
    let mut project = load_converted([("shop.clarity", source)]);
    project.locale = Locale { language: Some("en".into()), currency: None };
    let text = format!("translation en\n    personName \"Name\"\n    customerName \"Name\"\n    customerChangedBy \"Changed by\"\n{en}");
    translations::attach(&mut project, [("translations/en.clarity", text.as_str())]);
    project
}

#[test]
fn an_entity_names_the_text_of_its_refusal() {
    let project = translated(NAMED, "    customerChangedSince \"{current.changedBy.name} changed {current.name} since you opened it\"\n");
    assert!(messages(&project, Severity::Error).is_empty(), "{:#?}", project.diagnostics);
    assert_eq!(project.entity("Customer").unwrap().saved_since.as_deref(), Some("customerChangedSince"));
    assert!(translations::keys(&project).iter().any(|k| k.key == "customerChangedSince" && k.places == ["entity Customer"]));

    let project = translated(NAMED, "    customerChangedSince \"{current.phone} changed\"\n");
    assert_has(&messages(&project, Severity::Error), "@customerChangedSince needs a value for {current.phone}");
    let project = translated(NAMED, "");
    assert_has(&messages(&project, Severity::Error), "key 'customerChangedSince' has no value in en");

    let written = NAMED.replace("savedSince @customerChangedSince", "savedSince \"Changed since\"");
    assert_has(
        &messages(&load_converted([("shop.clarity", written.as_str())]), Severity::Error),
        "the text of savedSince is a key: write savedSince @customerChangedSince, with its text in the language files",
    );
    let record = "module shop\n\nrecord Dot\n    savedSince @dotChanged\n    x: integer\n";
    assert_has(&messages(&load_converted([("shop.clarity", record)]), Severity::Error), "a record is never stored, so it has no 'savedSince'");
}

const CODE: &str = "module shop

entity Customer
    name: text(80)

record Dot
    x: integer

action keep(customer: Customer)
    effects db.write
    save customer anyway
    reload customer
";

fn code_errors(source: &str) -> Vec<String> {
    messages(&load_converted([("shop.clarity", source)]), Severity::Error)
}

#[test]
fn save_anyway_and_reload_of_a_record() {
    let project = load_converted([("shop.clarity", CODE)]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let action = &project.files[0].functions[0];
    assert!(matches!(&action.body[0].kind, StmtKind::Save { anyway: true, .. }));
    assert!(matches!(&action.body[1].kind, StmtKind::Reload { data } if data.name == "customer"));

    assert_has(&code_errors(&CODE.replace("    effects db.write\n", "")), "action 'keep' saves but does not declare 'effects db.write'");
    assert_has(&code_errors(&CODE.replace("save customer anyway", "save 3 anyway")), "'save' needs a record, not integer");
    let dot = CODE.replace("    reload customer\n", "    let dot = new Dot\n    reload dot\n");
    assert_has(&code_errors(&dot), "Dot is a record, which is never stored, so 'reload' has nothing to read");
    let number = CODE.replace("    reload customer\n", "    let n = 3\n    reload n\n");
    assert_has(&code_errors(&number), "'reload' reads a record again, and n is integer");
    let unknown = CODE.replace("    reload customer\n", "    reload nobody\n");
    assert_has(&code_errors(&unknown), "unknown name 'nobody'");
}
