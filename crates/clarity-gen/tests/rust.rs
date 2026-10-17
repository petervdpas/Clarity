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
    since: date, format long

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

function big_orders(limit: money) -> list<Order>
    return find o in Order where o.total >= limit and o.status in [Pending, Approved] order by o.total desc, o.number

action approve(order: Order)
    require order.status == Pending
    require within_credit(order)
    effects db.write
    order.status = Approved
    save order

page OrderForm: window
    title \"Order\"
    data order: Order
    data orders: list<Order>
    on return: window_returned
    form name order_form at 1,1 span 12 rows 1 bind order on submit: order_submitted
        textbox name number_input at 1,1 span 4 bind order.number readonly
    table name grid at 1,2 span 12 rows 4 bind orders columns number, total on open: grid_opened
    button \"Approve\" name approve_button at 1,6 span 2 on click: approve_clicked
    textbox name number_filter at 1,7 span 4 on change: filter_changed

code OrderForm
    handler approve_clicked
        if not within_credit(order)
            message warning \"Too much\"
            approve_button.enabled = false
            return
        approve(order)
        message \"Approved {order.number}\"

    handler window_returned
        reload orders
        reload order
        filter_changed()

    handler filter_changed
        orders = find o in Order where o.number contains number_filter.text and not (o.customer == order.customer) order by o.number
        if order.number contains \"ORD\"
            number_filter.text = \"\"

    handler order_submitted
        number_input.error = \"Check the number\"
        number_input.readonly = not number_input.readonly

    handler grid_opened
        open OrderForm with order = grid.selected
        close
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
            "build.rs",
            "clarity-runtime/Cargo.toml",
            "clarity-runtime/src/dialect.rs",
            "clarity-runtime/src/lib.rs",
            "clarity-runtime/src/mssql.rs",
            "clarity-runtime/src/mysql.rs",
            "clarity-runtime/src/postgres.rs",
            "clarity-runtime/src/query.rs",
            "clarity-runtime/src/sql.rs",
            "clarity-runtime/src/sqlite.rs",
            "clarity-runtime/src/store.rs",
            "clarity-runtime/src/ui.rs",
            "icons/icon.png",
            "src/main.rs",
            "src/model.json",
            "src/sales.rs",
            "src/schema.rs",
            "tauri.conf.json",
            "ui/index.html",
            "ui/runtime.css",
            "ui/runtime.js",
            "ui/themes/clarity.css",
        ]
    );
    let cargo = file(&files, "Cargo.toml");
    assert!(cargo.contains("name = \"shop\""), "{cargo}");
    assert!(cargo.contains("clarity-runtime = { path = \"clarity-runtime\", default-features = false, features = [\"sqlite\"] }"), "{cargo}");
    let runtime = file(&files, "clarity-runtime/Cargo.toml");
    assert!(runtime.contains("edition = \"2024\"") && !runtime.contains("workspace"), "{runtime}");
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
    // Empty text is no value, so unique text is unique among filled values.
    assert!(schema.contains("\"number\" TEXT NOT NULL DEFAULT '',\n"), "{schema}");
    assert!(
        schema.contains("CREATE UNIQUE INDEX \"order_number_unique\" ON \"order\" (\"number\") WHERE \"number\" != '';"),
        "{schema}"
    );
    assert!(schema.contains("\"customer\" INTEGER REFERENCES \"customer\" (\"id\")"), "{schema}");
    assert!(schema.contains("\"credit_limit\" TEXT NOT NULL DEFAULT '100'"), "{schema}");
    assert!(schema.contains("pub const SOURCES: &[(&str, &[&[&str]])] = &[\n    (\"main\", &[\n"), "{schema}");
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

