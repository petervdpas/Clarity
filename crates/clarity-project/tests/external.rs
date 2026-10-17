//! A project with an external SQLite database: loading it checks the
//! dictionary against the database's tables.

use clarity_project::load_modules;
use clarity_spec::Severity;

const PROJECT: &str = "project Shop\n    stack rust web desktop\n    datasource main sqlite \"data/shop.db\"\n    datasource erp sqlite \"legacy/erp.db\" external\n";

const ERP: &str = "module erp\nsource erp\n\nentity Invoice\n    table \"tblInvoice\"\n    key \"InvoiceID\"\n    number: text(20), column \"InvNo\"\n";

#[test]
fn loading_checks_external_tables() {
    let root = std::env::temp_dir().join(format!("clarity-external-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let dir = root.join("shop");
    std::fs::create_dir_all(dir.join("legacy")).unwrap();
    std::fs::write(root.join("shop.clarity"), PROJECT).unwrap();
    std::fs::write(dir.join("erp.clarity"), ERP).unwrap();

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
