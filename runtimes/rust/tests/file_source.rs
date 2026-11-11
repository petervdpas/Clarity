//! A data source that is a file in a format (docs/FORMATS-PLAN.md): a CSV
//! read into a table its entity finds, saves and deletes in, each change
//! written back to the file with its own column order and the columns no
//! entity maps kept.
#![cfg(all(feature = "formats", feature = "sqlite"))]

use clarity_runtime::{Db, FileSource, Id, Record, Result, SqlValue, sql};

#[derive(Debug, Clone, Default, PartialEq)]
struct Login {
    id: Id,
    site: String,
    visits: i64,
    paid: bool,
}

impl Record for Login {
    const SOURCE: &'static str = "logins";
    const TABLE: &'static str = "login";
    const COLUMNS: &'static [&'static str] = &["Site", "visits", "paid"];
    fn id(&self) -> Id {
        self.id
    }
    fn set_id(&mut self, id: Id) {
        self.id = id;
    }
    fn to_values(&self) -> Vec<SqlValue> {
        vec![sql::text(&self.site), sql::integer(self.visits), SqlValue::Bool(self.paid)]
    }
    fn from_values(id: Id, values: &[SqlValue]) -> Result<Self> {
        Ok(Login { id, site: sql::get_text("site", &values[0])?, visits: sql::get_integer("visits", &values[1])?, paid: sql::get_boolean("paid", &values[2])? })
    }
}

const FIELDS: &str = r#"[{"name":"Site","kind":"text"},{"name":"visits","kind":"integer"},{"name":"paid","kind":"bool"}]"#;

fn source(path: &str) -> FileSource<'_> {
    FileSource { name: "logins", format: "csv", path, options: r#"{"separator":";"}"#, table: "login", key: "id", fields: FIELDS, readonly: false }
}

#[test]
fn a_csv_file_is_a_data_source_written_back_after_each_change() {
    let folder = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("file_source");
    std::fs::create_dir_all(&folder).unwrap();
    let file = folder.join("logins.csv");
    std::fs::write(&file, "Site;Note;visits;paid\r\nexample.com;keep me;3;true\r\nshop.nl;;1;false\r\n").unwrap();
    let mut db = Db::connect(&[]).unwrap();
    db.add_file(source(file.to_str().unwrap())).unwrap();

    let mut all: Vec<Login> = db.all().unwrap();
    assert_eq!(all, vec![Login { id: 1, site: "example.com".into(), visits: 3, paid: true }, Login { id: 2, site: "shop.nl".into(), visits: 1, paid: false }]);

    all[0].visits = 4;
    db.save(&mut all[0]).unwrap();
    let mut new = Login { site: "a;b".into(), visits: 0, paid: true, ..Login::default() };
    db.save(&mut new).unwrap();
    db.delete(&all[1]).unwrap();
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "Site;Note;visits;paid\r\nexample.com;keep me;4;true\r\n\"a;b\";;0;true\r\n");
    assert!(!folder.join("logins.csv.new").exists(), "the new file replaced the old one");

    // A column the entity maps that the file lacks stops the program as it opens.
    std::fs::write(&file, "Site;paid\r\n").unwrap();
    let mut db = Db::connect(&[]).unwrap();
    let error = db.add_file(source(file.to_str().unwrap())).unwrap_err().to_string();
    assert!(error.contains("data source logins: the CSV has no column 'visits'"), "{error}");
}

#[test]
fn a_file_not_there_yet_starts_empty_and_is_made_at_the_first_save() {
    let folder = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("file_source_new");
    let _ = std::fs::remove_dir_all(&folder);
    let file = folder.join("fresh").join("logins.csv");
    let mut db = Db::connect(&[]).unwrap();
    db.add_file(source(file.to_str().unwrap())).unwrap();
    assert!(db.all::<Login>().unwrap().is_empty());
    db.save(&mut Login { site: "new.nl".into(), visits: 0, paid: true, ..Login::default() }).unwrap();
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "Site;visits;paid\r\nnew.nl;0;true\r\n");
    // A readonly source needs its file.
    let mut db = Db::connect(&[]).unwrap();
    let missing = folder.join("missing.csv");
    let error = db.add_file(FileSource { readonly: true, ..source(missing.to_str().unwrap()) }).unwrap_err().to_string();
    assert!(error.contains("data source logins: cannot read"), "{error}");
}

