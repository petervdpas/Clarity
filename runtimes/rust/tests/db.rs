//! A record written the way Clarity generates them, stored in SQLite.

use std::fmt;
use std::str::FromStr;

use clarity_runtime::{Cond, Db, Dialect, Decimal, Error, Id, NaiveDate, Op, Query, Record, Show, SqlValue, require, sql};

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

const CUSTOMERS: &str = "CREATE TABLE customer (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    kind TEXT NOT NULL,
    credit_limit TEXT NOT NULL,
    active INTEGER NOT NULL,
    since TEXT,
    referred_by INTEGER REFERENCES customer (id)
);";
const STEPS: [&[&str]; 1] = [&[CUSTOMERS]];

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
    let more = [STEPS[0], &["ALTER TABLE customer ADD COLUMN notes TEXT;"]];
    assert_eq!(db.migrate(&more).unwrap(), 1);
    assert_eq!(db.schema_version().unwrap(), 2);
}

#[test]
fn a_failed_step_changes_nothing() {
    let mut db = db();
    let broken = [STEPS[0], &["ALTER TABLE customer ADD COLUMN a TEXT; THIS IS NOT SQL;"]];
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
    assert_eq!(reply.messages[0].text.show(), "careful");
    assert_eq!(reply.open[0].with["record"], "x");
    assert!(!reply.close);
}

/// A text value a handler sets travels as the text as written with its slots,
/// so the screen shows it in the person's language and format.
#[test]
fn a_reply_carries_the_texts_shown_in_text_values() {
    use clarity_runtime::ui::{Slot, Text, Ui, WindowState};

    let mut ui = Ui::from_state(&WindowState::new("Game", serde_json::Map::new()));
    ui.set_shown("rules", Text::new("Your move, {name}").with("name", Slot::text("Ada")));
    ui.set_shown("hint", Text::plain("Pick a square"));
    let reply = serde_json::to_value(ui.reply("Game", serde_json::Map::new())).unwrap();
    assert_eq!(reply["shown"]["hint"], "Pick a square");
    assert_eq!(reply["shown"]["rules"]["text"], "Your move, {name}");
    assert_eq!(reply["shown"]["rules"]["values"]["name"]["value"], "Ada");
    // A reply without them carries none.
    let quiet = serde_json::to_value(Ui::default().reply("Game", serde_json::Map::new())).unwrap();
    assert!(quiet.get("shown").is_none());
}

fn customer(name: &str, kind: CustomerKind, limit: &str, active: bool) -> Customer {
    Customer {
        name: name.into(),
        kind,
        credit_limit: limit.parse().unwrap(),
        active,
        ..Customer::default()
    }
}

/// Ada (Business, 1250.50), Grace (Private, 99.5) and Linus (Business, 900,
/// inactive), stored in that order.
fn three() -> (Db, Vec<Customer>) {
    let db = db();
    let mut all = vec![
        customer("Ada Lovelace", CustomerKind::Business, "1250.50", true),
        customer("Grace Hopper", CustomerKind::Private, "99.5", true),
        customer("Linus", CustomerKind::Business, "900", false),
    ];
    for c in &mut all {
        db.save(c).unwrap();
    }
    (db, all)
}

fn names(found: &[Customer]) -> Vec<&str> {
    found.iter().map(|c| c.name.as_str()).collect()
}

#[test]
fn finds_records_by_a_condition() {
    let (db, _) = three();
    let business = Query::new().filter(Cond::compare("kind", Op::Eq, sql::name(&CustomerKind::Business)));
    assert_eq!(names(&db.find::<Customer>(&business).unwrap()), ["Ada Lovelace", "Linus"]);

    // Contains ignores case.
    let named = Query::new().filter(Cond::contains("name", "HOP"));
    assert_eq!(names(&db.find::<Customer>(&named).unwrap()), ["Grace Hopper"]);

    // Decimals compare as numbers: 99.5 is less than 900, though "99.5" sorts after "900".
    let big = Query::new().filter(Cond::number("credit_limit", Op::Gt, sql::decimal("900".parse().unwrap())));
    assert_eq!(names(&db.find::<Customer>(&big).unwrap()), ["Ada Lovelace"]);

    let active_business = Query::new().filter(
        Cond::is_true("active").and(Cond::one_of("kind", vec![sql::name(&CustomerKind::Business)])),
    );
    assert_eq!(names(&db.find::<Customer>(&active_business).unwrap()), ["Ada Lovelace"]);

    let inactive_or_private = Query::new().filter(
        Cond::is_true("active").negated().or(Cond::compare("kind", Op::Eq, sql::name(&CustomerKind::Private))),
    );
    assert_eq!(names(&db.find::<Customer>(&inactive_or_private).unwrap()), ["Grace Hopper", "Linus"]);
}

