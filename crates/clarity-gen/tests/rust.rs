use clarity_gen::{OutFile, generate};
use clarity_project::{Backend, Settings, starter};
use clarity_spec::load_sources;

const SALES: &str = "module sales

enum OrderStatus
    Pending
    Approved

entity Customer
    name: text(80), required
    credit_limit: money, default 100
    active: bool, default true
    since: date

entity Order
    number: text(12), unique
    status: OrderStatus
    customer: ref Customer
    total: money

function within_credit(order: Order) -> bool
    return order.total <= order.customer.credit_limit

function doubled(n: integer) -> money
    ensure result >= 0
    let x: money = n * 2
    if x < 0
        return 0
    return x + 0.5

action approve(order: Order)
    require order.status == Pending
    require within_credit(order)
    effects db.write
    order.status = Approved
    save order
";

fn shop() -> Settings {
    starter("Shop", "shop")
}

fn files() -> Vec<OutFile> {
    let project = load_sources([("sales.clarity", SALES)]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    generate(&project, &shop()).unwrap()
}

fn file<'a>(files: &'a [OutFile], path: &str) -> &'a str {
    &files.iter().find(|f| f.path == path).unwrap_or_else(|| panic!("no {path}")).text
}

#[test]
fn writes_a_complete_cargo_project() {
    let files = files();
    let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(
        paths,
        [
            "Cargo.toml",
            "clarity-runtime/Cargo.toml",
            "clarity-runtime/src/lib.rs",
            "clarity-runtime/src/sql.rs",
            "src/main.rs",
            "src/sales.rs",
            "src/schema.rs",
        ]
    );
    let cargo = file(&files, "Cargo.toml");
    assert!(cargo.contains("name = \"shop\""), "{cargo}");
    assert!(cargo.contains("clarity-runtime = { path = \"clarity-runtime\" }"), "{cargo}");
    assert!(file(&files, "src/main.rs").contains("mod sales;"));
}

#[test]
fn is_deterministic() {
    assert_eq!(files(), files());
}

#[test]
fn entities_become_records_with_their_defaults() {
    let files = files();
    let sales = file(&files, "src/sales.rs");
    assert!(sales.contains("pub struct Customer {"), "{sales}");
    assert!(sales.contains("    pub credit_limit: Decimal,"), "{sales}");
    assert!(sales.contains("    pub since: Option<NaiveDate>,"), "{sales}");
    assert!(sales.contains("    pub customer: Option<Id>,"), "{sales}");
    assert!(sales.contains("credit_limit: Decimal::new(100, 0),"), "{sales}");
    assert!(sales.contains("active: true,"), "{sales}");
    assert!(sales.contains("const TABLE: &'static str = \"order\";"), "{sales}");
    assert!(sales.contains("pub enum OrderStatus {"), "{sales}");
}

#[test]
fn the_schema_is_the_first_migration() {
    let files = files();
    let schema = file(&files, "src/schema.rs");
    assert!(schema.contains("CREATE TABLE \"order\" ("), "{schema}");
    assert!(schema.contains("\"number\" TEXT NOT NULL DEFAULT '' UNIQUE"), "{schema}");
    assert!(schema.contains("\"customer\" INTEGER REFERENCES \"customer\" (\"id\")"), "{schema}");
    assert!(schema.contains("\"credit_limit\" TEXT NOT NULL DEFAULT '100'"), "{schema}");
    assert!(schema.contains("pub const STEPS: &[&str] = &["), "{schema}");
}

#[test]
fn functions_and_actions_become_rust_functions() {
    let files = files();
    let sales = file(&files, "src/sales.rs");
    assert!(sales.contains("pub fn within_credit(db: &Db, order: &Order) -> Result<bool> {"), "{sales}");
    assert!(
        sales.contains("(order.total <= db.load::<Customer>(order.customer)?.credit_limit)"),
        "{sales}"
    );
    assert!(sales.contains("pub fn approve(db: &Db, order: &mut Order) -> Result<()> {"), "{sales}");
    assert!(sales.contains("require((order.status == OrderStatus::Pending), \"order.status == Pending\")?;"), "{sales}");
    assert!(sales.contains("require(within_credit(db, order)?, \"within_credit(order)\")?;"), "{sales}");
    assert!(sales.contains("order.status = OrderStatus::Approved;"), "{sales}");
    assert!(sales.contains("db.save(order)?;"), "{sales}");
    // Integers widen where a decimal is expected, and ensure wraps the body.
    assert!(sales.contains("let mut x: Decimal = Decimal::from((n * 2i64));"), "{sales}");
    assert!(sales.contains("return Ok(Decimal::from(0i64));"), "{sales}");
    assert!(sales.contains("return Ok((x + Decimal::new(5, 1)));"), "{sales}");
    assert!(sales.contains("ensure((result >= Decimal::from(0i64)), \"result >= 0\")?;"), "{sales}");
}

#[test]
fn refuses_backends_that_are_not_generated_yet() {
    let project = load_sources([("sales.clarity", SALES)]);
    let mut settings = shop();
    settings.stack.backend = Backend::Go;
    let errors = generate(&project, &settings).unwrap_err();
    assert_eq!(errors, ["the Go backend is not generated yet; Rust is"]);
}

#[test]
fn refuses_a_project_with_errors() {
    let project = load_sources([("sales.clarity", "module sales\n\nentity A\n    b: Nope\n")]);
    let errors = generate(&project, &shop()).unwrap_err();
    assert!(errors[0].contains("fix the errors"), "{errors:?}");
}
