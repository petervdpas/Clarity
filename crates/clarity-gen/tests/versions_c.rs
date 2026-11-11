//! A save never overwrites someone else's change (docs/APP-CAPABILITIES-PLAN.md
//! step 5) on the C stack: a versioned entity names its version column and
//! carries the version, an entity's `savedSince` text is filled from the
//! record as stored now, `save ... anyway` and `reload` of a record call the
//! runtime, and an entity whose versions step is not written yet carries
//! nothing of versions.

use clarity_spec::model::Locale;
use clarity_spec::{Project, Severity, SourceRef};

const SHOP: &str = "module shop

entity Person
    name: text(80), label @personName

entity Customer
    name: text(80), label @customerName
    visits: integer, label @customerVisits
    changedBy: ref Person, label @customerChangedBy
    savedSince @customerChangedSince

action keep(customer: Customer)
    effects db.write
    save customer anyway
    reload customer

function fresh(customer: Customer) -> Customer
    let found = customer
    reload found
    return found

action touchAll(customers: list<Customer>)
    effects db.write
    for c in customers
        reload c
        save c

app Shop
    title @shopTitle
    main Main

page Main: window
    data customers: list<Customer>
    label @hello name hello at 1,1 span 4
    button @hello name again at 1,2 span 2 on click: againClicked

code Main
    handler againClicked
        let first = customers[1]
        reload first
        reload customers
";

const ERP: &str = "module erp
source erp

entity Invoice
    table \"tblInvoice\"
    key \"InvoiceID\"
    version \"RowVersion\"
    number: text(20), label @invoiceNumber

entity Supplier
    version none
    name: text(80), label @supplierName
";

const FIRST: &str = "migration \"Shop\"
    add entity Person
        name: text(80)
    add entity Customer
        name: text(80)
        visits: integer
        changedBy: ref Person
";

const VERSIONS: &str = "migration \"Versions\"
    add version Person
    add version Customer
";

const EN: &str = "translation en
    shopTitle \"Shop\"
    hello \"Hello\"
    personName \"Name\"
    customerName \"Name\"
    customerVisits \"Visits\"
    customerChangedBy \"Changed by\"
    invoiceNumber \"Number\"
    supplierName \"Name\"
    customerChangedSince \"{current.changedBy.name} changed {current.name} since you opened it ({current.visits} visits)\"
";

fn project(steps: &[(&str, &str)]) -> Project {
    let sources = clarity_spec::converted([("shop/shop.clarity", SHOP), ("erp/erp.clarity", ERP)]);
    let mut project = clarity_spec::load_in(sources.iter().map(|(p, t)| (p.as_str(), t.as_str())), steps.iter().copied(), &[SourceRef::sql("main"), SourceRef::sql("erp").external()]);
    project.locale = Locale { language: Some("en".into()), currency: None };
    clarity_spec::translations::attach(&mut project, [("translations/en.clarity", EN)]);
    let errors: Vec<String> = project.diagnostics.iter().filter(|d| d.severity == Severity::Error).map(|d| d.message.clone()).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    project
}

fn files(steps: &[(&str, &str)]) -> Vec<clarity_gen::OutFile> {
    let settings = clarity_project::parse("project Shop\n    stack c desktop\n    datasource main sqlite \"data/shop.db\"\n    datasource erp sqlite \"legacy/erp.db\" external\n").unwrap();
    clarity_gen::generate(&project(steps), &settings).unwrap_or_else(|e| panic!("{e:#?}"))
}

fn file(files: &[clarity_gen::OutFile], path: &str) -> String {
    files.iter().find(|f| f.path == path).unwrap_or_else(|| panic!("no {path}")).text.clone()
}

fn versioned() -> Vec<clarity_gen::OutFile> {
    files(&[("migrations/main/0001.clarity", FIRST), ("migrations/main/0002.clarity", VERSIONS)])
}

fn has(text: &str, wanted: &str) {
    assert!(text.contains(wanted), "no {wanted:?} in:\n{text}");
}

/// From `start` to the next line that is just `};`.
fn block<'a>(text: &'a str, start: &str) -> &'a str {
    let rest = &text[text.find(start).unwrap_or_else(|| panic!("no {start:?} in:\n{text}"))..];
    &rest[..rest.find("\n};\n").unwrap_or(rest.len())]
}