#[test]
fn handlers_work_on_the_window_and_the_screen() {
    let files = files();
    let sales = file(&files, "src/sales.rs");
    assert!(sales.contains("pub struct OrderFormWindow {\n    pub order: Order,\n    pub orders: Vec<Order>,\n}"), "{sales}");
    assert!(sales.contains("orders: db.all::<Order>()?,"), "{sales}");
    assert!(
        sales.contains("pub fn order_form__approve_clicked(db: &Db, w: &mut OrderFormWindow, ui: &mut Ui) -> Result<()> {"),
        "{sales}"
    );
    assert!(sales.contains("if !within_credit(db, &w.order)? {"), "{sales}");
    // Shown text goes to the screen as text with slots, which the screen
    // fills in the format of the person using the application.
    assert!(sales.contains("ui.message(Level::Warning, Text::plain(\"Too much\"));"), "{sales}");
    assert!(
        sales.contains("ui.message(Level::Info, Text::new(\"Approved {order.number}\").with(\"order.number\", Slot::text(w.order.number.clone())));"),
        "{sales}"
    );
    assert!(sales.contains("ui.set_error(\"number_input\", Text::plain(\"Check the number\"));"), "{sales}");
    assert!(sales.contains("ui.set_enabled(\"approve_button\", false);"), "{sales}");
    assert!(sales.contains("approve(db, &mut w.order)?;"), "{sales}");
    assert!(sales.contains("ui.open(\"OrderForm\", vec![(\"order\", to_value(&ui.selected(\"grid\", &w.orders)?)?)]);"), "{sales}");
    assert!(sales.contains("ui.close = true;"), "{sales}");

    let main = file(&files, "src/main.rs");
    assert!(main.contains("(\"OrderForm\", \"approve_clicked\") => {"), "{main}");
    assert!(main.contains("const MAIN_WINDOW: &str = \"OrderForm\";"), "{main}");
    assert!(main.contains("const DATABASES: &[(&str, Dialect, &str)] = &[(\"main\", Dialect::Sqlite, \"data/shop.db\")];"), "{main}");
    assert!(file(&files, "src/model.json").contains("\"OrderForm\""));
}

#[test]
fn handlers_set_errors_and_readonly_on_inputs() {
    let files = files();
    let sales = file(&files, "src/sales.rs");
    assert!(
        sales.contains("ui.set_error(\"number_input\", Text::plain(\"Check the number\"));"),
        "{sales}"
    );
    assert!(
        sales.contains("ui.set_readonly(\"number_input\", !ui.readonly(\"number_input\", true));"),
        "{sales}"
    );
    let main = file(&files, "src/main.rs");
    assert!(main.contains("(\"OrderForm\", \"order_submitted\") => {"), "{main}");
}

#[test]
fn window_events_and_reload_reach_the_backend() {
    let files = files();
    let main = file(&files, "src/main.rs");
    assert!(main.contains("(\"OrderForm\", \"return\") => Some(\"window_returned\"),"), "{main}");
    assert!(main.contains("fn close_window("), "{main}");
    assert!(main.contains("on_event(db, state, \"open\")"), "{main}");
    let sales = file(&files, "src/sales.rs");
    assert!(sales.contains("    w.orders = db.all::<Order>()?;\n"), "{sales}");
    assert!(sales.contains("    if w.order.id != 0 {\n        w.order = db.get::<Order>(w.order.id)?;\n    }\n"), "{sales}");
}

#[test]
fn queries_become_database_queries() {
    let files = files();
    let sales = file(&files, "src/sales.rs");
    assert!(
        sales.contains(
            "return Ok(db.find::<Order>(&Query::new().filter(Cond::number(\"total\", Op::GtEq, sql::decimal(limit)).and(Cond::one_of(\"status\", vec![sql::name(&OrderStatus::Pending), sql::name(&OrderStatus::Approved)]))).order_number(\"total\", true).order(\"number\", false))?);"
        ),
        "{sales}"
    );
    assert!(
        sales.contains(
            "w.orders = db.find::<Order>(&Query::new().filter(Cond::contains(\"number\", &ui.text(\"number_filter\")).and(Cond::compare(\"customer\", Op::Eq, sql::reference(w.order.customer)).negated())).order(\"number\", false))?;"
        ),
        "{sales}"
    );
    assert!(sales.contains("if contains(&w.order.number, &String::from(\"ORD\")) {"), "{sales}");
    assert!(sales.contains("ui.set_text(\"number_filter\", String::from(\"\"));"), "{sales}");
    // A handler runs another handler of its window.
    assert!(sales.contains("    order_form__filter_changed(db, w, ui)?;\n"), "{sales}");
}

#[test]
fn every_runtime_source_file_is_copied() {
    let files = files();
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../clarity-runtime/src");
    for entry in std::fs::read_dir(dir).unwrap() {
        let name = entry.unwrap().file_name().into_string().unwrap();
        let path = format!("clarity-runtime/src/{name}");
        assert!(files.iter().any(|f| f.path == path), "{path} is not copied into generated projects");
    }
}

