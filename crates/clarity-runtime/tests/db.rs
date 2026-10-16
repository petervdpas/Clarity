//! A record written the way Clarity generates them, stored in SQLite.

use std::fmt;
use std::str::FromStr;

use clarity_runtime::{Db, Decimal, Error, Id, NaiveDate, Record, Show, SqlValue, require, sql};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum CustomerKind {
    #[default]
    Private,
    Business,
}

impl fmt::Display for CustomerKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            CustomerKind::Private => "Private",
            CustomerKind::Business => "Business",
        })
    }
}

impl FromStr for CustomerKind {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, ()> {
        match s {
            "Private" => Ok(CustomerKind::Private),
            "Business" => Ok(CustomerKind::Business),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
struct Customer {
    id: Id,
    name: String,
    kind: CustomerKind,
    credit_limit: Decimal,
    active: bool,
    since: Option<NaiveDate>,
    referred_by: Option<Id>,
}

impl Record for Customer {
    const TABLE: &'static str = "customer";
    const COLUMNS: &'static [&'static str] = &["name", "kind", "credit_limit", "active", "since", "referred_by"];

    fn id(&self) -> Id {
        self.id
    }

    fn set_id(&mut self, id: Id) {
        self.id = id;
    }

    fn to_values(&self) -> Vec<SqlValue> {
        vec![
            sql::text(&self.name),
            sql::name(&self.kind),
            sql::decimal(self.credit_limit),
            sql::boolean(self.active),
            sql::date(self.since),
            sql::reference(self.referred_by),
        ]
    }

    fn from_values(id: Id, values: &[SqlValue]) -> clarity_runtime::Result<Self> {
        Ok(Customer {
            id,
            name: sql::get_text("name", &values[0])?,
            kind: sql::get_name("kind", &values[1])?,
            credit_limit: sql::get_decimal("credit_limit", &values[2])?,
            active: sql::get_boolean("active", &values[3])?,
            since: sql::get_date("since", &values[4])?,
            referred_by: sql::get_reference("referred_by", &values[5])?,
        })
    }
}

const STEPS: [&str; 1] = ["CREATE TABLE customer (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    kind TEXT NOT NULL,
    credit_limit TEXT NOT NULL,
    active INTEGER NOT NULL,
    since TEXT,
    referred_by INTEGER REFERENCES customer (id)
);"];

fn db() -> Db {
    let mut db = Db::memory().unwrap();
    assert_eq!(db.migrate(&STEPS).unwrap(), 1);
    db
}

fn ada() -> Customer {
    Customer {
        name: "Ada Lovelace".into(),
        kind: CustomerKind::Business,
        credit_limit: Decimal::new(125050, 2),
        active: true,
        since: NaiveDate::from_ymd_opt(2026, 1, 15),
        ..Customer::default()
    }
}

#[test]
fn saves_loads_updates_and_deletes_records() {
    let db = db();
    let mut customer = ada();
    db.save(&mut customer).unwrap();
    assert_eq!(customer.id, 1);
    assert_eq!(db.get::<Customer>(1).unwrap(), customer);

    customer.credit_limit = Decimal::new(99, 1);
    db.save(&mut customer).unwrap();
    assert_eq!(db.get::<Customer>(1).unwrap().credit_limit.to_string(), "9.9");

    let mut grace = Customer {
        name: "Grace Hopper".into(),
        referred_by: Some(customer.id),
        ..Customer::default()
    };
    db.save(&mut grace).unwrap();
    let all = db.all::<Customer>().unwrap();
    assert_eq!(all.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(), ["Ada Lovelace", "Grace Hopper"]);
    assert_eq!(db.load::<Customer>(grace.referred_by).unwrap().name, "Ada Lovelace");

    // A record another record refers to stays until that reference is gone.
    assert_eq!(db.delete(&customer), Err(Error::InUse { table: "customer", id: 1 }));
    db.delete(&grace).unwrap();
    db.delete(&customer).unwrap();
    assert_eq!(db.get::<Customer>(1), Err(Error::NotFound { table: "customer", id: Some(1) }));
    assert_eq!(db.load::<Customer>(None), Err(Error::NotFound { table: "customer", id: None }));
}

#[test]
fn applies_each_migration_step_once() {
    let mut db = db();
    assert_eq!(db.schema_version().unwrap(), 1);
    assert_eq!(db.migrate(&STEPS).unwrap(), 0);
    let more = [STEPS[0], "ALTER TABLE customer ADD COLUMN notes TEXT;"];
    assert_eq!(db.migrate(&more).unwrap(), 1);
    assert_eq!(db.schema_version().unwrap(), 2);
}

#[test]
fn a_failed_step_changes_nothing() {
    let mut db = db();
    let broken = [STEPS[0], "ALTER TABLE customer ADD COLUMN a TEXT; THIS IS NOT SQL;"];
    assert!(db.migrate(&broken).is_err());
    assert_eq!(db.schema_version().unwrap(), 1);
}

#[test]
fn contracts_and_text() {
    assert_eq!(require(true, "x > 0"), Ok(()));
    assert_eq!(require(false, "x > 0"), Err(Error::Contract("x > 0".into())));
    assert_eq!(Decimal::new(125050, 2).show(), "1250.50");
    assert_eq!(NaiveDate::from_ymd_opt(2026, 1, 15).show(), "2026-01-15");
    assert_eq!(None::<NaiveDate>.show(), "");
}

#[test]
fn a_reply_carries_the_state_messages_and_windows() {
    use clarity_runtime::ui::{Level, Ui, WindowState, from_data, to_data, to_value};

    #[derive(serde::Serialize, serde::Deserialize)]
    struct Form {
        name: String,
        rows: Vec<String>,
    }

    let mut state = WindowState::new("Form", to_data(&Form { name: "a".into(), rows: vec!["x".into(), "y".into()] }).unwrap());
    state.selected.insert("grid".into(), 1);
    let mut ui = Ui::from_state(&state);
    let mut form: Form = from_data(state.data.clone()).unwrap();
    assert!(ui.enabled("save"));
    ui.set_enabled("save", false);
    assert_eq!(ui.caption("save", "Save"), "Save");
    assert_eq!(ui.selected("grid", &form.rows).unwrap(), "y");
    assert!(ui.selected("other", &form.rows).is_err());
    form.name = "b".into();
    ui.message(Level::Warning, "careful".into());
    ui.open("Other", vec![("record", to_value(&form.rows[0]).unwrap())]);
    let reply = ui.reply("Form", to_data(&form).unwrap());
    assert_eq!(reply.state.data["name"], "b");
    assert_eq!(reply.state.parts["save"].enabled, Some(false));
    assert_eq!(reply.messages[0].text, "careful");
    assert_eq!(reply.open[0].with["record"], "x");
    assert!(!reply.close);
}