#[test]
fn a_versioned_entity_names_its_column_and_carries_the_version() {
    let out = versioned();
    let header = file(&out, "src/records.h");
    has(block(&header, "struct Customer {"), "    int64_t row_version;");
    has(block(&header, "struct Invoice {"), "    int64_t row_version;");
    assert!(!block(&header, "struct Supplier {").contains("row_version"));
    let records = file(&out, "src/records.c");
    has(block(&records, "const ClarityEntity customer_entity = {"), "    .version = \"row_version\",\n    .version_at = offsetof(Customer, row_version),\n    .saved_since = customer_saved_since,");
    has(block(&records, "const ClarityEntity invoice_entity = {"), "    .version = \"RowVersion\",\n    .version_at = offsetof(Invoice, row_version),");
    assert!(!block(&records, "const ClarityEntity invoice_entity = {").contains(".saved_since"));
    let supplier = block(&records, "const ClarityEntity supplier_entity = {");
    assert!(!supplier.contains(".version") && !supplier.contains(".saved_since"), "{supplier}");
    // The version travels with the record's JSON, so a window's record keeps the one it read.
    has(&records, "    cJSON_AddItemToObject(out, \"rowVersion\", clarity_json_integer(r->row_version));\n");
    has(&records, "    if (!clarity_json_get_integer(in, \"rowVersion\", &r->row_version)) return false;\n");
    let supplier_json = &records[records.find("cJSON* supplier_json(const Supplier* r)").unwrap()..];
    assert!(!supplier_json[..supplier_json.find("\n}\n").unwrap()].contains("rowVersion"));
}

#[test]
fn saved_since_fills_the_entity_text_from_the_stored_record() {
    let records = file(&versioned(), "src/records.c");
    has(
        &records,
        "static bool customer_saved_since(ClarityDb* db, const void* record) {\n    const Customer* current = record;\n    ClarityText* text = clarity_key_text(\"customerChangedSince\");\n    text = clarity_text_with(text, \"current.id\", clarity_slot_integer(current->id, NULL));\n    text = clarity_text_with(text, \"current.name\", clarity_slot_text(current->name));\n    text = clarity_text_with(text, \"current.visits\", clarity_slot_integer(current->visits, NULL));\n    text = clarity_text_with(text, \"current.changedBy\", clarity_slot_integer(current->changedBy, NULL));\n    text = clarity_text_with(text, \"current.changedBy.id\", clarity_slot_integer(current->changedBy, NULL));\n    Person changedBy_ref = {0};\n    if (current->changedBy != 0 && person_load(db, current->changedBy, &changedBy_ref)) {\n        text = clarity_text_with(text, \"current.changedBy.name\", clarity_slot_text(changedBy_ref.name));\n    }\n    return clarity_refuse(text);\n}\n",
    );
    assert!(!records.contains("person_saved_since"));
}

#[test]
fn save_anyway_and_reload_of_a_record_call_the_runtime() {
    let out = versioned();
    let header = file(&out, "src/records.h");
    has(&header, "bool customer_save_anyway(ClarityDb* db, Customer* record);\n");
    has(&header, "bool customer_reload(ClarityDb* db, Customer* record);\n");
    let records = file(&out, "src/records.c");
    has(&records, "bool customer_save_anyway(ClarityDb* db, Customer* record) { return clarity_save_anyway(db, &customer_entity, record); }");
    has(&records, "bool customer_reload(ClarityDb* db, Customer* record) { return clarity_reload(db, &customer_entity, record); }");
    let shop = file(&out, "src/modules/shop.c");
    has(&shop, "    if (!customer_save_anyway(db, customer)) return false;\n    if (!customer_reload(db, customer)) return false;\n");
    // A local holds a record of its own; a loop's variable points at the list's.
    has(&shop, "if (!customer_reload(db, &found)) return false;\n");
    has(&shop, "if (!customer_reload(db, c)) return false;\n");
    // In a handler, a local is read again; a window's data loads again.
    has(&shop, "if (!customer_reload(db, &first)) return false;\n");
    has(&shop, "if (!customer_all(db, &w->customers)) return false;\n");
}

#[test]
fn without_the_versions_step_nothing_of_versions_is_emitted() {
    let out = files(&[("migrations/main/0001.clarity", FIRST)]);
    let records = file(&out, "src/records.c");
    let header = file(&out, "src/records.h");
    for entity in ["customer", "person"] {
        let descriptor = block(&records, &format!("const ClarityEntity {entity}_entity = {{"));
        assert!(!descriptor.contains(".version") && !descriptor.contains(".saved_since"), "{descriptor}");
    }
    assert!(!records.contains("customer_saved_since"), "{records}");
    let customer_json = &records[records.find("cJSON* customer_json(const Customer* r)").unwrap()..];
    assert!(!customer_json[..customer_json.find("\n}\n").unwrap()].contains("rowVersion"));
    for entity in ["struct Customer {", "struct Person {"] {
        assert!(!block(&header, entity).contains("row_version"), "{header}");
    }
    // `save ... anyway` of an unchecked record is a save.
    has(&file(&out, "src/modules/shop.c"), "if (!customer_save_anyway(db, customer)) return false;\n");
}