/// SALES as it was: a `limit`, an integer total, a note, no unique number.
const FIRST_STEP: &str = "migration \"First\"
    add entity Customer
        name: text(80)
        limit: money, default 100
        active: bool, default true
        since: date
    add entity Order
        number: text(12)
        status: OrderStatus
        customer: ref Customer
        total: integer
        note: text
";

const SECOND_STEP: &str = "migration \"Tidy up\"
    rename field Customer.limit to credit_limit
    change field Order.total: money
    remove field Order.note
    add unique Order.number
";

#[test]
fn every_migration_step_becomes_sql() {
    let project = clarity_spec::load_with_migrations(
        [("sales.clarity", SALES)],
        [("0001.clarity", FIRST_STEP), ("0002.clarity", SECOND_STEP)],
    );
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let files = generate(&project, &shop()).unwrap();
    let schema = file(&files, "src/schema.rs");
    assert!(schema.contains("        // 1: First\n        &[\n            r#\"CREATE TABLE \"customer\" ("), "{schema}");
    assert!(schema.contains("        // 2: Tidy up\n"), "{schema}");
    assert!(schema.contains("ALTER TABLE \"customer\" RENAME COLUMN \"limit\" TO \"credit_limit\";"), "{schema}");
    // SQLite changes a column's type by building the table again.
    assert!(schema.contains("CREATE TABLE \"order__new\" ("), "{schema}");
    assert!(schema.contains("COALESCE(CAST(\"total\" AS TEXT), '0')"), "{schema}");
    assert!(schema.contains("DROP TABLE \"order\";\"#,\n            r#\"ALTER TABLE \"order__new\" RENAME TO \"order\";"), "{schema}");
    assert!(
        schema.contains("CREATE UNIQUE INDEX \"order_number_unique\" ON \"order\" (\"number\") WHERE \"number\" != '';"),
        "{schema}"
    );
}

#[test]
fn changes_without_a_step_are_not_generated() {
    let project = clarity_spec::load_with_migrations([("sales.clarity", SALES)], [("0001.clarity", FIRST_STEP)]);
    let errors = generate(&project, &shop()).unwrap_err();
    assert!(errors[0].contains("not in a migration yet"), "{errors:?}");
    assert!(errors[0].contains("rename") || errors[0].contains("remove field Customer.limit"), "{errors:?}");
    assert!(errors[0].contains("clarity migrate"), "{errors:?}");
}

#[test]
fn each_data_source_gets_its_own_database() {
    use clarity_project::{DataSource, SourceKind};
    use clarity_spec::SourceRef;
    let erp = "module erp\nuse sales\nsource erp\n\nentity Invoice\n    number: text(20)\n    customer: ref Customer\n";
    let project = clarity_spec::load_in(
        [("sales.clarity", SALES), ("erp.clarity", erp)],
        std::iter::empty(),
        &[SourceRef::sql("main"), SourceRef::sql("erp")],
    );
    assert!(project.diagnostics.iter().all(|d| d.severity == clarity_spec::Severity::Warning), "{:#?}", project.diagnostics);
    let mut settings = shop();
    settings.datasources.push(DataSource { name: "erp".into(), kind: SourceKind::Sqlite, url: "data/erp.db".into(), auth: None, external: false, readonly: false });
    // Without steps yet, generating is refused for both.
    assert_eq!(generate(&project, &settings).unwrap_err().len(), 2);

    let project = clarity_spec::load_sources([("sales.clarity", SALES), ("erp.clarity", erp)]);
    let mut project = project;
    project.sources = vec![SourceRef::sql("main"), SourceRef::sql("erp")];
    let files = generate(&project, &settings).unwrap();
    let main = file(&files, "src/main.rs");
    assert!(main.contains("const DATABASES: &[(&str, Dialect, &str)] = &[(\"main\", Dialect::Sqlite, \"data/shop.db\"), (\"erp\", Dialect::Sqlite, \"data/erp.db\")];"), "{main}");
    let erp_rs = file(&files, "src/erp.rs");
    assert!(erp_rs.contains("const SOURCE: &'static str = \"erp\";"), "{erp_rs}");
    let schema = file(&files, "src/schema.rs");
    assert!(schema.contains("    (\"erp\", &[\n"), "{schema}");
    // The customer is in the other database, so no foreign key.
    assert!(schema.contains("\"customer\" INTEGER\n"), "{schema}");
}

