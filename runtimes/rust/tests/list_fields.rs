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

#[test]
fn a_typed_item_is_added_once_or_refused() {
    use clarity_runtime::list_fields::{self, Added, ItemKind};
    for case in cases("added") {
        let kind: ItemKind = serde_json::from_value(case["item"].clone()).unwrap();
        let items = case["items"].as_array().unwrap();
        match list_fields::added(kind, &[], items, case["typed"].as_str().unwrap()) {
            Added::Refused => assert!(case.get("refused").is_some(), "{case}: refused"),
            Added::Items(gives) => assert_eq!(Value::Array(gives), case["gives"], "{case}"),
        }
    }
}

#[test]
fn a_typed_date_or_enum_value_must_be_one() {
    use clarity_runtime::list_fields::{self, Added, ItemKind};
    let genres = ["Jazz".to_string(), "Rock".to_string()];
    assert!(matches!(list_fields::added(ItemKind::Enum, &genres, &[], "Polka"), Added::Refused));
    assert!(matches!(list_fields::added(ItemKind::Enum, &genres, &[], " Rock "), Added::Items(i) if i == vec![Value::from("Rock")]));
    assert!(matches!(list_fields::added(ItemKind::Date, &[], &[], "2026-02-30"), Added::Refused));
    assert!(matches!(list_fields::added(ItemKind::Date, &[], &[], "2026-10-31"), Added::Items(i) if i == vec![Value::from("2026-10-31")]));
    assert!(matches!(list_fields::added(ItemKind::Decimal, &[], &[Value::from("1.50")], "1.5"), Added::Items(i) if i.len() == 1), "1.50 and 1.5 are one item");
}

fn drawn(state: clarity_runtime::ui::WindowState) -> clarity_runtime::Result<clarity_runtime::ui::Reply> {
    let data = state.data.clone();
    Ok(clarity_runtime::ui::Ui::from_state(&state).reply(&state.window, data))
}

#[test]
fn item_event_sets_the_list_or_says_refused() {
    use clarity_runtime::list_fields::{self, ItemKind};
    use clarity_runtime::ui::WindowState;
    let db = clarity_runtime::Db::memory().unwrap();
    let mut data = serde_json::Map::new();
    data.insert("album".into(), serde_json::json!({ "id": 1, "tracks": [1] }));
    let state = WindowState::new("Album", data);
    let reply = list_fields::event(&db, state.clone(), "tracks", "album.tracks", ItemKind::Integer, &[], " 12 ", None, drawn, drawn).unwrap();
    assert_eq!(reply.state.data["album"]["tracks"], serde_json::json!([1, 12]));
    assert!(!reply.refused);
    let reply = list_fields::event(&db, state.clone(), "tracks", "album.tracks", ItemKind::Integer, &[], "twelve", None, |_| panic!("refused"), drawn).unwrap();
    assert!(reply.refused);
    assert_eq!(reply.state.data, state.data, "the list stays");
    assert_eq!(serde_json::to_value(&reply).unwrap()["refused"], Value::Bool(true));
    let kept = list_fields::event(&db, state.clone(), "tracks", "album.tracks", ItemKind::Integer, &[], "1", None, drawn, drawn).unwrap();
    assert!(serde_json::to_value(&kept).unwrap().get("refused").is_none(), "refused travels only when true");
}
