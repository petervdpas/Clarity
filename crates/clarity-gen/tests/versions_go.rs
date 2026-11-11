//! A save never overwrites someone else's change (docs/APP-CAPABILITIES-PLAN.md
//! step 5) on the Go stack: a versioned entity names its version column and
//! carries the version, an entity's `savedSince` text is filled from the
//! record as stored now, `save ... anyway` and `reload` of a record call the
//! runtime, and an entity whose versions step is not written yet carries
//! nothing of versions.

use clarity_project::{Backend, DataSource, Settings, SourceKind};
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

app Shop
    title @shopTitle
    main Main

page Main: window
    title @shopTitle
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
    customerChangedSince \"{current.changedBy.name} changed {current.name} since you opened it ({current.visits} visits)\"
    personName \"Name\"
    personPassword \"Password\"
    customerName \"Name\"
    customerVisits \"Visits\"
    customerChangedBy \"Changed by\"
    invoiceNumber \"Number\"
    supplierName \"Name\"
    shopTitle \"Shop\"
    signInButton \"Sign in\"
";

fn settings() -> Settings {
    let mut settings = clarity_project::starter("Shop", "shop");
    settings.stack.backend = Backend::Go;
    settings.datasources.push(DataSource { name: "erp".into(), kind: SourceKind::Sqlite, url: "legacy/erp.db".into(), auth: None, external: true, readonly: false, beside: false, openapi: None, picks: Vec::new(), pick_schemas: Vec::new(), paging: None, file: None });
    settings
}

fn project(shop: &str, steps: &[(&str, &str)], settings: &Settings) -> Project {
    let sources = clarity_spec::converted([("shop/shop.clarity", shop), ("erp/erp.clarity", ERP)]);
    let mut project = clarity_spec::load_in_signed(
        sources.iter().map(|(p, t)| (p.as_str(), t.as_str())),
        steps.iter().copied(),
        &[SourceRef::sql("main"), SourceRef::sql("erp").external()],
        settings.signin.as_ref(),
    );
    project.locale = Locale { language: Some("en".into()), currency: None };
    clarity_spec::translations::attach(&mut project, [("translations/en.clarity", EN)]);
    // Fields without label keys are fine here.
    let errors: Vec<String> = project.diagnostics.iter().filter(|d| d.severity == Severity::Error && !d.message.contains("label")).map(|d| d.message.clone()).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    project
}

fn module_with(shop: &str, steps: &[(&str, &str)], settings: &Settings, path: &str) -> String {
    let files = clarity_gen::generate(&project(shop, steps, settings), settings).unwrap_or_else(|e| panic!("{e:#?}"));
    files.iter().find(|f| f.path == path).unwrap_or_else(|| panic!("no {path}")).text.clone()
}

fn module_of(shop: &str, steps: &[(&str, &str)], path: &str) -> String {
    module_with(shop, steps, &settings(), path)
}

fn module(steps: &[(&str, &str)], path: &str) -> String {
    module_of(SHOP, steps, path)
}

fn versioned(path: &str) -> String {
    module(&[("migrations/main/0001.clarity", FIRST), ("migrations/main/0002.clarity", VERSIONS)], path)
}

fn has(text: &str, wanted: &str) {
    assert!(text.contains(wanted), "no {wanted:?} in:\n{text}");
}

/// The part of `text` from `from` up to `to` (or its end).
fn between<'a>(text: &'a str, from: &str, to: &str) -> &'a str {
    let rest = &text[text.find(from).unwrap_or_else(|| panic!("no {from:?} in:\n{text}"))..];
    &rest[..rest[from.len()..].find(to).map_or(rest.len(), |at| at + from.len())]
}

#[test]
fn a_versioned_entity_names_its_column_and_carries_the_version() {
    let shop = versioned("shop_module.go");
    has(&shop, "type Customer struct {\n\tID         int64  `json:\"id\"`\n\tName       string `json:\"name\"`\n\tVisits     int64  `json:\"visits\"`\n\tChangedBy  *int64 `json:\"changedBy\"`\n\tRowVersion int64  `json:\"rowVersion\"`\n}\n");
    has(&shop, "func (*Customer) VersionColumn() string {\n\treturn \"row_version\"\n}\n\n");
    has(&shop, "func (r *Customer) GetVersion() int64 {\n\treturn r.RowVersion\n}\n\n");
    has(&shop, "func (r *Customer) SetVersion(version int64) {\n\tr.RowVersion = version\n}\n\n");
    // The version is no column of Columns: the runtime reads and writes it itself.
    has(&shop, "func (*Customer) Columns() []string {\n\treturn []string{\"name\", \"visits\", \"changed_by\"}\n}\n");

    let erp = versioned("erp_module.go");
    has(&erp, "func (*Invoice) VersionColumn() string {\n\treturn \"RowVersion\"\n}\n\n");
    // `version none`: saves are not checked, nothing of versions.
    let supplier = between(&erp, "type Supplier struct", "\ntype ");
    for word in ["RowVersion", "VersionColumn", "GetVersion", "SetVersion"] {
        assert!(!supplier.contains(word), "{word} in:\n{supplier}");
    }
}