#[test]
fn an_external_source_is_opened_but_never_migrated() {
    use clarity_project::{DataSource, SourceKind};
    use clarity_spec::SourceRef;
    let erp = "module erp\nuse sales\nsource erp\n\nentity Invoice\n    table \"tblInvoice\"\n    key \"InvoiceID\"\n    number: text(20), column \"InvNo\"\n    total: money\n\nfunction big(limit: money) -> list<Invoice>\n    return find i in Invoice where i.number contains \"INV\" and i.total > limit order by i.number\n";
    let mut project = load_sources([("sales.clarity", SALES), ("erp.clarity", erp)]);
    project.sources = vec![SourceRef::sql("main"), SourceRef::sql("erp").external()];
    let mut settings = shop();
    settings.datasources.push(DataSource {
        name: "erp".into(),
        kind: SourceKind::Sqlite,
        url: "legacy/erp.db".into(),
        auth: None,
        external: true,
        readonly: false,
    });
    let files = generate(&project, &settings).unwrap();
    let main = file(&files, "src/main.rs");
    assert!(main.contains("(\"erp\", Dialect::Sqlite, \"legacy/erp.db\")"), "{main}");
    let schema = file(&files, "src/schema.rs");
    assert!(!schema.contains("(\"erp\""), "{schema}");
    assert!(!schema.contains("tblInvoice"), "{schema}");
    let erp_rs = file(&files, "src/erp.rs");
    assert!(erp_rs.contains("const TABLE: &'static str = \"tblInvoice\";"), "{erp_rs}");
    assert!(erp_rs.contains("const KEY: &'static str = \"InvoiceID\";"), "{erp_rs}");
    assert!(erp_rs.contains("const COLUMNS: &'static [&'static str] = &[\"InvNo\", \"total\"];"), "{erp_rs}");
    assert!(erp_rs.contains("Cond::contains(\"InvNo\", &String::from(\"INV\"))"), "{erp_rs}");
    assert!(erp_rs.contains(".order(\"InvNo\", false)"), "{erp_rs}");
}

#[test]
fn a_taken_over_database_keeps_its_names() {
    use clarity_project::{DataSource, SourceKind};
    use clarity_spec::SourceRef;
    let start = "module start\npage Start: window\n    title \"Start\"\n";
    let erp = "module erp\nsource erp\n\nentity Invoice\n    table \"invoices\"\n    key \"InvoiceID\"\n    number: text(20), unique, column \"InvoiceNumber\"\n    total: money\n";
    let baseline = "migration \"Taken over\" baseline\n    add entity Invoice\n        table \"tblInvoice\"\n        key \"InvoiceID\"\n        number: text(20), unique, column \"InvNo\"\n        total: money\n";
    let remap = "migration \"Tidy names\"\n    map table Invoice to \"invoices\"\n    map column Invoice.number to \"InvoiceNumber\"\n";
    let project = clarity_spec::load_in(
        [("start.clarity", start), ("erp.clarity", erp)],
        [("migrations/erp/0001.clarity", baseline), ("migrations/erp/0002.clarity", remap)],
        &[SourceRef::sql("main"), SourceRef::sql("erp")],
    );
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let mut settings = shop();
    settings.datasources.push(DataSource { name: "erp".into(), kind: SourceKind::Sqlite, url: "data/erp.db".into(), auth: None, external: false, readonly: false });
    let files = generate(&project, &settings).unwrap();
    let schema = file(&files, "src/schema.rs");
    // The baseline creates only what is missing, under the old names.
    assert!(schema.contains("CREATE TABLE IF NOT EXISTS \"tblInvoice\" (\n    \"InvoiceID\" INTEGER PRIMARY KEY,\n    \"InvNo\" TEXT NOT NULL DEFAULT ''"), "{schema}");
    assert!(schema.contains("CREATE UNIQUE INDEX IF NOT EXISTS \"tblInvoice_InvNo_unique\""), "{schema}");
    // Then the renames, with the unique index following its column.
    assert!(schema.contains("ALTER TABLE \"tblInvoice\" RENAME TO \"invoices\";"), "{schema}");
    assert!(schema.contains("ALTER TABLE \"invoices\" RENAME COLUMN \"InvNo\" TO \"InvoiceNumber\";"), "{schema}");
    assert!(schema.contains("CREATE UNIQUE INDEX \"invoices_InvoiceNumber_unique\""), "{schema}");
}

