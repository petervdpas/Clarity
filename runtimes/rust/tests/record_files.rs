//! Files in records (U1c), against the cases every runtime shares
//! (conformance/record-files.json), and kept in SQLite or in a file store.

use clarity_runtime::picture::{Picture, PictureColumn};
use clarity_runtime::record_file::{self, RecordFile};
use clarity_runtime::{Db, Id, Record, SqlValue, sql};
use serde_json::{Value, json};

fn cases() -> Value {
    serde_json::from_str(include_str!("../../../conformance/record-files.json")).unwrap()
}

#[test]
fn a_file_in_a_store_is_kept_under_its_name_in_a_folder_of_its_contents() {
    for c in cases()["storePaths"].as_array().unwrap() {
        assert_eq!(record_file::store_path(c["text"].as_str().unwrap().as_bytes(), c["chosen"].as_str().unwrap()), c["path"].as_str().unwrap(), "{c}");
    }
    for c in cases()["names"].as_array().unwrap() {
        assert_eq!(record_file::name_of(c["path"].as_str().unwrap()), c["name"].as_str().unwrap());
    }
}

#[test]
fn a_copy_goes_to_the_downloads_folder_under_a_free_name() {
    for c in cases()["downloads"].as_array().unwrap() {
        let env = c["env"].as_object().unwrap().clone();
        let found = record_file::downloads_in(c["os"].as_str().unwrap(), |name| env.get(name).and_then(Value::as_str).map(str::to_string));
        assert_eq!(found.as_deref(), c["folder"].as_str(), "{c}");
    }
    for c in cases()["copies"].as_array().unwrap() {
        let taken: Vec<&str> = c["taken"].as_array().unwrap().iter().map(|t| t.as_str().unwrap()).collect();
        assert_eq!(record_file::free_name(c["name"].as_str().unwrap(), |n| taken.contains(&n)), c["saved"].as_str().unwrap(), "{c}");
    }
}

#[test]
fn a_file_travels_with_its_name() {
    let stored = RecordFile { name: "manual.pdf".into(), content: Picture::Stored };
    assert_eq!(serde_json::to_value(&stored).unwrap(), json!({"name": "manual.pdf", "stored": true}));
    let new: RecordFile = serde_json::from_value(json!({"name": "C:\\docs\\a.pdf", "data": "JVBERg=="})).unwrap();
    assert_eq!(new, RecordFile { name: "a.pdf".into(), content: Picture::New(b"%PDF".to_vec()) });
    let in_store: RecordFile = serde_json::from_value(json!({"name": "b.pdf", "file": "ab/b.pdf"})).unwrap();
    assert_eq!(in_store.content, Picture::InStore("ab/b.pdf".into()));
}

#[derive(Debug, Clone, Default, PartialEq)]
struct Product {
    id: Id,
    manual: Option<RecordFile>,
    sheet: Option<RecordFile>,
}

impl Record for Product {
    const TABLE: &'static str = "product";
    const COLUMNS: &'static [&'static str] = &["manual", "manual_name", "sheet", "sheet_name"];
    const PICTURES: &'static [PictureColumn] =
        &[PictureColumn { column: "manual", store: None, name: Some("manual_name") }, PictureColumn { column: "sheet", store: Some("documents"), name: Some("sheet_name") }];

    fn id(&self) -> Id {
        self.id
    }

    fn set_id(&mut self, id: Id) {
        self.id = id;
    }

    fn to_values(&self) -> Vec<SqlValue> {
        vec![sql::file_content(&self.manual), sql::file_name(&self.manual), sql::file_content(&self.sheet), sql::file_name(&self.sheet)]
    }

    fn from_values(id: Id, v: &[SqlValue]) -> clarity_runtime::Result<Self> {
        Ok(Product { id, manual: sql::get_file("manual", &v[0], &v[1])?, sheet: sql::get_file("sheet", &v[2], &v[3])? })
    }
}

#[test]
fn a_file_is_kept_with_its_name_in_the_database_or_its_store() {
    let store = std::env::temp_dir().join(format!("clarity-record-files-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&store);
    std::fs::create_dir_all(&store).unwrap();
    let mut db = Db::memory().unwrap();
    db.migrate(&[&["CREATE TABLE product (id INTEGER PRIMARY KEY, manual BLOB, manual_name TEXT, sheet TEXT, sheet_name TEXT)"]]).unwrap();
    db.add_store("documents", &store.display().to_string()).unwrap();

    let pdf = b"%PDF-1.7 not a picture".to_vec();
    let mut product = Product {
        manual: Some(RecordFile { name: "manual.pdf".into(), content: Picture::New(pdf.clone()) }),
        sheet: Some(RecordFile { name: "sheet.pdf".into(), content: Picture::New(pdf.clone()) }),
        ..Product::default()
    };
    db.save(&mut product).unwrap();
    let read: Product = db.get(product.id).unwrap();
    assert_eq!(read.manual, Some(RecordFile { name: "manual.pdf".into(), content: Picture::Stored }));
    assert_eq!(db.picture::<Product>(product.id, "manual").unwrap(), Some(pdf.clone()));
    // In the store, under its name in a folder of its contents.
    let path = record_file::store_path(&pdf, "sheet.pdf");
    assert_eq!(std::fs::read(store.join(&path)).unwrap(), pdf);
    assert_eq!(db.picture::<Product>(product.id, "sheet").unwrap(), Some(pdf.clone()));
    assert_eq!(read.sheet.as_ref().map(|f| f.name.as_str()), Some("sheet.pdf"));

    // Saved again, the file stays; emptied, it is gone.
    let mut again = read.clone();
    db.save(&mut again).unwrap();
    assert_eq!(db.picture::<Product>(product.id, "manual").unwrap(), Some(pdf));
    again.manual = None;
    db.save(&mut again).unwrap();
    assert_eq!(db.get::<Product>(product.id).unwrap().manual, None);

    // Copied to another record (`product.sheet = other.manual`): with its name, the bytes read when stored.
    let read: Product = db.get(product.id).unwrap();
    let copied = db.copied_file::<Product>(&read.sheet, read.id, "sheet", Some("documents"), None).unwrap();
    assert_eq!(copied, Some(RecordFile { name: "sheet.pdf".into(), content: Picture::New(b"%PDF-1.7 not a picture".to_vec()) }));
    assert_eq!(db.copied_file::<Product>(&read.manual, read.id, "manual", None, None).unwrap(), None);
    let mut other = Product { manual: copied, ..Product::default() };
    db.save(&mut other).unwrap();
    assert_eq!(db.picture::<Product>(other.id, "manual").unwrap(), Some(b"%PDF-1.7 not a picture".to_vec()));
    let _ = std::fs::remove_dir_all(&store);
}