#[derive(Debug, Clone, Default, PartialEq)]
struct Order {
    id: Id,
    number: String,
}

impl Record for Order {
    const SOURCE: &'static str = "feed";
    const TABLE: &'static str = "order";
    const COLUMNS: &'static [&'static str] = &["number"];
    fn id(&self) -> Id {
        self.id
    }
    fn set_id(&mut self, id: Id) {
        self.id = id;
    }
    fn to_values(&self) -> Vec<SqlValue> {
        vec![sql::text(&self.number)]
    }
    fn from_values(id: Id, values: &[SqlValue]) -> Result<Self> {
        Ok(Order { id, number: sql::get_text("number", &values[0])? })
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
struct Line {
    id: Id,
    order: Id,
    product: String,
}

impl Record for Line {
    const SOURCE: &'static str = "feed";
    const TABLE: &'static str = "line";
    const COLUMNS: &'static [&'static str] = &["order", "product"];
    fn id(&self) -> Id {
        self.id
    }
    fn set_id(&mut self, id: Id) {
        self.id = id;
    }
    fn to_values(&self) -> Vec<SqlValue> {
        vec![sql::integer(self.order), sql::text(&self.product)]
    }
    fn from_values(id: Id, values: &[SqlValue]) -> Result<Self> {
        Ok(Line { id, order: sql::get_integer("order", &values[0])?, product: sql::get_text("product", &values[1])? })
    }
}

const ORDERS_XML: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<shop>\n  <orders>\n    <order number=\"A-1\"><lines><line product=\"Tea\"/><line product=\"Coffee\"/></lines></order>\n    <order number=\"A-2\"><lines><line product=\"Milk\"/></lines></order>\n  </orders>\n</shop>\n";
const ORDERS_OPTIONS: &str = r#"{"at":"shop/orders/order","fields":{"number":"@number"},"nested":{"lines":{"at":"lines/line","fields":{"product":"@product"},"parent":"order"}}}"#;
const ORDERS_FIELDS: &str = r#"[{"name":"number","kind":"text"},{"name":"lines","kind":"records","entity":"Line","fields":[{"name":"product","kind":"text"}],"base":{},"table":"line","key":"id"}]"#;

#[test]
fn a_file_source_holds_the_records_inside_its_records_in_their_own_table() {
    let folder = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("file_source_nested");
    std::fs::create_dir_all(&folder).unwrap();
    let file = folder.join("orders.xml");
    std::fs::write(&file, ORDERS_XML).unwrap();
    let mut db = Db::connect(&[]).unwrap();
    let path = file.to_str().unwrap();
    db.add_file(FileSource { name: "feed", format: "xml", path, options: ORDERS_OPTIONS, table: "order", key: "id", fields: ORDERS_FIELDS, readonly: false }).unwrap();

    let orders: Vec<Order> = db.all().unwrap();
    assert_eq!(orders, vec![Order { id: 1, number: "A-1".into() }, Order { id: 2, number: "A-2".into() }]);
    let lines: Vec<Line> = db.all().unwrap();
    assert_eq!(
        lines,
        vec![Line { id: 1, order: 1, product: "Tea".into() }, Line { id: 2, order: 1, product: "Coffee".into() }, Line { id: 3, order: 2, product: "Milk".into() }]
    );

    db.save(&mut Line { order: 2, product: "Juice".into(), ..Line::default() }).unwrap();
    db.delete(&lines[0]).unwrap();
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<shop>\n  <orders>\n    <order number=\"A-1\"><lines><line product=\"Coffee\"/></lines></order>\n    <order number=\"A-2\"><lines><line product=\"Milk\"/><line product=\"Juice\"/></lines></order>\n  </orders>\n</shop>\n"
    );
}