#[test]
fn data_events_become_record_hooks() {
    let text = SALES.to_string()
        + "
code Order
    before save
        if order.total < 0
            refuse \"An order cannot have a negative total\"

    after delete
        refuse \"Deleted {order.number}\"
";
    let project = load_sources([("sales.clarity", text.as_str())]);
    assert!(!project.has_errors(), "{:#?}", project.diagnostics);
    let files = generate(&project, &shop()).unwrap();
    let sales = file(&files, "src/sales.rs");
    assert!(sales.contains("pub fn before_save_order(db: &Db, order: &mut Order) -> Result<()> {"), "{sales}");
    assert!(sales.contains("return Err(Error::Refused(Text::plain(\"An order cannot have a negative total\")));"), "{sales}");
    assert!(
        sales.contains("return Err(Error::Refused(Text::new(\"Deleted {order.number}\").with(\"order.number\", Slot::text(order.number.clone()))));"),
        "{sales}"
    );
    assert!(sales.contains("    const EVENTS: bool = true;\n"), "{sales}");
    assert!(sales.contains("    fn before_save(&mut self, db: &Db) -> Result<()> {\n        before_save_order(db, self)\n    }\n"), "{sales}");
    assert!(sales.contains("    fn after_delete(&mut self, db: &Db) -> Result<()> {\n        after_delete_order(db, self)\n    }\n"), "{sales}");
    // Only the entity with events gets hooks.
    assert_eq!(sales.matches("const EVENTS").count(), 1, "{sales}");
}

#[test]
fn the_renderer_gets_the_language_and_currency() {
    let mut project = load_sources([("sales.clarity", SALES)]);
    project.locale.language = Some("nl-NL".into());
    project.locale.currency = Some("EUR".into());
    let files = generate(&project, &shop()).unwrap();
    let model = file(&files, "src/model.json");
    assert!(model.contains("\"locale\":{\"language\":\"nl-NL\",\"currency\":\"EUR\"}"), "{model}");
}

#[test]
fn slots_carry_typed_values_with_their_format() {
    let text = SALES.to_string()
        + "
action check(order: Order, rate: decimal)
    if order.total > 1000
        refuse \"{order.number} of {order.total} is {order.status} at {rate}, for a customer since {order.customer.since}\"
";
    let project = load_sources([("sales.clarity", text.as_str())]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let files = generate(&project, &shop()).unwrap();
    let sales = file(&files, "src/sales.rs");
    for slot in [
        ".with(\"order.number\", Slot::text(order.number.clone()))",
        ".with(\"order.total\", Slot::money(order.total, None))",
        ".with(\"order.status\", Slot::name(&order.status, \"OrderStatus\"))",
        ".with(\"rate\", Slot::decimal(rate, None, None))",
        // The field's format travels with the value.
        ".with(\"order.customer.since\", Slot::date(db.load::<Customer>(order.customer)?.since, Some(\"long\")))",
    ] {
        assert!(sales.contains(slot), "{slot} in\n{sales}");
    }
}

#[test]
fn a_refusal_reaches_the_screen_with_its_slots() {
    let files = files();
    let main = file(&files, "src/main.rs");
    assert!(main.contains("type Answer<T> = std::result::Result<T, Value>;"), "{main}");
    assert!(main.contains("Error::Refused(text) => serde_json::to_value(text).unwrap_or_default(),"), "{main}");
    assert!(main.contains("fn dispatch_event(app: tauri::State<App>, state: WindowState, handler: String) -> Answer<Reply> {"), "{main}");
}

#[test]
fn the_application_carries_its_translations() {
    let mut project = load_sources([("sales.clarity", SALES)]);
    project.translations.insert("nl".into(), [("Order".to_string(), "Bestelling".to_string())].into());
    let files = generate(&project, &shop()).unwrap();
    let model = file(&files, "src/model.json");
    assert!(model.contains("\"translations\":{\"nl\":{\"Order\":\"Bestelling\"}}"), "{model}");
}

#[test]
fn the_application_carries_its_theme() {
    let text = SALES.to_string() + "\ntheme Shop\n    color accent \"#1f6f5c\"\n    style primary background accent\n";
    let mut project = load_sources([("sales.clarity", text.as_str())]);
    clarity_spec::themes::use_themes(&mut project, &["Shop"]);
    let files = generate(&project, &shop()).unwrap();
    let model = file(&files, "src/model.json");
    assert!(model.contains("\"themes\":[\"Shop\"]"), "{model}");
    assert!(model.contains("\"light\":\"#1f6f5c\""), "{model}");
}
