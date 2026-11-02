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
    assert!(matches!(db.save(&mut order), Err(Error::Required { .. })));
    assert_eq!(db.find::<Order>(&clarity_runtime::query::Query::new()).unwrap(), vec![kept]);
}

/// A window with a form on an order: its number box and its customer box
/// show the order's fields, as the generated program names them.
const BOUND: &[clarity_runtime::required::Bound] = &[
    clarity_runtime::required::Bound { part: "numberBox", data: "order", table: "orders", column: "number" },
    clarity_runtime::required::Bound { part: "customerBox", data: "order", table: "orders", column: "customer_id" },
];

/// The save button's handler as generated code runs it: the order of the
/// window saved, the window replied with what it holds then.
fn save_clicked(db: &Db, state: clarity_runtime::ui::WindowState) -> Result<clarity_runtime::ui::Reply> {
    use clarity_runtime::ui::Ui;
    let mut data: serde_json::Map<String, Value> = state.data.clone();
    let mut order: Order = Order { id: data["order"]["id"].as_i64().unwrap(), number: data["order"]["number"].as_str().unwrap().into(), customer: data["order"]["customer"].as_i64() };
    db.save(&mut order)?;
    data.insert("order".into(), serde_json::json!({ "id": order.id, "number": order.number, "customer": order.customer }));
    Ok(Ui::from_state(&state).reply(&state.window, data))
}

fn window(order: Value) -> clarity_runtime::ui::WindowState {
    clarity_runtime::ui::WindowState::new("OrderForm", serde_json::json!({ "order": order }).as_object().unwrap().clone())
}

fn error(state: &clarity_runtime::ui::WindowState, part: &str) -> String {
    state.parts.get(part).and_then(|p| p.error.as_ref()).map(|e| e.show()).unwrap_or_default()
}

#[test]
fn a_refused_required_field_shows_why_at_its_input_and_code_gets_the_refusal() {
    use clarity_runtime::required::shown_at_input;
    let db = db();
    let drawn = |state: clarity_runtime::ui::WindowState| Ok(clarity_runtime::ui::Ui::from_state(&state).reply(&state.window, state.data.clone()));
    let reply = shown_at_input(window(serde_json::json!({ "id": 0, "number": "", "customer": null })), BOUND, |s| save_clicked(&db, s), drawn).unwrap();
    // The person sees why at the input that shows the field, as a form check shows it; no message beside it.
    assert_eq!(error(&reply.state, "numberBox"), "Number is required");
    assert_eq!(reply.state.parts["numberBox"].error.as_ref().unwrap().key.as_deref(), Some("fieldRequired"));
    assert_eq!(error(&reply.state, "customerBox"), "");
    assert!(reply.messages.is_empty());
    // The next try clears it, and the next empty field shows at its own input.
    let mut state = reply.state;
    state.data["order"]["number"] = Value::from("ORD-1");
    let reply = shown_at_input(state, BOUND, |s| save_clicked(&db, s), drawn).unwrap();
    assert_eq!(error(&reply.state, "numberBox"), "");
    assert_eq!(error(&reply.state, "customerBox"), "Customer is required");
    let mut state = reply.state;
    state.data["order"]["customer"] = Value::from(4);
    let reply = shown_at_input(state, BOUND, |s| save_clicked(&db, s), drawn).unwrap();
    assert_eq!(error(&reply.state, "customerBox"), "");
    assert_ne!(reply.state.data["order"]["id"], Value::from(0));
    // An error code set at the input since is its own, and stays.
    let mut state = reply.state;
    state.parts.entry("numberBox".into()).or_default().error = Some(clarity_runtime::ui::Text::plain("Too short"));
    let reply = shown_at_input(state, BOUND, |s| Ok(clarity_runtime::ui::Ui::from_state(&s).reply(&s.window, s.data.clone())), drawn).unwrap();
    assert_eq!(error(&reply.state, "numberBox"), "Too short");
    // Code gets the refusal it can handle: which record, which column.
    let Err(Error::Required { table, column, .. }) = db.save(&mut Order { number: "ORD-2".into(), ..Order::default() }) else { panic!("saved") };
    assert_eq!((table, column.as_str()), ("orders", "customer_id"));
    // A record no input shows (not the window's order) is refused as an error the screen says.
    let other = shown_at_input(window(serde_json::json!({ "id": 99, "number": "ORD-3", "customer": 4 })), BOUND, |_| db.save(&mut Order::default()).map(|_| unreachable!()), drawn);
    let Err(refused) = other else { panic!("shown") };
    assert_eq!(clarity_runtime::program::said(&refused).show(), "Number is required");
}
