//! A project with an external SQLite database: loading it checks the
//! dictionary against the database's tables.

use clarity_project::load_modules;
use clarity_spec::Severity;

const PROJECT: &str = "project Shop\n    stack rust desktop\n    datasource main sqlite \"data/shop.db\"\n    datasource erp sqlite \"legacy/erp.db\" external\n";

const ERP: &str = "module erp\nsource erp\n\nentity Invoice\n    table \"tblInvoice\"\n    key \"InvoiceID\"\n    number: text(20), column \"InvNo\", label @invoiceNumberLabel\n";

/// The default language, with the keys ERP uses.
const EN: &str = "translation en\n    invoiceNumberLabel \"Number\"\n    yes \"Yes\"\n    no \"No\"\n    ok \"OK\"\n    cancel \"Cancel\"\n    dismiss \"Dismiss\"\n    undoChangedSince \"This was changed since, so it cannot be taken back\"\n    undoTooOld \"This was done too long ago to take back\"\n    savedSince \"Someone else changed this record since you opened it. Reload it to see their change.\"\n";

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

    let tables = clarity_project::source_tables(&dir, "erp", None).unwrap();
    let listed: Vec<(&str, Option<&str>)> = tables.iter().map(|t| (t.name.as_str(), t.not_importable.as_deref())).collect();
    assert_eq!(
        listed,
        [("Lines", Some("its key is not one integer column, which is not supported yet")), ("tblInvoice", None)]
    );

    let done = clarity_project::import_tables(&dir, "erp", "erp", &["tblInvoice".to_string()], None).unwrap();
    assert_eq!(done.path, dir.join("erp.clarity"));
    let project = load_modules(&dir).unwrap();
    // Its saves are not checked until its version column is named, which is said.
    let messages: Vec<&str> = project.diagnostics.iter().map(|d| d.message.as_str()).collect();
    assert_eq!(messages, ["Invoice has no version column, so saves of it are not checked: name its column with version \"Column\", or write version none"]);
    assert_eq!(project.source_of("Invoice"), Some("erp"));

    // The module is not written over.
    let again = clarity_project::import_tables(&dir, "erp", "erp", &["tblInvoice".to_string()], None).unwrap_err();
    assert!(again.contains("there is already a module file"), "{again}");
    // Only external SQL databases are imported.
    let main = clarity_project::source_tables(&dir, "main", None).unwrap_err();
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

    clarity_project::import_tables(&dir, "erp", "erp", &["tblInvoice".to_string()], None).unwrap();
    let done = clarity_project::take_ownership(&dir, "erp").unwrap();
    assert_eq!(done.step, dir.join("migrations/erp/0001.clarity"));
    let step = std::fs::read_to_string(&done.step).unwrap();
    assert!(step.contains("migration \"Taken over\" baseline\n    add entity Invoice\n        table \"tblInvoice\"\n        key \"InvoiceID\"\n"), "{step}");

    let settings = clarity_project::load(&root.join("shop.clarity")).settings.unwrap();
    assert!(!settings.datasources[1].external);
    // The project is in step with its database; the next step gives the table its version column.
    let project = load_modules(&dir).unwrap();
    let messages: Vec<&str> = project.diagnostics.iter().map(|d| d.message.as_str()).collect();
    assert_eq!(messages, ["Invoice has no version column yet (data source erp), so saves of it are not checked: write the next migration step, which adds it (add version Invoice)"]);

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
    // The project's own database goes to the person's data folder; an external one stays where it is.
    assert_eq!("data:shop/shop.db", built.datasources[0].url);
    assert_eq!(std::path::Path::new(&built.datasources[1].url), dir.join("legacy/erp.db"));
}

