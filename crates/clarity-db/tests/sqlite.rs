//! An existing SQLite database: reading its schema, importing tables into
//! the data dictionary, and checking that the dictionary still fits it.

use std::path::PathBuf;

use clarity_db::{drift, import, read_sqlite};
use clarity_spec::{Severity, SourceRef, load_in};

const LEGACY: &str = "
CREATE TABLE tblCustomers (
    CustID INTEGER PRIMARY KEY,
    CustName VARCHAR(80) NOT NULL,
    Email TEXT UNIQUE,
    CreditLimit NUMERIC(10,2),
    Active BOOLEAN,
    Since DATE
);
CREATE TABLE tblInvoice (
    InvoiceID INTEGER PRIMARY KEY,
    InvNo VARCHAR(20) NOT NULL UNIQUE,
    CustID INTEGER REFERENCES tblCustomers (CustID),
    Amount REAL,
    Issued DATETIME,
    Scan BLOB
);
CREATE TABLE OrderLines (OrderNo TEXT, Line INTEGER, PRIMARY KEY (OrderNo, Line));
";

/// A fresh copy of the legacy database, in its own folder.
fn legacy(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("clarity-db-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("erp.db");
    rusqlite::Connection::open(&path).unwrap().execute_batch(LEGACY).unwrap();
    path
}

const IMPORTED: &str = "// Imported by Clarity from data source erp.
module erp

source erp

entity Customers
    table \"tblCustomers\"
    key \"CustID\"
    cust_name: text(80), required, column \"CustName\"
    email: text, unique, column \"Email\"
    credit_limit: decimal(10, 2), column \"CreditLimit\"
    active: bool, column \"Active\"
    since: date, column \"Since\"

entity Invoice
    table \"tblInvoice\"
    key \"InvoiceID\"
    inv_no: text(20), required, unique, column \"InvNo\"
    cust_id: ref Customers, column \"CustID\"
    amount: decimal(18, 6), column \"Amount\"
    issued: datetime, column \"Issued\"
    // Scan is not imported: binary data is not supported yet.
";

#[test]
fn reads_the_tables_of_an_existing_database() {
    let path = legacy("read");
    let schema = read_sqlite(&path).unwrap();
    let names: Vec<&str> = schema.tables.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(names, ["OrderLines", "tblCustomers", "tblInvoice"]);
    let invoice = schema.table("tblInvoice").unwrap();
    assert_eq!(invoice.key(), Some("InvoiceID"));
    assert_eq!(invoice.foreign_keys, [("CustID".to_string(), "tblCustomers".to_string())]);
    assert!(invoice.column("InvNo").unwrap().unique);
    assert!(invoice.column("InvNo").unwrap().not_null);
    assert_eq!(schema.table("OrderLines").unwrap().key(), None);
    std::fs::remove_dir_all(path.parent().unwrap()).unwrap();

    assert!(read_sqlite(&path).unwrap_err().contains("cannot open"));
}

#[test]
fn imports_tables_as_entities_mapped_onto_them() {
    let path = legacy("import");
    let schema = read_sqlite(&path).unwrap();
    let tables: Vec<String> = ["tblCustomers", "tblInvoice", "OrderLines"].iter().map(|t| t.to_string()).collect();
    let result = import(&schema, &tables, "erp", "erp", &[]);
    assert_eq!(result.text, IMPORTED);
    assert_eq!(
        result.skipped,
        [("OrderLines".to_string(), "its key is not one integer column, which is not supported yet".to_string())]
    );

    // The module checks clean in a project with an external source erp.
    let project = load_in(
        [("erp.clarity", result.text.as_str())],
        std::iter::empty(),
        &[SourceRef::sql("main"), SourceRef::sql("erp").external()],
    );
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    assert!(drift(&project, "erp", &schema).is_empty(), "{:#?}", drift(&project, "erp", &schema));
    std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
}

#[test]
fn names_already_in_the_project_are_not_taken_again() {
    let path = legacy("names");
    let schema = read_sqlite(&path).unwrap();
    let result = import(&schema, &["tblInvoice".to_string()], "erp", "erp", &["Invoice".to_string()]);
    assert!(result.text.contains("\nentity InvoiceErp\n"), "{}", result.text);
    // A reference to a table that is not imported stays an integer.
    assert!(result.text.contains("    cust_id: integer, column \"CustID\"\n"), "{}", result.text);
    std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
}

#[test]
fn reports_where_the_dictionary_no_longer_fits_the_database() {
    let path = legacy("drift");
    let schema = read_sqlite(&path).unwrap();
    let tables = vec!["tblCustomers".to_string(), "tblInvoice".to_string()];
    let text = import(&schema, &tables, "erp", "erp", &[])
        .text
        .replace("column \"CustName\"", "column \"Name\"")
        .replace("since: date", "since: bool")
        .replace("table \"tblInvoice\"", "table \"Invoices\"");
    let project = load_in(
        [("erp.clarity", text.as_str())],
        std::iter::empty(),
        &[SourceRef::sql("main"), SourceRef::sql("erp").external()],
    );
    let found: Vec<(Severity, String)> = drift(&project, "erp", &schema).into_iter().map(|d| (d.severity, d.message)).collect();
    assert!(found.contains(&(Severity::Error, "Customers.cust_name: table 'tblCustomers' has no column 'Name'".to_string())), "{found:#?}");
    assert!(
        found.contains(&(Severity::Warning, "Customers.since: column 'Since' holds DATE, which does not fit bool".to_string())),
        "{found:#?}"
    );
    assert!(found.contains(&(Severity::Error, "Invoice: data source erp has no table 'Invoices'".to_string())), "{found:#?}");
    // A required column the dictionary does not know makes new records fail.
    assert!(
        found.contains(&(
            Severity::Warning,
            "Customers: column 'CustName' of 'tblCustomers' needs a value, but no field is stored in it; new records cannot be saved".to_string()
        )),
        "{found:#?}"
    );
    std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
}
