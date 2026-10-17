//! The generated migration steps applied by the runtime to real
//! PostgreSQL, MySQL and SQL Server databases, with records stored between
//! the steps. Each test needs a scratch database whose tables it may drop,
//! named by an environment variable, and is skipped without it:
//!
//! ```text
//! CLARITY_TEST_POSTGRES="host=localhost port=5432 user=postgres password=... dbname=scratch"
//! CLARITY_TEST_MYSQL="mysql://root:...@localhost:3306/scratch"
//! CLARITY_TEST_MSSQL="server=tcp:localhost,1433;database=scratch;user=sa;password=...;TrustServerCertificate=true"
//! ```

use clarity_project::SourceKind;
use clarity_runtime::{Cond, Db, Decimal, Dialect, Error, Id, NaiveDate, NaiveDateTime, Op, Query, Record, SqlValue, sql};

const SHOP: &str = "module shop

enum Kind
    Private
    Business

entity Customer
    name: text(80), unique
    email: email, unique
    kind: Kind
    credit_limit: money, default 100
    vip: bool
    remarks: text
    phone: text(20)

entity Purchase
    number: text(12), unique
    customer: ref Customer
    total: money
    placed: datetime
    paid: integer
    code: integer
";

const FIRST: &str = "migration \"First\"
    add entity Customer
        name: text(80), unique
        email: email
        kind: Kind
        limit: money, default 100
        active: bool, default true
        since: date
        notes: text
    add entity Order
        number: text(12)
        customer: ref Customer
        total: integer
        placed: datetime
        paid: bool
        code: text
";

const SECOND: &str = "migration \"Second\"
    rename field Customer.limit to credit_limit
    rename field Customer.notes to remarks
    rename field Customer.active to vip
    remove field Customer.since
    add field Customer.phone: text(20)
    add unique Customer.email
    change field Order.total: money
    add unique Order.number
    change field Order.paid: integer
    change field Order.code: integer
    rename entity Order to Purchase
";

/// The statements of both steps for a database of `kind`.
fn steps(kind: SourceKind) -> Vec<Vec<String>> {
    let project = clarity_spec::load_with_migrations([("shop.clarity", SHOP)], [("0001.clarity", FIRST), ("0002.clarity", SECOND)]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    clarity_gen::migration_sql(&project, "main", kind).unwrap().into_iter().map(|(_, s)| s).collect()
}


/// A customer as the first step stores it.
#[derive(Debug, Clone, Default, PartialEq)]
struct Before {
    id: Id,
    name: String,
    email: String,
    kind: String,
    limit: Decimal,
    active: bool,
    since: Option<NaiveDate>,
    notes: String,
}

impl Record for Before {
    const TABLE: &'static str = "customer";
    const COLUMNS: &'static [&'static str] = &["name", "email", "kind", "limit", "active", "since", "notes"];
    fn id(&self) -> Id {
        self.id
    }
    fn set_id(&mut self, id: Id) {
        self.id = id;
    }
    fn to_values(&self) -> Vec<SqlValue> {
        vec![
            sql::text(&self.name),
            sql::text(&self.email),
            sql::text(&self.kind),
            sql::decimal(self.limit),
            sql::boolean(self.active),
            sql::date(self.since),
            sql::text(&self.notes),
        ]
    }
    fn from_values(id: Id, v: &[SqlValue]) -> clarity_runtime::Result<Self> {
        Ok(Before {
            id,
            name: sql::get_text("name", &v[0])?,
            email: sql::get_text("email", &v[1])?,
            kind: sql::get_text("kind", &v[2])?,
            limit: sql::get_decimal("limit", &v[3])?,
            active: sql::get_boolean("active", &v[4])?,
            since: sql::get_date("since", &v[5])?,
            notes: sql::get_text("notes", &v[6])?,
        })
    }
}

/// An order as the first step stores it.
#[derive(Debug, Clone, Default, PartialEq)]
struct Order {
    id: Id,
    number: String,
    customer: Option<Id>,
    total: i64,
    placed: Option<NaiveDateTime>,
    paid: bool,
    code: String,
}

