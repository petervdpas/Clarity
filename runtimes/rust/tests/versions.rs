//! A save never overwrites someone else's change (conformance/versions.json):
//! the version read with every record, the checked save and delete, a save
//! anyway, reload, undo by the version, and a file source changed on disk.

use std::collections::HashMap;

use clarity_runtime::{Db, Error, Id, Record, Result, SqlValue, sql};
use serde_json::Value;

#[derive(Debug, Clone, Default, PartialEq)]
struct Item {
    id: Id,
    name: String,
    version: i64,
}

impl Record for Item {
    const TABLE: &'static str = "item";
    const COLUMNS: &'static [&'static str] = &["name"];
    const VERSION: Option<&'static str> = Some("row_version");
    fn id(&self) -> Id {
        self.id
    }
    fn set_id(&mut self, id: Id) {
        self.id = id;
    }
    fn version(&self) -> i64 {
        self.version
    }
    fn set_version(&mut self, version: i64) {
        self.version = version;
    }
    fn to_values(&self) -> Vec<SqlValue> {
        vec![sql::text(&self.name)]
    }
    fn from_values(id: Id, v: &[SqlValue]) -> Result<Self> {
        Ok(Item { id, name: sql::get_text("name", &v[0])?, version: 0 })
    }
}

fn file() -> Value {
    serde_json::from_str(include_str!("../../../conformance/versions.json")).unwrap()
}

/// Whether `result` is what the step expects: ok, refused with `refusal`'s words, or not found.
fn expect<T>(name: &str, step: &Value, result: Result<T>, refusal: &Value) {
    let expected = step["expect"].as_str().unwrap_or("ok");
    match (expected, result) {
        ("ok", Ok(_)) => {}
        ("refused", Err(Error::Refused(text))) => {
            assert_eq!(text.key.as_deref(), refusal["key"].as_str(), "{name}: {step}");
            assert_eq!(text.text, refusal["english"].as_str().unwrap(), "{name}: {step}");
        }
        ("notFound", Err(Error::NotFound { .. })) => {}
        (expected, Ok(_)) => panic!("{name}: {step} went, but {expected} was expected"),
        (expected, Err(err)) => panic!("{name}: {step} failed with {err}, but {expected} was expected"),
    }
}

#[test]
fn versions_conformance() {
    let file = file();
    for case in file["cases"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let db = Db::memory().unwrap();
        db.batch("", file["table"].as_str().unwrap()).unwrap();
        let mut copies: HashMap<String, Item> = HashMap::new();
        let mut last_step = 0;
        for step in case["steps"].as_array().unwrap() {
            let copy = step["copy"].as_str().unwrap_or_default().to_string();
            match step["do"].as_str().unwrap() {
                "insert" => {
                    let mut item = Item { name: step["name"].as_str().unwrap().into(), ..Item::default() };
                    expect(name, step, db.save(&mut item), &file["refusal"]);
                    copies.insert(copy, item);
                }
                "read" => {
                    let of = copies[step["of"].as_str().unwrap()].id;
                    copies.insert(copy, db.get::<Item>(of).unwrap());
                }
                "set" => copies.get_mut(&copy).unwrap().name = step["name"].as_str().unwrap().into(),
                "save" => expect(name, step, db.save(copies.get_mut(&copy).unwrap()), &file["refusal"]),
                "saveAnyway" => expect(name, step, db.save_anyway(copies.get_mut(&copy).unwrap()), &file["refusal"]),
                "delete" => expect(name, step, db.delete(&copies[&copy]), &file["refusal"]),
                "reload" => expect(name, step, db.reload(copies.get_mut(&copy).unwrap()), &file["refusal"]),
                "version" => assert_eq!(copies[&copy].version, step["is"].as_i64().unwrap(), "{name}: {step}"),
                "stored" => {
                    let rows = db.rows("", "SELECT name, row_version FROM item WHERE id = ?1", &[SqlValue::Integer(copies[&copy].id)]).unwrap();
                    if step["gone"].as_bool() == Some(true) {
                        assert!(rows.is_empty(), "{name}: {step}");
                    } else {
                        let expected = vec![sql::text(step["name"].as_str().unwrap()), SqlValue::Integer(step["version"].as_i64().unwrap())];
                        assert_eq!(rows, vec![expected], "{name}: {step}");
                    }
                }
                "stepSave" => {
                    let item = copies.get_mut(&copy).unwrap();
                    expect(name, step, db.step(|| db.save(item)), &file["refusal"]);
                    last_step = db.last_step();
                }
                "undo" => expect(name, step, db.undo(last_step), &file["undoRefusal"]),
                "redo" => expect(name, step, db.redo(last_step), &file["undoRefusal"]),
                other => panic!("{name}: no step {other}"),
            }
        }
    }
}