#[test]
fn saved_since_fills_the_entity_text_from_the_stored_record() {
    let shop = versioned("shop_module.go");
    has(
        &shop,
        "func (r *Customer) SavedSince(db *clarity.Db) clarity.Text {\n\ttext := clarity.KeyText(\"customerChangedSince\").\n\t\tWith(\"current.id\", clarity.SlotInteger(r.ID, \"\")).\n\t\tWith(\"current.name\", clarity.SlotText(r.Name)).\n\t\tWith(\"current.visits\", clarity.SlotInteger(r.Visits, \"\"))\n\tif r.ChangedBy != nil {\n\t\ttext = text.\n\t\t\tWith(\"current.changedBy\", clarity.SlotInteger(*r.ChangedBy, \"\")).\n\t\t\tWith(\"current.changedBy.id\", clarity.SlotInteger(*r.ChangedBy, \"\"))\n\t\tif changedByRecord, err := clarity.Get[Person](db, *r.ChangedBy); err == nil {\n\t\t\ttext = text.With(\"current.changedBy.name\", clarity.SlotText(changedByRecord.Name))\n\t\t}\n\t}\n\treturn text\n}\n\n",
    );
    // Without a savedSince line, the runtime's own savedSince.
    assert!(!shop.contains("func (r *Person) SavedSince("), "{shop}");
}

#[test]
fn save_anyway_and_reload_of_a_record_call_the_runtime() {
    let shop = versioned("shop_module.go");
    let keep = between(&shop, "func Keep(", "\n}\n");
    has(keep, "if err := clarity.SaveAnyway(db, customer); err != nil {");
    has(keep, "if err := clarity.Reload(db, customer); err != nil {");
    let fresh = between(&shop, "func Fresh(", "\n}\n");
    has(fresh, "if err := clarity.Reload(db, found); err != nil {");
}

#[test]
fn a_reloaded_record_is_checked_against_readable_when() {
    // `readable when` asks for sign-in: Person signs in by name and password.
    let readable = SHOP
        .replace("entity Person\n    name: text(80), label @personName\n", "entity Person\n    name: text(80), label @personName\n    passwordHash: text, hashed, label @personPassword\n")
        .replace("    savedSince @customerChangedSince\n", "    savedSince @customerChangedSince\n    readable when customer.visits >= 0\n")
        + "\npage SignIn: signin\n    textbox name name at 1,1 span 4\n    password name password at 1,2 span 4\n    button @signInButton name signIn at 1,3 span 2 on click -> signin password\n";
    let first = FIRST.replace("    add entity Person\n        name: text(80)\n", "    add entity Person\n        name: text(80)\n        passwordHash: text, hashed\n");
    let mut signed = settings();
    signed.signin = clarity_project::parse("project Shop\n    stack go desktop\n    signin Person on desktop\n        password name passwordHash\n").unwrap().signin;
    let shop = module_with(&readable, &[("migrations/main/0001.clarity", first.as_str()), ("migrations/main/0002.clarity", VERSIONS)], &signed, "shop_module.go");
    let keep = between(&shop, "func Keep(", "\n}\n");
    has(keep, "if err := clarity.Reload(db, customer); err != nil {\n\t\treturn err\n\t}\n\tif err := readableCustomer(db, customer); err != nil {\n\t\treturn err\n\t}\n");
}

#[test]
fn without_the_versions_step_nothing_of_versions_is_emitted() {
    let shop = module(&[("migrations/main/0001.clarity", FIRST)], "shop_module.go");
    for word in ["RowVersion", "rowVersion", "VersionColumn", "GetVersion", "SetVersion", "SavedSince"] {
        assert!(!shop.contains(word), "{word} in:\n{shop}");
    }
    // `save ... anyway` of an unchecked record is a save.
    has(&shop, "clarity.SaveAnyway(db, customer)");
}