impl Record for Order {
    const TABLE: &'static str = "order";
    const COLUMNS: &'static [&'static str] = &["number", "customer", "total", "placed", "paid", "code"];
    fn id(&self) -> Id {
        self.id
    }
    fn set_id(&mut self, id: Id) {
        self.id = id;
    }
    fn to_values(&self) -> Vec<SqlValue> {
        vec![
            sql::text(&self.number),
            sql::reference(self.customer),
            sql::integer(self.total),
            sql::date_time(self.placed),
            sql::boolean(self.paid),
            sql::text(&self.code),
        ]
    }
    fn from_values(id: Id, v: &[SqlValue]) -> clarity_runtime::Result<Self> {
        Ok(Order {
            id,
            number: sql::get_text("number", &v[0])?,
            customer: sql::get_reference("customer", &v[1])?,
            total: sql::get_integer("total", &v[2])?,
            placed: sql::get_date_time("placed", &v[3])?,
            paid: sql::get_boolean("paid", &v[4])?,
            code: sql::get_text("code", &v[5])?,
        })
    }
}

/// A customer after the second step.
#[derive(Debug, Clone, Default, PartialEq)]
struct Customer {
    id: Id,
    name: String,
    email: String,
    kind: String,
    credit_limit: Decimal,
    vip: bool,
    remarks: String,
    phone: String,
}

impl Record for Customer {
    const TABLE: &'static str = "customer";
    const COLUMNS: &'static [&'static str] = &["name", "email", "kind", "credit_limit", "vip", "remarks", "phone"];
    fn id(&self) -> Id {
        self.id
    }
    fn set_id(&mut self, id: Id) {
        self.id = id;
    }
    fn to_values(&self) -> Vec<SqlValue> {
        vec![
            sql::text(&self.name),
            sql::text(&self.email),
            sql::text(&self.kind),
            sql::decimal(self.credit_limit),
            sql::boolean(self.vip),
            sql::text(&self.remarks),
            sql::text(&self.phone),
        ]
    }
    fn from_values(id: Id, v: &[SqlValue]) -> clarity_runtime::Result<Self> {
        Ok(Customer {
            id,
            name: sql::get_text("name", &v[0])?,
            email: sql::get_text("email", &v[1])?,
            kind: sql::get_text("kind", &v[2])?,
            credit_limit: sql::get_decimal("credit_limit", &v[3])?,
            vip: sql::get_boolean("vip", &v[4])?,
            remarks: sql::get_text("remarks", &v[5])?,
            phone: sql::get_text("phone", &v[6])?,
        })
    }
}

/// An order after the second step: another table, and a money total.
#[derive(Debug, Clone, Default, PartialEq)]
struct Purchase {
    id: Id,
    number: String,
    customer: Option<Id>,
    total: Decimal,
    placed: Option<NaiveDateTime>,
    paid: i64,
    code: i64,
}

impl Record for Purchase {
    const TABLE: &'static str = "purchase";
    const COLUMNS: &'static [&'static str] = &["number", "customer", "total", "placed", "paid", "code"];
    fn id(&self) -> Id {
        self.id
    }
    fn set_id(&mut self, id: Id) {
        self.id = id;
    }
    fn to_values(&self) -> Vec<SqlValue> {
        vec![
            sql::text(&self.number),
            sql::reference(self.customer),
            sql::decimal(self.total),
            sql::date_time(self.placed),
            sql::integer(self.paid),
            sql::integer(self.code),
        ]
    }
    fn from_values(id: Id, v: &[SqlValue]) -> clarity_runtime::Result<Self> {
        Ok(Purchase {
            id,
            number: sql::get_text("number", &v[0])?,
            customer: sql::get_reference("customer", &v[1])?,
            total: sql::get_decimal("total", &v[2])?,
            placed: sql::get_date_time("placed", &v[3])?,
            paid: sql::get_integer("paid", &v[4])?,
            code: sql::get_integer("code", &v[5])?,
        })
    }
}

fn names<R>(found: &[R], name: fn(&R) -> &str) -> Vec<String> {
    found.iter().map(|r| name(r).to_string()).collect()
}

