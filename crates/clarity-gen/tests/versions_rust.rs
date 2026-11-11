//! A save never overwrites someone else's change (docs/APP-CAPABILITIES-PLAN.md
//! step 5) on the Rust stack: a versioned entity names its version column
//! and carries the version, an entity's `savedSince` text is filled from the
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

function again(customer: Customer) -> Customer
    reload customer
    return customer

app Shop
    title @shopTitle
    main Main

page Main: window
    label @hello name hello at 1,1 span 4
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

fn settings() -> Settings {
    let mut settings = clarity_project::starter("Shop", "shop");
    assert_eq!(settings.stack.backend, Backend::Rust);
    settings.datasources.push(DataSource { name: "erp".into(), kind: SourceKind::Sqlite, url: "legacy/erp.db".into(), auth: None, external: true, readonly: false, beside: false, openapi: None, picks: Vec::new(), pick_schemas: Vec::new(), paging: None, file: None });
    settings
}

fn project(steps: &[(&str, &str)]) -> Project {
    let sources = clarity_spec::converted([("shop/shop.clarity", SHOP), ("erp/erp.clarity", ERP)]);
    let mut project = clarity_spec::load_in(sources.iter().map(|(p, t)| (p.as_str(), t.as_str())), steps.iter().copied(), &[SourceRef::sql("main"), SourceRef::sql("erp").external()]);
    project.locale = Locale { language: Some("en".into()), currency: None };
    clarity_spec::translations::attach(&mut project, [("translations/en.clarity", EN)]);
    let errors: Vec<String> = project.diagnostics.iter().filter(|d| d.severity == Severity::Error).map(|d| d.message.clone()).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    project
}

fn module(steps: &[(&str, &str)], path: &str) -> String {
    let files = clarity_gen::generate(&project(steps), &settings()).unwrap_or_else(|e| panic!("{e:#?}"));
    files.iter().find(|f| f.path == path).unwrap_or_else(|| panic!("no {path}")).text.clone()
}

fn versioned(path: &str) -> String {
    module(&[("migrations/main/0001.clarity", FIRST), ("migrations/main/0002.clarity", VERSIONS)], path)
}

fn has(text: &str, wanted: &str) {
    assert!(text.contains(wanted), "no {wanted:?} in:\n{text}");
}

#[test]
fn a_versioned_entity_names_its_column_and_carries_the_version() {
    let shop = versioned("src/modules/shop.rs");
    has(&shop, "pub struct Customer {\n    pub id: Id,\n");
    has(&shop, "    #[serde(default, rename = \"rowVersion\")]\n    pub row_version: i64,\n}");
    has(&shop, "            row_version: 0,\n        }\n    }\n}");
    has(&shop, "    const VERSION: Option<&'static str> = Some(\"row_version\");\n");
    has(&shop, "    fn version(&self) -> i64 {\n        self.row_version\n    }\n\n    fn set_version(&mut self, version: i64) {\n        self.row_version = version;\n    }\n");
    // Built from values, a record is at version 0 until the runtime sets the one read.
    has(&shop, "            row_version: 0,\n        })\n    }");

    let erp = versioned("src/modules/erp.rs");
    has(&erp, "    const VERSION: Option<&'static str> = Some(\"RowVersion\");\n");
    // `version none`: saves are not checked, nothing of versions.
    let supplier = &erp[erp.find("pub struct Supplier").unwrap()..];
    assert!(!supplier.contains("row_version") && !supplier.contains("VERSION"), "{supplier}");
}

#[test]
fn saved_since_fills_the_entity_text_from_the_stored_record() {
    let shop = versioned("src/modules/shop.rs");
    has(
        &shop,
        "    fn saved_since(db: &Db, current: &Self) -> Text {\n        let mut text = Text::key(\"customerChangedSince\")\n            .with(\"current.id\", Slot::integer(current.id, None))\n            .with(\"current.name\", Slot::text(current.name.clone()))\n            .with(\"current.visits\", Slot::integer(current.visits, None))\n            .with(\"current.changedBy\", Slot::integer(current.changedBy.unwrap_or(0), None))\n            .with(\"current.changedBy.id\", Slot::integer(current.changedBy.unwrap_or(0), None));\n        // A missing reference leaves its slots empty.\n        if let Some(referred_changed_by) = current.changedBy.and_then(|id| db.get::<crate::modules::shop::Person>(id).ok()) {\n            text = text\n                .with(\"current.changedBy.name\", Slot::text(referred_changed_by.name.clone()));\n        }\n        text\n    }\n",
    );
    // Without a savedSince line, the runtime's own savedSince.
    let person = &shop[shop.find("impl Record for Person").unwrap()..];
    let person = &person[..person.find("impl Record for Customer").unwrap_or(person.len())];
    assert!(!person.contains("fn saved_since"), "{person}");
}

#[test]
fn save_anyway_and_reload_of_a_record_call_the_runtime() {
    let shop = versioned("src/modules/shop.rs");
    has(&shop, "pub fn keep(db: &Db, customer: &mut Customer) -> Result<()> {\n    db.save_anyway(customer)?;\n    db.reload(customer)?;\n");
    has(&shop, "    db.reload(&mut found)?;\n");
    // A function reads its record; one it reads again is its own copy.
    has(&shop, "pub fn again(db: &Db, customer: &Customer) -> Result<Customer> {\n    let mut customer = customer.clone();\n    db.reload(&mut customer)?;\n");
}

#[test]
fn without_the_versions_step_nothing_of_versions_is_emitted() {
    let shop = module(&[("migrations/main/0001.clarity", FIRST)], "src/modules/shop.rs");
    for word in ["row_version", "VERSION", "fn version", "fn set_version", "fn saved_since"] {
        assert!(!shop.contains(word), "{word} in:\n{shop}");
    }
    // `save ... anyway` of an unchecked record is a save.
    has(&shop, "db.save_anyway(customer)?;\n");
}

/// The versioned shop builds on the runtime, `savedSince`, `anyway` and
/// `reload` included. A whole build, so ignored: run on its own with
/// `cargo test -p clarity-gen --test versions_rust -- --ignored` (it shares
/// the examples' target/g8c-shared).
#[test]
#[ignore]
fn the_versioned_shop_builds_on_the_runtime() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let runtime = root.join("runtimes/rust").canonicalize().unwrap();
    let target = root.join("target/g8c-shared");
    let ui = std::env::var_os("CLARITY_UI").map(std::path::PathBuf::from).unwrap_or_else(|| root.join("target/clarity-ui"));
    let out = root.join("target/generated").join(format!("clarity-gen-versions-rust-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).unwrap();
    let project = project(&[("migrations/main/0001.clarity", FIRST), ("migrations/main/0002.clarity", VERSIONS)]);
    clarity_gen::write_out(&out, &clarity_gen::generate(&project, &settings()).unwrap(), &["target"]).unwrap();
    std::fs::create_dir_all(out.join(".cargo")).unwrap();
    std::fs::write(out.join(".cargo/config.toml"), format!("[patch.crates-io]\nclarity-runtime = {{ path = {:?} }}\n", runtime.display().to_string())).unwrap();
    let status = std::process::Command::new("cargo").args(["build", "--quiet"]).current_dir(&out).env("CARGO_TARGET_DIR", &target).env("CLARITY_UI", &ui).status().unwrap();
    assert!(status.success(), "the generated shop does not build; it is in {}", out.display());
    std::fs::remove_dir_all(&out).unwrap();
}
