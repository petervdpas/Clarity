//! A project with an external SQLite database: loading it checks the
//! dictionary against the database's tables.

use clarity_project::load_modules;
use clarity_spec::Severity;

const PROJECT: &str = "project Shop\n    stack rust web desktop\n    datasource main sqlite \"data/shop.db\"\n    datasource erp sqlite \"legacy/erp.db\" external\n";

const ERP: &str = "module erp\nsource erp\n\nentity Invoice\n    table \"tblInvoice\"\n    key \"InvoiceID\"\n    number: text(20), column \"InvNo\", label @invoiceNumberLabel\n";

/// The default language, with the keys ERP uses.
const EN: &str = "translation en\n    invoiceNumberLabel \"Number\"\n    yes \"Yes\"\n    no \"No\"\n    ok \"OK\"\n    cancel \"Cancel\"\n    dismiss \"Dismiss\"\n    undoChangedSince \"This was changed since, so it cannot be taken back\"\n";

#[test]
fn loading_checks_external_tables() {
    let root = std::env::temp_dir().join(format!("clarity-external-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let dir = root.join("shop");
    std::fs::create_dir_all(dir.join("legacy")).unwrap();
    std::fs::write(root.join("shop.clarity"), PROJECT).unwrap();
    std::fs::write(dir.join("erp.clarity"), ERP).unwrap();
    std::fs::create_dir_all(dir.join("translations")).unwrap();
    std::fs::write(dir.join("translations/en.clarity"), EN).unwrap();

    // Without the database, its tables are not checked.
    let project = load_modules(&dir).unwrap();
    let warnings: Vec<&str> = project.diagnostics.iter().filter(|d| d.severity == Severity::Warning).map(|d| d.message.as_str()).collect();
    assert!(warnings.iter().any(|w| w.starts_with("data source erp: cannot open") && w.ends_with("its tables were not checked")), "{warnings:#?}");

    // With it, a column the table does not have is an error.
    rusqlite::Connection::open(dir.join("legacy/erp.db"))
        .unwrap()
        .execute_batch("CREATE TABLE tblInvoice (InvoiceID INTEGER PRIMARY KEY, Number TEXT);")
        .unwrap();
    let project = load_modules(&dir).unwrap();
    let errors: Vec<&str> = project.diagnostics.iter().filter(|d| d.severity == Severity::Error).map(|d| d.message.as_str()).collect();
    assert_eq!(errors, ["Invoice.number: table 'tblInvoice' has no column 'InvNo'"]);
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn importing_writes_a_module_of_mapped_entities() {
    let root = std::env::temp_dir().join(format!("clarity-import-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let dir = root.join("shop");
    std::fs::create_dir_all(dir.join("legacy")).unwrap();
    std::fs::write(root.join("shop.clarity"), PROJECT).unwrap();
    rusqlite::Connection::open(dir.join("legacy/erp.db"))
        .unwrap()
        .execute_batch(
            "CREATE TABLE tblInvoice (InvoiceID INTEGER PRIMARY KEY, InvNo TEXT NOT NULL);
             CREATE TABLE Lines (OrderNo TEXT, Line INTEGER, PRIMARY KEY (OrderNo, Line));",
        )
        .unwrap();

    let tables = clarity_project::source_tables(&dir, "erp").unwrap();
    let listed: Vec<(&str, Option<&str>)> = tables.iter().map(|t| (t.name.as_str(), t.not_importable.as_deref())).collect();
    assert_eq!(
        listed,
        [("Lines", Some("its key is not one integer column, which is not supported yet")), ("tblInvoice", None)]
    );

    let done = clarity_project::import_tables(&dir, "erp", "erp", &["tblInvoice".to_string()]).unwrap();
    assert_eq!(done.path, dir.join("erp.clarity"));
    let project = load_modules(&dir).unwrap();
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    assert_eq!(project.source_of("Invoice"), Some("erp"));

    // The module is not written over.
    let again = clarity_project::import_tables(&dir, "erp", "erp", &["tblInvoice".to_string()]).unwrap_err();
    assert!(again.contains("there is already a module file"), "{again}");
    // Only external SQL databases are imported.
    let main = clarity_project::source_tables(&dir, "main").unwrap_err();
    assert!(main.contains("data source main is not external"), "{main}");
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn taking_ownership_writes_a_baseline_and_drops_external() {
    let root = std::env::temp_dir().join(format!("clarity-own-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let dir = root.join("shop");
    std::fs::create_dir_all(dir.join("legacy")).unwrap();
    std::fs::write(root.join("shop.clarity"), PROJECT).unwrap();
    rusqlite::Connection::open(dir.join("legacy/erp.db"))
        .unwrap()
        .execute_batch("CREATE TABLE tblInvoice (InvoiceID INTEGER PRIMARY KEY, InvNo TEXT NOT NULL UNIQUE);")
        .unwrap();

    // Nothing to take over before there are entities.
    let empty = clarity_project::take_ownership(&dir, "erp").unwrap_err();
    assert!(empty.contains("no entities"), "{empty}");

    clarity_project::import_tables(&dir, "erp", "erp", &["tblInvoice".to_string()]).unwrap();
    let done = clarity_project::take_ownership(&dir, "erp").unwrap();
    assert_eq!(done.step, dir.join("migrations/erp/0001.clarity"));
    let step = std::fs::read_to_string(&done.step).unwrap();
    assert!(step.contains("migration \"Taken over\" baseline\n    add entity Invoice\n        table \"tblInvoice\"\n        key \"InvoiceID\"\n"), "{step}");

    let settings = clarity_project::load(&root.join("shop.clarity")).settings.unwrap();
    assert!(!settings.datasources[1].external);
    // The project is in step with its database: nothing pending, no problems.
    let project = load_modules(&dir).unwrap();
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);

    let again = clarity_project::take_ownership(&dir, "erp").unwrap_err();
    assert!(again.contains("is not external"), "{again}");
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn every_sqlite_file_is_found_from_the_project_folder() {
    let settings = clarity_project::parse(PROJECT).unwrap();
    // A folder this system calls absolute, and paths compared as paths: the
    // separator between them is the system's own.
    let dir = std::path::absolute("/work/shop").unwrap();
    let built = clarity_project::build_settings(&dir, &settings);
    assert_eq!(std::path::Path::new(&built.datasources[0].url), dir.join("data/shop.db"));
    assert_eq!(std::path::Path::new(&built.datasources[1].url), dir.join("legacy/erp.db"));
}