#[test]
fn finds_records_by_a_reference() {
    let (db, all) = three();
    let mut referred = customer("Referred", CustomerKind::Private, "0", true);
    referred.referred_by = Some(all[0].id);
    db.save(&mut referred).unwrap();
    let query = Query::new().filter(Cond::compare("referred_by", Op::Eq, sql::reference(Some(all[0].id))));
    assert_eq!(names(&db.find::<Customer>(&query).unwrap()), ["Referred"]);
}

#[test]
fn orders_what_it_finds() {
    let (db, _) = three();
    let by_limit = Query::new().order_number("credit_limit", true);
    assert_eq!(names(&db.find::<Customer>(&by_limit).unwrap()), ["Ada Lovelace", "Linus", "Grace Hopper"]);
    let by_kind_then_name = Query::new().order("kind", false).order("name", true);
    assert_eq!(
        names(&db.find::<Customer>(&by_kind_then_name).unwrap()),
        ["Linus", "Ada Lovelace", "Grace Hopper"]
    );
    // Without an order, records come in the order they were stored.
    assert_eq!(names(&db.find::<Customer>(&Query::new()).unwrap()), ["Ada Lovelace", "Grace Hopper", "Linus"]);
}

#[test]
fn contains_treats_wildcards_as_text() {
    let db = db();
    let mut odd = customer("100% sure_thing", CustomerKind::Private, "0", true);
    db.save(&mut odd).unwrap();
    let mut plain = customer("100 sure things", CustomerKind::Private, "0", true);
    db.save(&mut plain).unwrap();
    let query = Query::new().filter(Cond::contains("name", "0% sure_"));
    assert_eq!(names(&db.find::<Customer>(&query).unwrap()), ["100% sure_thing"]);
    assert!(clarity_runtime::contains("Ada Lovelace", "LOVE"));
    assert!(!clarity_runtime::contains("Ada", "Grace"));
}

#[test]
fn a_query_is_sql_with_parameters() {
    let query = Query::new()
        .filter(Cond::contains("name", "a").and(Cond::number("credit_limit", Op::LtEq, sql::integer(5)).negated()))
        .order("name", true);
    let (sql, params) = query.sql(Dialect::Sqlite, "customer", "id", "\"id\", \"name\"");
    assert_eq!(
        sql,
        "SELECT \"id\", \"name\" FROM \"customer\" WHERE (\"name\" LIKE ?1 ESCAPE '!' AND NOT (CAST(\"credit_limit\" AS REAL) <= CAST(?2 AS REAL))) ORDER BY \"name\" DESC, \"id\""
    );
    assert_eq!(params, [sql::text("%a%"), sql::integer(5)]);
}

/// The table built again, as a step that changes a column does it.
const REBUILD: &str = "CREATE TABLE customer__new (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    kind TEXT NOT NULL,
    credit_limit TEXT NOT NULL,
    active INTEGER NOT NULL,
    since TEXT,
    referred_by INTEGER REFERENCES customer (id)
);
INSERT INTO customer__new (id, name, kind, credit_limit, active, since, referred_by)
    SELECT id, name, kind, credit_limit, active, since, referred_by FROM customer;
DROP TABLE customer;
ALTER TABLE customer__new RENAME TO customer;";

#[test]
fn a_step_can_build_a_referenced_table_again() {
    let mut db = db();
    let mut ada = ada();
    db.save(&mut ada).unwrap();
    let mut grace = customer("Grace Hopper", CustomerKind::Private, "0", true);
    grace.referred_by = Some(ada.id);
    db.save(&mut grace).unwrap();

    assert_eq!(db.migrate(&[STEPS[0], &[REBUILD]]).unwrap(), 1);
    assert_eq!(db.get::<Customer>(grace.id).unwrap().referred_by, Some(ada.id));
    // References are checked again after the step.
    assert_eq!(db.delete(&ada), Err(Error::InUse { table: "customer", id: ada.id }));
}

#[test]
fn a_step_that_breaks_references_changes_nothing() {
    let mut db = db();
    let mut grace = customer("Grace Hopper", CustomerKind::Private, "0", true);
    db.save(&mut grace).unwrap();
    let dangling = "UPDATE customer SET referred_by = 99;";
    assert!(db.migrate(&[STEPS[0], &[dangling]]).is_err());
    assert_eq!(db.schema_version().unwrap(), 1);
    assert_eq!(db.get::<Customer>(grace.id).unwrap().referred_by, None);
}

/// A record kept in another database than the customers.
#[derive(Debug, Clone, Default, PartialEq)]
struct Invoice {
    id: Id,
    number: String,
    customer: Option<Id>,
}

