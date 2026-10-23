//! Lists of values as fields (U1c), kept in one column as JSON, against
//! the cases every runtime shares (conformance/list-fields.json).

use clarity_runtime::sql::{self, SqlValue};
use clarity_runtime::{Decimal, NaiveDate};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

const CASES: &str = include_str!("../../../conformance/list-fields.json");

fn cases(part: &str) -> Vec<Value> {
    let all: Value = serde_json::from_str(CASES).expect("list-fields.json is JSON");
    all[part].as_array().unwrap().clone()
}

fn column(case: &Value) -> SqlValue {
    case["column"].as_str().map_or(SqlValue::Null, |s| SqlValue::Text(s.to_string()))
}

/// Reads the column as the item type.
fn read<T: Serialize + DeserializeOwned>(case: &Value) {
    let items: Vec<T> = sql::get_list("tags", &column(case)).unwrap();
    assert_eq!(serde_json::to_value(&items).unwrap(), case["items"], "{case}");
}

fn kept<T: Serialize + DeserializeOwned>(case: &Value) {
    let typed: Vec<T> = serde_json::from_value(case["items"].clone()).unwrap();
    assert_eq!(sql::list(&typed), SqlValue::Text(case["column"].as_str().unwrap().to_string()), "{case}");
}

fn each(case: &Value, run: [fn(&Value); 4]) {
    match case["item"].as_str().unwrap() {
        "text" | "enum" => run[0](case),
        "integer" => run[1](case),
        "decimal" => run[2](case),
        "date" => run[3](case),
        other => panic!("no item {other}"),
    }
}

#[test]
fn a_list_is_kept_as_compact_json() {
    for case in cases("kept") {
        each(&case, [kept::<String>, kept::<i64>, kept::<Decimal>, kept::<Option<NaiveDate>>]);
    }
}

#[test]
fn a_column_gives_its_items() {
    for case in cases("read") {
        each(&case, [read::<String>, read::<i64>, read::<Decimal>, read::<Option<NaiveDate>>]);
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
struct Album {
    id: clarity_runtime::Id,
    tags: Vec<String>,
    tracks: Vec<i64>,
}

impl clarity_runtime::Record for Album {
    const TABLE: &'static str = "album";
    const COLUMNS: &'static [&'static str] = &["tags", "tracks"];

    fn id(&self) -> clarity_runtime::Id {
        self.id
    }

    fn set_id(&mut self, id: clarity_runtime::Id) {
        self.id = id;
    }

    fn to_values(&self) -> Vec<SqlValue> {
        vec![sql::list(&self.tags), sql::list(&self.tracks)]
    }

    fn from_values(id: clarity_runtime::Id, v: &[SqlValue]) -> clarity_runtime::Result<Self> {
        Ok(Album { id, tags: sql::get_list("tags", &v[0])?, tracks: sql::get_list("tracks", &v[1])? })
    }
}

#[test]
fn a_query_finds_the_records_whose_list_holds_an_item() {
    use clarity_runtime::{Cond, Db, Query};
    let mut db = Db::memory().unwrap();
    db.migrate(&[&["CREATE TABLE album (id INTEGER PRIMARY KEY, tags TEXT NOT NULL DEFAULT '[]', tracks TEXT NOT NULL DEFAULT '[]')"]]).unwrap();
    for (tags, tracks) in [(vec!["jazz", "live"], vec![1, 5]), (vec!["live music"], vec![15]), (vec![], vec![])] {
        let mut album = Album { tags: tags.into_iter().map(String::from).collect(), tracks, ..Album::default() };
        db.save(&mut album).unwrap();
    }
    let live: Vec<Album> = db.find(&Query::new().filter(Cond::holds("tags", SqlValue::Text("live".into())))).unwrap();
    assert_eq!(live.len(), 1, "an item, not a word in one");
    let five: Vec<Album> = db.find(&Query::new().filter(Cond::holds("tracks", SqlValue::Integer(5)))).unwrap();
    assert_eq!(five.len(), 1);
    let none: Vec<Album> = db.find(&Query::new().filter(Cond::holds("tags", SqlValue::Text("rock".into())).negated())).unwrap();
    assert_eq!(none.len(), 3);
}
