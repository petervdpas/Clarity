use clarity_gen::{OutFile, generate};
use clarity_project::{Settings, starter};
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
            "README.md",
            "build.rs",
            "icons/icon.png",
            "src/main.rs",
            "src/model.json",
            "src/modules.rs",
            "src/modules/sales.rs",
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
    // The runtime crate from crates.io, at the version this generator is made
    // with, with the features of the project's databases only.
    let dependency = format!("clarity-runtime = {{ version = \"{}\", default-features = false, features = [\"sqlite\"] }}", runtime_version());
    assert!(cargo.contains(&dependency), "{cargo}");
    // A project of its own, not in a workspace around it.
    assert!(cargo.contains("\n[workspace]\n"), "{cargo}");
    assert!(file(&files, "src/main.rs").contains("mod modules;"));
    assert!(file(&files, "src/modules.rs").contains("pub mod sales;"));
}

#[test]
fn is_deterministic() {
    assert_eq!(files(), files());
}

#[test]
fn entities_become_records_with_their_defaults() {
    let files = files();
    let sales = file(&files, "src/modules/sales.rs");
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
    let sales = file(&files, "src/modules/sales.rs");
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
fn refuses_a_project_with_errors() {
    let project = load_sources([("sales.clarity", "module sales\n\nentity A\n    b: Nope\n")]);
    let errors = generate(&project, &shop()).unwrap_err();
    assert!(errors[0].contains("fix the errors"), "{errors:?}");
}

#[test]
fn handlers_work_on_the_window_and_the_screen() {
    let files = files();
    let sales = file(&files, "src/modules/sales.rs");
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

/// Every desktop window carries the title of the Clarity window it shows,
/// as on Avalonia; the renderer sets it when the window changes.
#[test]
fn a_desktop_window_takes_the_title_of_its_window() {
    let files = files();
    let main = file(&files, "src/main.rs");
    assert!(main.contains("fn set_title(window: tauri::WebviewWindow, title: String) {\n    let _ = window.set_title(&title);\n}"), "{main}");
    assert!(main.contains("            set_title,\n"), "{main}");
    let runtime = include_str!("../../../app/src/runtime/Runtime.svelte");
    assert!(runtime.contains("invoke(\"set_title\", { title })"), "the renderer sets the title");
}

#[test]
fn handlers_set_errors_and_readonly_on_inputs() {
    let files = files();
    let sales = file(&files, "src/modules/sales.rs");
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
    let sales = file(&files, "src/modules/sales.rs");
    assert!(sales.contains("    w.orders = db.all::<Order>()?;\n"), "{sales}");
    assert!(sales.contains("    if w.order.id != 0 {\n        w.order = db.get::<Order>(w.order.id)?;\n    }\n"), "{sales}");
}

#[test]
fn queries_become_database_queries() {
    let files = files();
    let sales = file(&files, "src/modules/sales.rs");
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

/// The version of the runtime crate, as runtimes/rust/Cargo.toml names it.
fn runtime_version() -> &'static str {
    let manifest = include_str!("../../../runtimes/rust/Cargo.toml");
    let start = manifest.find("\nversion = \"").unwrap() + "\nversion = \"".len();
    &manifest[start..start + manifest[start..].find('"').unwrap()]
}

/// The program builds on the runtime crate and carries no copy of it.
#[test]
fn no_runtime_source_is_copied() {
    let copied: Vec<String> = files().into_iter().map(|f| f.path).filter(|p| p.starts_with("clarity-runtime/")).collect();
    assert!(copied.is_empty(), "{copied:?}");
}

/// While working on the runtime, `.cargo/config.toml` in the project patches
/// the crate with the runtime in this repository; Clarity keeps that folder
/// when it writes the project again.
#[test]
fn a_cargo_patch_survives_writing_the_project_again() {
    assert!(clarity_gen::build_output(clarity_project::Backend::Rust).contains(&".cargo"));
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
    let erp_rs = file(&files, "src/modules/erp.rs");
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
    let erp_rs = file(&files, "src/modules/erp.rs");
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
    let sales = file(&files, "src/modules/sales.rs");
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
    let sales = file(&files, "src/modules/sales.rs");
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

/// One desktop window (DESIGN.md, Page kinds): Tauri opens one, and the
/// renderer shows every Clarity window in it.
#[test]
fn the_application_has_one_desktop_window() {
    let files = files();
    let conf: serde_json::Value = serde_json::from_str(file(&files, "tauri.conf.json")).unwrap();
    assert_eq!(conf["app"]["windows"].as_array().map(Vec::len), Some(1));
    for f in files.iter().filter(|f| f.path.ends_with(".rs")) {
        assert!(!f.text.contains("WebviewWindowBuilder") && !f.text.contains("WindowBuilder"), "{} opens a window", f.path);
    }
}

/// A target marked `in new window` is the one way to a second desktop
/// window: only then does the application make one, through `open_apart`.
#[test]
fn only_a_target_in_new_window_makes_a_second_desktop_window() {
    let apart = SALES.replace(
        "    button \"Approve\" name approve_button at 1,6 span 2 on click: approve_clicked",
        "    button \"Approve\" name approve_button at 1,6 span 2 on click: approve_clicked\n    button \"Again\" at 3,6 span 2 on click -> OrderForm in new window",
    );
    assert_ne!(apart, SALES);
    let project = load_sources([("sales.clarity", apart.as_str())]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let apart_files = generate(&project, &shop()).unwrap();
    let main = file(&apart_files, "src/main.rs");
    assert!(main.contains("fn open_apart("), "{main}");
    assert!(main.contains("tauri::WebviewWindowBuilder::new("), "{main}");
    assert!(main.contains("fn close_apart("), "{main}");
    assert!(main.contains("open_apart,\n            apart_start,\n            close_apart,"), "{main}");
    assert!(!file(&files(), "src/main.rs").contains("open_apart"));
}

/// A window opens at the size it says: the desktop window Tauri opens takes
/// the main window's, and a desktop window of its own takes the size the
/// renderer asks for (the window it shows first), else the standard one.
#[test]
fn a_window_opens_at_its_size() {
    let conf: serde_json::Value = serde_json::from_str(file(&files(), "tauri.conf.json")).unwrap();
    assert_eq!((conf["app"]["windows"][0]["width"].as_u64(), conf["app"]["windows"][0]["height"].as_u64()), (Some(1100), Some(760)));

    let sized = SALES.replace("    title \"Order\"\n", "    title \"Order\"\n    size 640 x 480\n").replace(
        "    button \"Approve\" name approve_button at 1,6 span 2 on click: approve_clicked",
        "    button \"Approve\" name approve_button at 1,6 span 2 on click: approve_clicked\n    button \"Again\" at 3,6 span 2 on click -> OrderForm in new window",
    );
    let project = load_sources([("sales.clarity", sized.as_str())]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let files = generate(&project, &shop()).unwrap();
    let conf: serde_json::Value = serde_json::from_str(file(&files, "tauri.conf.json")).unwrap();
    assert_eq!((conf["app"]["windows"][0]["width"].as_u64(), conf["app"]["windows"][0]["height"].as_u64()), (Some(640), Some(480)));
    let main = file(&files, "src/main.rs");
    assert!(main.contains("width: Option<f64>, height: Option<f64>"), "{main}");
    assert!(main.contains(".inner_size(width.unwrap_or(1100.0), height.unwrap_or(760.0))"), "{main}");
}

/// A record's id, which code can read, is the record's `id`.
#[test]
fn a_record_s_id_reads_as_its_id() {
    let source = format!("{SALES}\nfunction same(order: Order, other: Order) -> bool\n    return order.id == other.id\n");
    let files = generate(&load_sources([("sales.clarity", source.as_str())]), &shop()).unwrap_or_else(|e| panic!("{e:#?}"));
    let sales = file(&files, "src/modules/sales.rs");
    assert!(sales.contains("Ok((order.id == other.id))"), "{sales}");
}

/// The selected row goes on into paths and calls, as any record does.
#[test]
fn the_selected_row_is_a_record_in_any_expression() {
    let source = SALES.replace(
        "        open OrderForm with order = grid.selected\n",
        "        approve(grid.selected)\n        message \"{grid.selected.number} of {grid.selected.customer.name} is approved\"\n        open OrderForm with order = grid.selected\n",
    );
    let files = generate(&load_sources([("sales.clarity", source.as_str())]), &shop()).unwrap_or_else(|e| panic!("{e:#?}"));
    let sales = file(&files, "src/modules/sales.rs");
    assert!(sales.contains("approve(db, &mut ui.selected(\"grid\", &w.orders)?)?;"), "{sales}");
    assert!(sales.contains("ui.selected(\"grid\", &w.orders)?.number"), "{sales}");
}

/// A record given to a reference field is stored as its id.
#[test]
fn a_reference_field_takes_a_record() {
    let source = format!("{SALES}\naction assign(order: Order, customer: Customer)\n    effects db.write\n    order.customer = customer\n    save order\n");
    let files = generate(&load_sources([("sales.clarity", source.as_str())]), &shop()).unwrap_or_else(|e| panic!("{e:#?}"));
    let sales = file(&files, "src/modules/sales.rs");
    assert!(sales.contains("order.customer = Some(customer.id);"), "{sales}");
}

/// `none`, the empty value of a date or reference (X2).
#[test]
fn none_is_an_empty_option() {
    let source = format!("{SALES}\nfunction unplaced(order: Order) -> bool\n    return order.customer == none or order.customer.since != none\n\naction clear(order: Order, customer: Customer)\n    effects db.write\n    customer.since = none\n    order.customer = none\n    save order\n\nfunction without_customer() -> list<Order>\n    return find o in Order where o.customer == none\n");
    let files = generate(&load_sources([("sales.clarity", source.as_str())]), &shop()).unwrap_or_else(|e| panic!("{e:#?}"));
    let sales = file(&files, "src/modules/sales.rs");
    for line in ["order.customer.is_none()", "!db.load::<Customer>(order.customer)?.since.is_none()", "customer.since = None;", "order.customer = None;", "Cond::is_none(\"customer\")"] {
        assert!(sales.contains(line), "{line} not in {sales}");
    }
}

/// `today()` and `now()` read the runtime's clock (X2).
#[test]
fn today_reads_the_clock() {
    let source = format!("{SALES}\naction stamp(customer: Customer)\n    effects db.write\n    if customer.since == none or customer.since > today()\n        customer.since = today()\n    save customer\n");
    let files = generate(&load_sources([("sales.clarity", source.as_str())]), &shop()).unwrap_or_else(|e| panic!("{e:#?}"));
    let sales = file(&files, "src/modules/sales.rs");
    assert!(sales.contains("customer.since = Some(clarity_runtime::today());"), "{sales}");
    assert!(sales.contains("(customer.since > Some(clarity_runtime::today()))"), "{sales}");
}

/// An unbound checkbox holds its own state, sent with the window (X2).
#[test]
fn an_unbound_checkbox_is_checked_in_the_window_state() {
    let source = SALES
        .replace("    textbox name number_filter at 1,7 span 4 on change: filter_changed\n", "    textbox name number_filter at 1,7 span 4 on change: filter_changed\n    checkbox \"Only mine\" name mine at 5,7 span 3\n")
        .replace("            number_filter.text = \"\"\n", "            number_filter.text = \"\"\n        if mine.checked\n            mine.checked = false\n");
    let project = load_sources([("sales.clarity", source.as_str())]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let files = generate(&project, &shop()).unwrap_or_else(|e| panic!("{e:#?}"));
    let sales = file(&files, "src/modules/sales.rs");
    assert!(sales.contains("if ui.checked(\"mine\") {"), "{sales}");
    assert!(sales.contains("ui.set_checked(\"mine\", false);"), "{sales}");
}

const MUSIC: &str = "module music\n\nentity Track\n    title: text(80)\n    length: duration, min 0, default 180\n\nfunction album_length(tracks: list<Track>) -> duration\n    let total: duration = 0\n    for track in tracks\n        total = total + track.length\n    return total\n\ncode Track\n    before save\n        if track.length > 7200\n            refuse \"{track.title} lasts {track.length}\"\n\npage Tracks: window\n    data track: Track\n    number at 1,1 span 3 bind track.length\n    text at 4,1 span 3 bind track.length\n";

/// A duration is whole seconds; its slot shows it as 3:45 (X2).
#[test]
fn a_duration_is_seconds_with_its_own_slot() {
    let project = load_sources([("music.clarity", MUSIC)]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let files = generate(&project, &shop()).unwrap_or_else(|e| panic!("{e:#?}"));
    let music = file(&files, "src/modules/music.rs");
    assert!(music.contains("pub length: i64,"), "{music}");
    assert!(music.contains("            length: 180i64,"), "{music}");
    assert!(music.contains(".with(\"track.length\", Slot::duration(track.length))"), "{music}");
}

/// X2: a query through a reference: the condition and the order read the
/// referred record in a subquery.
#[test]
fn a_query_goes_through_a_reference() {
    let source = format!(
        "{SALES}\nfunction of_customers(name: text) -> list<Order>\n    return find o in Order where o.customer.name contains name and o.customer.credit_limit > 10 order by o.customer.name desc, o.customer.credit_limit\n"
    );
    let files = generate(&load_sources([("sales.clarity", source.as_str())]), &shop()).unwrap_or_else(|e| panic!("{e:#?}"));
    let sales = file(&files, "src/modules/sales.rs");
    for expected in [
        ".filter(Cond::through(\"customer\", \"customer\", \"id\", Cond::contains(\"name\", &name)).and(Cond::through(\"customer\", \"customer\", \"id\", Cond::number(\"credit_limit\", Op::Gt, ",
        ".order_through(&[Hop::new(\"customer\", \"customer\", \"id\")], \"name\", true, false).order_through(&[Hop::new(\"customer\", \"customer\", \"id\")], \"credit_limit\", false, true))?",
    ] {
        assert!(sales.contains(expected), "{expected}\nin\n{sales}");
    }
}

/// A loop that changes and saves its records works on the records in the
/// list, as on the other stacks.
#[test]
fn a_loop_changes_and_saves_its_records() {
    let source = format!("{SALES}\naction approve_all(orders: list<Order>)\n    effects db.write\n    let found = orders\n    for order in found\n        order.status = Approved\n        save order\n    for order in found\n        let n = order.number\n");
    let files = generate(&load_sources([("sales.clarity", source.as_str())]), &shop()).unwrap_or_else(|e| panic!("{e:#?}"));
    let sales = file(&files, "src/modules/sales.rs");
    assert!(sales.contains("for order in found.iter_mut() {"), "{sales}");
    assert!(sales.contains("for order in found.iter() {"), "{sales}");
}

/// Rows read again, or replaced by a query, are other records: a table or
/// list showing them loses its selected row, as on every stack.
#[test]
fn rows_read_again_lose_their_selected_row() {
    let files = files();
    let sales = file(&files, "src/modules/sales.rs");
    assert!(sales.contains("    w.orders = db.all::<Order>()?;\n    ui.deselect(\"grid\");\n"), "{sales}");
    assert!(sales.contains(".order(\"number\", false))?;\n    ui.deselect(\"grid\");\n"), "{sales}");
    // A single record read again keeps what is chosen.
    assert_eq!(sales.matches("ui.deselect(").count(), 2, "{sales}");
}

/// The record a table shows is saved and deleted as it is, as every stack
/// does: `save grid.selected` after a cell was edited (LANGUAGE.md, Editing
/// in a table).
#[test]
fn the_selected_row_is_saved_and_deleted_as_it_is() {
    let source = SALES.replace(
        "    handler grid_opened\n",
        "    handler grid_changed\n        save grid.selected\n        delete grid.selected\n\n    handler grid_opened\n",
    );
    let project = load_sources([("sales.clarity", source.as_str())]);
    assert!(project.diagnostics.iter().all(|d| d.severity != clarity_spec::Severity::Error), "{:#?}", project.diagnostics);
    let files = generate(&project, &shop()).unwrap_or_else(|e| panic!("{e:#?}"));
    let sales = file(&files, "src/modules/sales.rs");
    assert!(sales.contains("    db.save(&mut ui.selected(\"grid\", &w.orders)?)?;\n"), "{sales}");
    assert!(sales.contains("    db.delete(&mut ui.selected(\"grid\", &w.orders)?)?;\n"), "{sales}");
}

/// A click passes values: each written call is dispatched with its values
/// compiled in, and a handler may call another with values.
#[test]
fn a_click_passes_values_to_its_handler() {
    let source = SALES
        .replace(
            "    button \"Approve\" name approve_button at 1,6 span 2 on click: approve_clicked",
            "    button \"Approve\" name approve_button at 1,6 span 2 on click: approve_clicked\n    button \"7\" name d7 at 3,6 span 1 on click: digit(7)\n    button \"Half\" name half at 4,6 span 1 on click: scale(0.5, \"half\", true)\n    button \"Ok\" name ok at 5,6 span 1 on click: mark(Approved)",
        )
        .replace(
            "    handler order_submitted\n",
            "    handler digit(d: integer)\n        number_filter.text = \"\"\n\n    handler scale(by: decimal, word: text, loud: bool)\n        digit(3)\n\n    handler mark(status: OrderStatus)\n        digit(1)\n\n    handler order_submitted\n",
        );
    let project = load_sources([("sales.clarity", source.as_str())]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let files = generate(&project, &shop()).unwrap_or_else(|e| panic!("{e:#?}"));
    let sales = file(&files, "src/modules/sales.rs");
    assert!(sales.contains("pub fn order_form__digit(db: &Db, w: &mut OrderFormWindow, ui: &mut Ui, d: i64) -> Result<()> {"), "{sales}");
    assert!(sales.contains("pub fn order_form__scale(db: &Db, w: &mut OrderFormWindow, ui: &mut Ui, by: Decimal, word: String, loud: bool) -> Result<()> {"), "{sales}");
    assert!(sales.contains("    order_form__digit(db, w, ui, 3i64)?;\n"), "{sales}");
    let main = file(&files, "src/main.rs");
    assert!(main.contains("(\"OrderForm\", \"digit(7)\") => {"), "{main}");
    assert!(main.contains("sales::order_form__digit(db, &mut w, &mut ui, 7i64)?;"), "{main}");
    assert!(main.contains("(\"OrderForm\", \"scale(0.5, \\\"half\\\", true)\") => {"), "{main}");
    // main.rs reaches the module's types by their paths.
    assert!(main.contains("sales::order_form__scale(db, &mut w, &mut ui, clarity_runtime::Decimal::new(5, 1), String::from(\"half\"), true)?;"), "{main}");
    assert!(main.contains("sales::order_form__mark(db, &mut w, &mut ui, modules::sales::OrderStatus::Approved)?;"), "{main}");
    // A handler that takes values is only called with them.
    assert!(!main.contains("(\"OrderForm\", \"digit\")"), "{main}");
}

/// A window with plain values besides records, as each stack's tests use it.
const CALC: &str = "
page Calc: window
    title \"Calc\"
    data shown: decimal
    data pending: OrderStatus
    data fresh: bool, default true
    data word: text, default \"hi\"
    data count: integer, default 2
    data rate: decimal, default 0.5
    text name display at 1,1 span 4 bind shown
    button \"7\" name d7 at 1,2 span 1 on click: digit(7)

code Calc
    handler digit(d: integer)
        if fresh
            shown = 0
            fresh = false
        shown = shown * 10 + d
        pending = Approved
        count = count + 1
        word = \"x\"
";

/// A window holds plain values: fields of its data, at their default or
/// empty when it opens, read and changed by its handlers.
#[test]
fn a_window_holds_plain_values() {
    let source = format!("{SALES}{CALC}");
    let project = load_sources([("sales.clarity", source.as_str())]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let files = generate(&project, &shop()).unwrap_or_else(|e| panic!("{e:#?}"));
    let sales = file(&files, "src/modules/sales.rs");
    for field in ["pub shown: Decimal,", "pub pending: OrderStatus,", "pub fresh: bool,", "pub word: String,", "pub count: i64,", "pub rate: Decimal,"] {
        assert!(sales.contains(field), "{field} in {sales}");
    }
    for load in ["shown: Default::default(),", "pending: Default::default(),", "fresh: true,", "word: String::from(\"hi\"),", "count: 2i64,", "rate: Decimal::new(5, 1),"] {
        assert!(sales.contains(load), "{load} in {sales}");
    }
    assert!(sales.contains("w.fresh = false;"), "{sales}");
    assert!(sales.contains("w.pending = OrderStatus::Approved;"), "{sales}");
}