/// Both steps on a scratch database, with records kept across them.
fn migrates_and_keeps_records(variable: &str, dialect: Dialect, kind: SourceKind) {
    let Ok(at) = std::env::var(variable) else {
        eprintln!("{variable} is not set; skipped");
        return;
    };
    let mut db = Db::connect(&[("main", dialect, &at)]).unwrap();
    for table in ["purchase", "order", "customer", "clarity_schema"] {
        db.batch("main", &format!("DROP TABLE IF EXISTS {}", dialect.quote(table))).unwrap();
    }
    let steps = steps(kind);
    let steps: Vec<Vec<&str>> = steps.iter().map(|s| s.iter().map(String::as_str).collect()).collect();
    let steps: Vec<&[&str]> = steps.iter().map(Vec::as_slice).collect();

    assert_eq!(db.migrate(&steps[..1]).unwrap(), 1);
    let since = NaiveDate::from_ymd_opt(2026, 1, 15);
    let mut ada = Before {
        name: "Ada Lovelace".into(),
        email: "ada@example.com".into(),
        kind: "Business".into(),
        limit: "1250.50".parse().unwrap(),
        active: true,
        since,
        notes: "first's".into(),
        ..Before::default()
    };
    db.save(&mut ada).unwrap();
    let mut grace = Before { name: "Grace Hopper".into(), kind: "Private".into(), limit: "99.5".parse().unwrap(), ..Before::default() };
    db.save(&mut grace).unwrap();
    assert_eq!(db.get::<Before>(ada.id).unwrap(), ada);
    // Saving what is stored already is no missing record.
    db.save(&mut grace).unwrap();

    let placed = NaiveDate::from_ymd_opt(2026, 10, 17).unwrap().and_hms_opt(9, 30, 15);
    let mut order = Order { number: "ORD-1".into(), customer: Some(ada.id), total: 99, placed, paid: true, code: "42".into(), ..Order::default() };
    db.save(&mut order).unwrap();
    let mut blank = Order { number: "ORD-0".into(), ..Order::default() };
    db.save(&mut blank).unwrap();
    db.save(&mut order).unwrap();
    assert_eq!(db.delete(&ada), Err(Error::InUse { table: "customer", id: ada.id }));

    // Queries: contains ignores case and wildcards, decimals compare as numbers.
    let named = Query::new().filter(Cond::contains("name", "LOVE"));
    assert_eq!(names(&db.find::<Before>(&named).unwrap(), |c| &c.name), ["Ada Lovelace"]);
    assert!(db.find::<Before>(&Query::new().filter(Cond::contains("name", "%"))).unwrap().is_empty());
    let big = Query::new().filter(Cond::number("limit", Op::Gt, sql::integer(900)).and(Cond::is_true("active")));
    assert_eq!(names(&db.find::<Before>(&big).unwrap(), |c| &c.name), ["Ada Lovelace"]);
    let by_limit = Query::new().order_number("limit", false);
    assert_eq!(names(&db.find::<Before>(&by_limit).unwrap(), |c| &c.name), ["Grace Hopper", "Ada Lovelace"]);
    let on_day = Query::new().filter(Cond::compare("since", Op::Eq, sql::date(since)));
    assert_eq!(db.find::<Before>(&on_day).unwrap().len(), 1);
    let one_of = Query::new().filter(Cond::one_of("kind", vec![sql::text("Private")]));
    assert_eq!(names(&db.find::<Before>(&one_of).unwrap(), |c| &c.name), ["Grace Hopper"]);

    assert_eq!(db.migrate(&steps).unwrap(), 1);
    assert_eq!(db.migrate(&steps).unwrap(), 0);
    assert_eq!(db.schema_version().unwrap(), 2);

    let customer = db.get::<Customer>(ada.id).unwrap();
    assert_eq!(customer.credit_limit, "1250.50".parse::<Decimal>().unwrap());
    assert!(customer.vip);
    assert_eq!(customer.remarks, "first's");
    assert_eq!(customer.phone, "");
    let purchase = db.get::<Purchase>(order.id).unwrap();
    assert_eq!(purchase.total, Decimal::from(99));
    assert_eq!(purchase.customer, Some(ada.id));
    assert_eq!(purchase.placed, placed);
    assert_eq!((purchase.paid, purchase.code), (1, 42));
    // Empty text is no number: it becomes the default.
    assert_eq!(db.get::<Purchase>(blank.id).unwrap().code, 0);

    // Unique email: empty many times, a filled one once.
    let mut linus = Customer { name: "Linus".into(), ..Customer::default() };
    db.save(&mut linus).unwrap();
    assert_eq!(linus.credit_limit, Decimal::ZERO);
    let mut copy = Customer { name: "Copy".into(), email: "ada@example.com".into(), ..Customer::default() };
    assert!(db.save(&mut copy).is_err());
    // The reference is kept through the renamed table.
    let ada_now = db.get::<Customer>(ada.id).unwrap();
    assert_eq!(db.delete(&ada_now), Err(Error::InUse { table: "customer", id: ada.id }));
    let mut dangling = Purchase { number: "ORD-2".into(), customer: Some(999), ..Purchase::default() };
    assert!(matches!(db.save(&mut dangling), Err(Error::Invalid(_))));
    db.delete(&purchase).unwrap();
    db.delete(&ada_now).unwrap();
    assert_eq!(db.all::<Customer>().unwrap().len(), 2);
}