impl Record for Invoice {
    const SOURCE: &'static str = "erp";
    const TABLE: &'static str = "invoice";
    const COLUMNS: &'static [&'static str] = &["number", "customer"];

    fn id(&self) -> Id {
        self.id
    }

    fn set_id(&mut self, id: Id) {
        self.id = id;
    }

    fn to_values(&self) -> Vec<SqlValue> {
        vec![sql::text(&self.number), sql::reference(self.customer)]
    }

    fn from_values(id: Id, values: &[SqlValue]) -> clarity_runtime::Result<Self> {
        Ok(Invoice {
            id,
            number: sql::get_text("number", &values[0])?,
            customer: sql::get_reference("customer", &values[1])?,
        })
    }
}

#[test]
fn each_record_lives_in_its_own_data_source() {
    let dir = std::env::temp_dir().join(format!("clarity-sources-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let main = dir.join("main.db");
    let erp = dir.join("erp.db");
    let mut db = Db::connect(&[("main", Dialect::Sqlite, main.to_str().unwrap()), ("erp", Dialect::Sqlite, erp.to_str().unwrap())]).unwrap();
    db.migrate_source("main", &STEPS).unwrap();
    // A reference to another database is a plain id: no foreign key.
    db.migrate_source("erp", &[&["CREATE TABLE invoice (id INTEGER PRIMARY KEY, number TEXT NOT NULL, customer INTEGER);"]])
        .unwrap();

    let mut ada = ada();
    db.save(&mut ada).unwrap();
    let mut invoice = Invoice { number: "INV-1".into(), customer: Some(ada.id), ..Invoice::default() };
    db.save(&mut invoice).unwrap();
    drop(db);

    // Each table is in its own file.
    let only_main = Db::open(&main).unwrap();
    assert_eq!(only_main.all::<Customer>().unwrap().len(), 1);
    let db = Db::connect(&[("main", Dialect::Sqlite, main.to_str().unwrap()), ("erp", Dialect::Sqlite, erp.to_str().unwrap())]).unwrap();
    assert_eq!(db.all::<Invoice>().unwrap(), [invoice]);
    assert_eq!(db.load::<Customer>(db.get::<Invoice>(1).unwrap().customer).unwrap().name, "Ada Lovelace");
    std::fs::remove_dir_all(&dir).unwrap();
}

/// A record of a table that was there before Clarity: its own key column
/// and column names.
#[derive(Debug, Clone, Default, PartialEq)]
struct Legacy {
    id: Id,
    number: String,
}

impl Record for Legacy {
    const TABLE: &'static str = "tblInvoice";
    const KEY: &'static str = "InvoiceID";
    const COLUMNS: &'static [&'static str] = &["InvNo"];

    fn id(&self) -> Id {
        self.id
    }

    fn set_id(&mut self, id: Id) {
        self.id = id;
    }

    fn to_values(&self) -> Vec<SqlValue> {
        vec![sql::text(&self.number)]
    }

    fn from_values(id: Id, values: &[SqlValue]) -> clarity_runtime::Result<Self> {
        Ok(Legacy {
            id,
            number: sql::get_text("InvNo", &values[0])?,
        })
    }
}

#[test]
fn a_record_can_live_in_an_existing_table() {
    let mut db = Db::memory().unwrap();
    db.migrate(&[&["CREATE TABLE tblInvoice (InvoiceID INTEGER PRIMARY KEY, InvNo TEXT NOT NULL, Other TEXT);"]])
        .unwrap();
    let mut first = Legacy { number: "INV-1".into(), ..Legacy::default() };
    db.save(&mut first).unwrap();
    let mut second = Legacy { number: "INV-2".into(), ..Legacy::default() };
    db.save(&mut second).unwrap();
    assert_eq!(second.id, 2);
    second.number = "INV-2b".into();
    db.save(&mut second).unwrap();
    assert_eq!(db.get::<Legacy>(2).unwrap().number, "INV-2b");
    let found = db.find::<Legacy>(&Query::new().filter(Cond::contains("InvNo", "2b"))).unwrap();
    assert_eq!(found, [second.clone()]);
    db.delete(&first).unwrap();
    assert_eq!(db.all::<Legacy>().unwrap(), [second]);
}

#[test]
fn values_written_by_other_programs_are_read() {
    // SQLite keeps what was written: numbers in text columns, dates with
    // seconds or a T.
    assert_eq!(sql::get_text("code", &SqlValue::Integer(42)).unwrap(), "42");
    assert_eq!(sql::get_text("ratio", &SqlValue::Real(1.5)).unwrap(), "1.5");
    let at = |s: &str| sql::get_date_time("at", &SqlValue::Text(s.into())).unwrap().unwrap().to_string();
    assert_eq!(at("2026-10-17 09:30"), "2026-10-17 09:30:00");
    assert_eq!(at("2026-10-17 09:30:15"), "2026-10-17 09:30:15");
    assert_eq!(at("2026-10-17T09:30:15"), "2026-10-17 09:30:15");
    let day = sql::get_date("on", &SqlValue::Text("2026-10-17 00:00:00".into())).unwrap().unwrap();
    assert_eq!(day.to_string(), "2026-10-17");
}