#[cfg(all(feature = "formats", feature = "sqlite"))]
mod file_source {
    use super::*;
    use clarity_runtime::FileSource;

    #[derive(Debug, Clone, Default, PartialEq)]
    struct Line {
        id: Id,
        name: String,
    }

    impl Record for Line {
        const SOURCE: &'static str = "lines";
        const TABLE: &'static str = "line";
        const COLUMNS: &'static [&'static str] = &["name"];
        fn id(&self) -> Id {
            self.id
        }
        fn set_id(&mut self, id: Id) {
            self.id = id;
        }
        fn to_values(&self) -> Vec<SqlValue> {
            vec![sql::text(&self.name)]
        }
        fn from_values(id: Id, v: &[SqlValue]) -> Result<Self> {
            Ok(Line { id, name: sql::get_text("name", &v[0])? })
        }
    }

    #[test]
    fn a_file_changed_on_disk_refuses_the_save() {
        let file = file();
        let case = &file["file"];
        let folder = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("versions_file");
        std::fs::create_dir_all(&folder).unwrap();
        let path = folder.join("lines.csv");
        std::fs::write(&path, case["text"].as_str().unwrap()).unwrap();
        let fields = case["fields"].to_string();
        let mut db = Db::connect(&[]).unwrap();
        db.add_file(FileSource { name: "lines", format: case["format"].as_str().unwrap(), path: path.to_str().unwrap(), options: "{}", table: "line", key: "id", fields: &fields, readonly: false }).unwrap();
        let mut line = db.all::<Line>().unwrap().remove(0);
        for step in case["steps"].as_array().unwrap() {
            match step["do"].as_str().unwrap() {
                "save" => {
                    line.name = step["name"].as_str().unwrap().into();
                    expect("file", step, db.save(&mut line), &file["refusal"]);
                }
                "rewrite" => {
                    let bytes = std::fs::read(&path).unwrap();
                    // A later modification time, as a program writing the same bytes gives.
                    std::thread::sleep(std::time::Duration::from_millis(20));
                    std::fs::write(&path, bytes).unwrap();
                }
                "write" => std::fs::write(&path, step["text"].as_str().unwrap()).unwrap(),
                "contents" => assert_eq!(std::fs::read_to_string(&path).unwrap(), step["text"].as_str().unwrap()),
                other => panic!("file: no step {other}"),
            }
        }
    }
}

/// An item whose entity words the refusal itself, about the record as stored
/// now and what the database holds (as `{current.changedBy.name}` needs).
#[derive(Debug, Clone, Default, PartialEq)]
struct Worded {
    id: Id,
    name: String,
    version: i64,
}

impl Record for Worded {
    const TABLE: &'static str = "item";
    const COLUMNS: &'static [&'static str] = &["name"];
    const VERSION: Option<&'static str> = Some("row_version");
    fn id(&self) -> Id {
        self.id
    }
    fn set_id(&mut self, id: Id) {
        self.id = id;
    }
    fn version(&self) -> i64 {
        self.version
    }
    fn set_version(&mut self, version: i64) {
        self.version = version;
    }
    fn to_values(&self) -> Vec<SqlValue> {
        vec![sql::text(&self.name)]
    }
    fn from_values(id: Id, v: &[SqlValue]) -> Result<Self> {
        Ok(Worded { id, name: sql::get_text("name", &v[0])?, version: 0 })
    }
    fn saved_since(db: &Db, current: &Self) -> clarity_runtime::ui::Text {
        let count = db.all::<Worded>().map(|all| all.len()).unwrap_or(0);
        clarity_runtime::ui::Text::plain(format!("{} saved it; {count} records", current.name))
    }
}

#[test]
fn an_entity_words_the_refusal_with_the_database_at_hand() {
    let db = Db::memory().unwrap();
    db.batch("", file()["table"].as_str().unwrap()).unwrap();
    let mut first = Worded { name: "first".into(), ..Worded::default() };
    db.save(&mut first).unwrap();
    let mut second = db.get::<Worded>(first.id).unwrap();
    first.name = "Anna".into();
    db.save(&mut first).unwrap();
    for refused in [db.save(&mut second), db.delete(&second)] {
        match refused {
            Err(Error::Refused(text)) => assert_eq!(text.text, "Anna saved it; 1 records"),
            other => panic!("not refused in the entity's words: {other:?}"),
        }
    }
}