#[test]
fn postgres() {
    migrates_and_keeps_records("CLARITY_TEST_POSTGRES", Dialect::Postgres, SourceKind::Postgres);
}

#[test]
fn mysql() {
    migrates_and_keeps_records("CLARITY_TEST_MYSQL", Dialect::Mysql, SourceKind::Mysql);
}

#[test]
fn mssql() {
    migrates_and_keeps_records("CLARITY_TEST_MSSQL", Dialect::Mssql, SourceKind::Mssql);
}

#[test]
fn unique_text_needs_a_length_where_the_database_asks_for_one() {
    let project = clarity_spec::load_sources([("shop.clarity", "module shop\n\nentity Note\n    body: text, unique\n")]);
    assert!(clarity_gen::migration_sql(&project, "main", SourceKind::Postgres).is_ok());
    let errors = clarity_gen::migration_sql(&project, "main", SourceKind::Mysql).unwrap_err();
    assert_eq!(
        errors,
        ["Note: MySQL keeps unique text up to 768 characters; give field 'body' a maximum length, such as text(200)"]
    );
    assert!(clarity_gen::migration_sql(&project, "main", SourceKind::Mssql).is_err());
}

/// The steps of the scenario above per database, kept in
/// conformance/migrations.json for the C# runtime's server tests, so both
/// runtimes run the same SQL. `CLARITY_WRITE_CONFORMANCE=1` writes the file
/// again after a change to the SQL writer.
#[test]
fn the_shared_migration_steps_are_current() {
    let mut databases = serde_json::Map::new();
    for (name, kind) in [("sqlite", SourceKind::Sqlite), ("postgres", SourceKind::Postgres), ("mysql", SourceKind::Mysql), ("mssql", SourceKind::Mssql)] {
        databases.insert(name.to_string(), serde_json::json!(steps(kind)));
    }
    let cases = serde_json::json!({
        "about": "The two migration steps of the server scenario (crates/clarity-gen/tests/servers.rs), as the generator writes them per database: a list of steps, each a list of statements. The Rust runtime runs them from the generator; the C# runtime's tests read them here (Clarity.Runtime.Tests/ServerTests.cs).",
        "steps": databases,
    });
    let text = serde_json::to_string_pretty(&cases).unwrap() + "\n";
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance/migrations.json");
    if std::env::var_os("CLARITY_WRITE_CONFORMANCE").is_some() {
        std::fs::write(&path, &text).unwrap();
    }
    let kept = std::fs::read_to_string(&path).unwrap_or_default();
    assert!(kept == text, "conformance/migrations.json is out of date; run this test with CLARITY_WRITE_CONFORMANCE=1");
}
