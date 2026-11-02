//! A `required` field must have a value: a save refuses a record whose
//! required field is empty, as every runtime does (conformance/required.json).

use clarity_runtime::{Db, Error, Id, Record, Result, SqlValue, sql};
use serde_json::Value;

fn case_value(value: &Value) -> SqlValue {
    match value {
        Value::Null => SqlValue::Null,
        Value::Bool(b) => SqlValue::Bool(*b),
        Value::Number(n) => SqlValue::Integer(n.as_i64().unwrap()),
        Value::String(s) => SqlValue::Text(s.clone()),
        other => panic!("no case value: {other}"),
    }
}

#[test]
fn a_save_refuses_an_empty_required_field_as_every_runtime_does() {
    let file: Value = serde_json::from_str(include_str!("../../../conformance/required.json")).unwrap();
    let (key, english) = (file["refusal"]["key"].as_str().unwrap(), file["refusal"]["english"].as_str().unwrap());
    for c in file["cases"].as_array().unwrap() {
        let name = c["name"].as_str().unwrap();
        let required: Vec<(&str, &str)> = c["required"].as_array().unwrap().iter().map(|r| (r["column"].as_str().unwrap(), r["label"].as_str().unwrap())).collect();
        let values = c["values"].as_object().unwrap();
        let columns: Vec<&str> = values.keys().map(String::as_str).collect();
        let sql: Vec<SqlValue> = values.values().map(case_value).collect();
        match (clarity_runtime::check_required(&required, &columns, &sql), c["refused"].as_str()) {
            (Ok(()), None) => {}
            (Err(Error::Refused(text)), Some(label)) => {
                assert_eq!(text.key.as_deref(), Some(key), "{name}");
                assert_eq!(text.text, english, "{name}");
                assert_eq!(text.values.get("field").map(|s| s.value.clone()), Some(Value::String(label.into())), "{name}");
                assert_eq!(text.show(), format!("{label} is required"), "{name}");
            }
            (got, want) => panic!("{name}: got {got:?}, want {want:?}"),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
struct Order {
    id: Id,
    number: String,
    customer: Option<Id>,
}

impl Record for Order {
    const TABLE: &'static str = "orders";
    const COLUMNS: &'static [&'static str] = &["number", "customer_id"];
    const REQUIRED: &'static [(&'static str, &'static str)] = &[("number", "Number"), ("customer_id", "Customer")];
    fn id(&self) -> Id {
        self.id
    }
    fn set_id(&mut self, id: Id) {
        self.id = id;
    }
    fn to_values(&self) -> Vec<SqlValue> {
        vec![sql::text(&self.number), self.customer.map_or(SqlValue::Null, SqlValue::Integer)]
    }
    fn from_values(id: Id, values: &[SqlValue]) -> Result<Self> {
        Ok(Order { id, number: sql::get_text("number", &values[0])?, customer: match values[1] { SqlValue::Integer(i) => Some(i), _ => None } })
    }
}

fn db() -> Db {
    let mut db = Db::memory().unwrap();
    db.migrate(&[&["CREATE TABLE orders (id INTEGER PRIMARY KEY, number TEXT NOT NULL, customer_id INTEGER);"]]).unwrap();
    db
}

#[test]
fn an_order_without_its_customer_is_not_saved_and_stays_new() {
    let db = db();
    let mut order = Order { number: "ORD-1".into(), ..Order::default() };
    let refused = db.save(&mut order).unwrap_err();
    assert_eq!(refused.to_string(), "Customer is required");
    assert_eq!(order.id, 0);
    assert!(db.find::<Order>(&clarity_runtime::query::Query::new()).unwrap().is_empty());
    order.customer = Some(4);
    db.save(&mut order).unwrap();
    assert_ne!(order.id, 0);
    // A stored record that loses a required value is not changed either.
    let kept = order.clone();
    order.number = String::new();
    assert!(matches!(db.save(&mut order), Err(Error::Refused(_))));
    assert_eq!(db.find::<Order>(&clarity_runtime::query::Query::new()).unwrap(), vec![kept]);
}