/// A data source that is a file (docs/FORMATS-PLAN.md) imports as one
/// entity, a text field per column; types are not guessed from the values.
/// Beside it Import writes a mapping with the paths that differ from the
/// fields' names and the source's options, and lays the source over it.
#[test]
fn a_csv_source_imports_as_one_entity_of_text_fields() {
    let root = std::env::temp_dir().join(format!("clarity-import-csv-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let dir = root.join("shop");
    std::fs::create_dir_all(dir.join("imports")).unwrap();
    std::fs::write(root.join("shop.clarity"), "project Shop\n    stack rust desktop\n    datasource main sqlite \"data/shop.db\"\n    datasource logins csv \"imports/logins.csv\" external\n        separator \";\"\n").unwrap();
    std::fs::write(dir.join("imports/logins.csv"), "Login URL;visits\r\nexample.com;3\r\n").unwrap();

    let tables = clarity_project::source_tables(&dir, "logins", None).unwrap();
    assert_eq!(tables.iter().map(|t| (t.name.as_str(), t.not_importable.is_none())).collect::<Vec<_>>(), [("logins", true)]);
    let done = clarity_project::import_tables(&dir, "logins", "logins", &["logins".to_string()], None).unwrap();
    assert!(done.text.contains("source logins"), "{}", done.text);
    assert!(done.text.ends_with("\nmapping LoginsFeed of Logins from csv\n    loginUrl at \"Login URL\"\n    separator \";\"\n"), "{}", done.text);
    assert!(!done.text.contains("column"), "the mapping says where the fields are: {}", done.text);
    assert!(!done.text.contains("integer"), "every field is text: {}", done.text);
    let settings = std::fs::read_to_string(root.join("shop.clarity")).unwrap();
    assert!(settings.ends_with("    datasource logins file \"imports/logins.csv\" external using LoginsFeed\n"), "{settings}");
    assert!(done.project_file.is_some());
    let project = load_modules(&dir).unwrap();
    assert!(project.diagnostics.iter().all(|d| d.severity != Severity::Error), "{:#?}", project.diagnostics);

    // The mapping page's preview reads the file through the mapping, as the entity's fields hold it.
    let preview = clarity_project::mappings::preview_mapping(&dir, "LoginsFeed", std::path::Path::new("imports/logins.csv"), 10).unwrap();
    assert_eq!(preview.problem, None);
    assert_eq!(preview.tables.len(), 1);
    assert_eq!(preview.tables[0].columns, ["loginUrl", "visits"]);
    assert_eq!(preview.tables[0].rows, [["example.com", "3"]]);
    let listed = clarity_project::mappings::mappings(&dir).unwrap();
    assert_eq!(listed.iter().map(|m| (m.form.name.as_str(), m.form.entity.as_str())).collect::<Vec<_>>(), [("LoginsFeed", "Logins")]);
    assert!(listed[0].path.ends_with("logins.clarity"), "{}", listed[0].path);
    std::fs::remove_dir_all(&root).unwrap();
}

const ORDERS_XML: &str = r#"<?xml version="1.0"?>
<shop><orders>
  <order number="A-1" note="gift"><placed>2026-11-01</placed><note>leave at door</note><lines><line product="Coffee" quantity="2"/></lines></order>
  <order number="A-2"><placed>2026-11-02</placed></order>
</orders></shop>"#;

/// A project with one XML file source, `at` given or not.
fn xml_project(name: &str, at: Option<&str>) -> (std::path::PathBuf, std::path::PathBuf) {
    let root = std::env::temp_dir().join(format!("clarity-import-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let dir = root.join("shop");
    std::fs::create_dir_all(dir.join("imports")).unwrap();
    let at = at.map(|a| format!("        at \"{a}\"\n")).unwrap_or_default();
    std::fs::write(root.join("shop.clarity"), format!("project Shop\n    stack rust desktop\n    datasource main sqlite \"data/shop.db\"\n    datasource feed xml \"imports/orders.xml\" external\n{at}")).unwrap();
    std::fs::write(dir.join("imports/orders.xml"), ORDERS_XML).unwrap();
    (root, dir)
}

/// An XML file source lists its records by path; importing one writes a
/// text field per attribute and child element, and a mapping with the `at`
/// and the paths (`@note` where the element `note` would be found first).
#[test]
fn an_xml_source_imports_the_records_at_a_path() {
    let (root, dir) = xml_project("xml", None);
    let tables = clarity_project::source_tables(&dir, "feed", None).unwrap();
    let names: Vec<&str> = tables.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(names, ["shop/orders/order", "shop/orders/order/lines/line"]);
    let done = clarity_project::import_tables(&dir, "feed", "feed", &["shop/orders/order".to_string()], None).unwrap();
    assert!(done.text.contains("entity Order\n"), "{}", done.text);
    assert!(done.text.contains("number: text,"), "{}", done.text);
    assert!(done.text.contains("note2: text,"), "{}", done.text);
    assert!(done.text.ends_with("\nmapping OrderFeed of Order from xml\n    at \"shop/orders/order\"\n    note at \"@note\"\n    note2 at \"note\"\n"), "{}", done.text);
    let settings = std::fs::read_to_string(root.join("shop.clarity")).unwrap();
    assert!(settings.ends_with("    datasource feed file \"imports/orders.xml\" external using OrderFeed\n"), "{settings}");
    assert!(done.project_file.is_some());
    let project = load_modules(&dir).unwrap();
    assert!(project.diagnostics.iter().all(|d| d.severity != Severity::Error), "{:#?}", project.diagnostics);

    let preview = clarity_project::mappings::preview_mapping(&dir, "OrderFeed", std::path::Path::new("imports/orders.xml"), 1).unwrap();
    assert_eq!(preview.problem, None);
    let orders = &preview.tables[0];
    assert_eq!((orders.columns.join(","), orders.count), ("number,note,placed,note2".to_string(), 2));
    assert_eq!(orders.rows, [["A-1", "gift", "2026-11-01", "leave at door"]]);

    // Now the source reads orders, at the mapping's `at`: the lines inside them stay in the file.
    let tables = clarity_project::source_tables(&dir, "feed", None).unwrap();
    assert!(tables[1].not_importable.is_some(), "{tables:?}");
    let other = clarity_project::import_tables(&dir, "feed", "lines", &["shop/orders/order/lines/line".to_string()], None).unwrap_err();
    assert_eq!(other, "data source feed reads 'shop/orders/order' (its at), not 'shop/orders/order/lines/line'; import 'shop/orders/order', or change its at first");
    std::fs::remove_dir_all(&root).unwrap();
}

/// Import from an XSD writes the source's one entity with its real types:
/// enums, sizes, `required`, a list of values. Records inside it stay in
/// the file, named as skipped; an enumeration of values that are no names
/// is text with its values in a comment.
#[test]
fn an_xsd_imports_typed_entities() {
    let (root, dir) = xml_project("xsd", Some("shop/orders/order"));
    std::fs::write(
        dir.join("imports/orders.xsd"),
        r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
  <xs:element name="shop"><xs:complexType><xs:sequence>
    <xs:element name="orders"><xs:complexType><xs:sequence>
      <xs:element name="order" type="OrderType" maxOccurs="unbounded"/>
    </xs:sequence></xs:complexType></xs:element>
  </xs:sequence></xs:complexType></xs:element>
  <xs:simpleType name="Status"><xs:restriction base="xs:string">
    <xs:enumeration value="open"/><xs:enumeration value="in-transit"/>
  </xs:restriction></xs:simpleType>
  <xs:complexType name="OrderType"><xs:sequence>
    <xs:element name="placed" type="xs:date"/>
    <xs:element name="total" type="xs:decimal" minOccurs="0"/>
    <xs:element name="note" minOccurs="0"><xs:simpleType><xs:restriction base="xs:string"><xs:maxLength value="200"/></xs:restriction></xs:simpleType></xs:element>
    <xs:element name="tag" type="xs:string" minOccurs="0" maxOccurs="unbounded"/>
    <xs:element name="customer"><xs:complexType><xs:sequence><xs:element name="name" type="xs:string"/></xs:sequence></xs:complexType></xs:element>
    <xs:element name="lines" minOccurs="0"><xs:complexType><xs:sequence>
      <xs:element name="line" maxOccurs="unbounded"><xs:complexType>
        <xs:attribute name="product" type="xs:string" use="required"/>
        <xs:attribute name="quantity" type="xs:int"/>
      </xs:complexType></xs:element>
    </xs:sequence></xs:complexType></xs:element>
  </xs:sequence>
  <xs:attribute name="number" use="required"><xs:simpleType><xs:restriction base="xs:string"><xs:maxLength value="10"/></xs:restriction></xs:simpleType></xs:attribute>
  <xs:attribute name="status" type="Status"/>
  <xs:attribute name="paid" type="xs:boolean"/>
  <xs:attribute name="kind"><xs:simpleType><xs:restriction base="xs:string"><xs:enumeration value="retail"/><xs:enumeration value="wholesale"/></xs:restriction></xs:simpleType></xs:attribute>
  </xs:complexType>
</xs:schema>"#,
    )
    .unwrap();
    let schema = std::path::Path::new("imports/orders.xsd");
    let tables = clarity_project::source_tables(&dir, "feed", Some(schema)).unwrap();
    let names: Vec<(&str, bool)> = tables.iter().map(|t| (t.name.as_str(), t.not_importable.is_none())).collect();
    assert_eq!(names, [("shop", false), ("shop/orders/order", true), ("shop/orders/order/customer", false), ("shop/orders/order/lines/line", false)]);
    let chosen: Vec<String> = ["shop/orders/order", "shop/orders/order/customer", "shop/orders/order/lines/line"].map(String::from).to_vec();
    let done = clarity_project::import_tables(&dir, "feed", "feed", &chosen, Some(schema)).unwrap();
    let text = &done.text;
    for wanted in [
        "entity Order\n",
        "number: text(10), required,",
        "status: text,",
        "// status takes one of \"open\", \"in-transit\".",
        "kind: OrderKind,",
        "paid: bool,",
        "placed: date, required,",
        "total: decimal(18, 6),",
        "note: text(200),",
        "tag: list<text>,",
        "enum OrderKind\n    Retail",
        "    Wholesale",
    ] {
        assert!(text.contains(wanted), "{wanted} in {text}");
    }
    for unwanted in ["customer", "lines", "entity Customer", "entity Line", "enum Status"] {
        assert!(!text.contains(unwanted), "{unwanted} in {text}");
    }
    let skipped: Vec<&str> = done.skipped.iter().map(|(_, why)| why.as_str()).collect();
    assert_eq!(
        skipped,
        [
            "its records are inside shop/orders/order, and stay in the file untouched",
            "its records are inside shop/orders/order, and stay in the file untouched",
            "field 'customer' holds records, which stay in the file untouched",
            "field 'lines' holds records, which stay in the file untouched",
        ]
    );
    assert!(text.ends_with("\nmapping OrderFeed of Order from xml\n    at \"shop/orders/order\"\n"), "the paths are the fields' names: {text}");
    assert!(done.project_file.is_some(), "the source is laid over the mapping");
    let project = load_modules(&dir).unwrap();
    assert!(project.diagnostics.iter().all(|d| d.severity != Severity::Error), "{:#?}", project.diagnostics);
    std::fs::remove_dir_all(&root).unwrap();
}
