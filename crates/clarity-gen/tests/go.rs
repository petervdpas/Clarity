//! The Go backend (G3a): the data dictionary, functions, actions and data
//! events as Go, over the Go runtime copied into the module.

use clarity_gen::{OutFile, generate};
use clarity_project::{Backend, Settings, SourceKind, Ui, starter};
use clarity_spec::load_sources;

const SALES: &str = "module sales

enum OrderStatus
    Pending
    Approved label \"Approved by sales\"

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
    event: text(20)

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

function described(order: Order) -> text
    let all = find o in Order where o.customer == order.customer
    let shown = \"Order \" + order.number + \" is \" + order.status
    for other in all
        if other.number contains \"X\" or count(all) > 3
            shown = shown + \"!\"
    return shown

action approve(order: Order)
    require order.status == Pending
    require within_credit(order)
    effects db.write
    order.status = Approved
    save order

code Order
    before save
        if order.status == Approved and not within_credit(order)
            refuse \"Order {order.number} is over its customer's credit limit\"

    after delete
        return

page OrderForm: window
    title \"Order\"
    data order: Order
";

fn settings() -> Settings {
    let mut settings = starter("Shop", "shop");
    settings.stack.backend = Backend::Go;
    settings.stack.ui = Ui::Web;
    settings
}

fn files() -> Vec<OutFile> {
    let project = load_sources([("sales.clarity", SALES)]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    generate(&project, &settings()).unwrap_or_else(|e| panic!("{e:#?}"))
}

fn file<'a>(files: &'a [OutFile], path: &str) -> &'a str {
    &files.iter().find(|f| f.path == path).unwrap_or_else(|| panic!("no {path} in {:?}", files.iter().map(|f| &f.path).collect::<Vec<_>>())).text
}

/// Asserts `text` holds `expected`, lines and all.
fn has(text: &str, expected: &str) {
    assert!(text.contains(expected), "expected\n{expected}\nin\n{text}");
}

/// The runtime module's path and version, as runtimes/go names them.
const RUNTIME: &str = "github.com/petervdpas/Clarity/runtimes/go";

fn runtime_version() -> &'static str {
    let source = include_str!("../../../runtimes/go/clarity/version.go");
    let start = source.find("const Version = \"").unwrap() + "const Version = \"".len();
    &source[start..start + source[start..].find('"').unwrap()]
}

#[test]
fn writes_a_go_module_on_the_runtime_module() {
    let files = files();
    let module = file(&files, "go.mod");
    has(module, "module shop\n");
    // The runtime, at the version this generator is made with.
    has(module, &format!("\t{RUNTIME} {}\n", runtime_version()));
    // The runtime's requirements, at the versions it is tested with.
    let runtime = include_str!("../../../runtimes/go/go.mod");
    let sqlite = runtime.lines().find(|l| l.contains("modernc.org/sqlite")).unwrap().trim();
    has(module, sqlite);
    // The sums of the runtime's modules this application needs, and those of
    // the runtime module itself as recorded when it was published.
    let sums = include_str!("../../../runtimes/go/go.sum");
    let published = include_str!("../src/go/runtime.sum");
    assert!(file(&files, "go.sum").lines().all(|l| sums.lines().chain(published.lines()).any(|s| s == l)));
    has(file(&files, "go.sum"), &format!("{RUNTIME} {} h1:", runtime_version()));
    has(file(&files, "go.sum"), &format!("{RUNTIME} {}/go.mod h1:", runtime_version()));
    has(file(&files, "go.sum"), "modernc.org/sqlite v1.54.0 h1:");
    // The code imports the runtime's packages from the module.
    has(file(&files, "sales_module.go"), &format!("\t\"{RUNTIME}/clarity\"\n"));
    assert!(files.iter().any(|f| f.path == "README.md"));
}

/// The program builds on the runtime module and carries no copy of it.
#[test]
fn no_runtime_source_is_copied() {
    let files = files();
    let copied: Vec<&str> = files.iter().map(|f| f.path.as_str()).filter(|p| p.starts_with("clarity/") || p.starts_with("shell/") || p.ends_with("_test.go")).collect();
    assert!(copied.is_empty(), "{copied:?}");
}

/// While working on the runtime, a go.work next to the module builds it on
/// the runtime in this repository; Clarity keeps that file when it writes
/// the module again.
#[test]
fn a_workspace_file_survives_writing_the_module_again() {
    let kept = clarity_gen::build_output(clarity_project::Backend::Go);
    assert!(kept.contains(&"go.work") && kept.contains(&"go.work.sum"), "{kept:?}");
}

/// A module imports the package of each server database its data sources
/// are, which turns the driver on, with the modules it needs, and no others
/// (as the C# output picks packages).
#[test]
fn a_module_gets_the_drivers_of_its_databases_only() {
    let project = load_sources([("sales.clarity", SALES)]);
    let drivers = [
        (SourceKind::Postgres, "postgresql", "github.com/jackc/pgx/v5 "),
        (SourceKind::Mysql, "mysql", "github.com/go-sql-driver/mysql "),
        (SourceKind::Mssql, "sqlserver", "github.com/microsoft/go-mssqldb "),
    ];
    let import = |package: &str| format!("\t_ \"{RUNTIME}/{package}\"\n");
    let plain = generate(&project, &settings()).unwrap();
    for (_, package, module) in drivers {
        assert!(!file(&plain, "main.go").contains(&import(package)), "{package}");
        assert!(!file(&plain, "go.mod").contains(module), "{module}");
        assert!(!file(&plain, "go.sum").contains(module), "{module}");
    }
    // Shared with the SQLite driver and Wails, so it stays.
    has(file(&plain, "go.mod"), "golang.org/x/crypto ");

    for (kind, package, module) in drivers {
        let mut settings = settings();
        settings.datasources[0].kind = kind;
        settings.datasources[0].url = "env:SHOP_DATABASE".into();
        let files = generate(&project, &settings).unwrap();
        has(file(&files, "main.go"), &import(package));
        has(file(&files, "go.mod"), module);
        has(file(&files, "go.sum"), module);
        for (_, other, other_module) in drivers.iter().filter(|d| d.1 != package) {
            assert!(!file(&files, "main.go").contains(&import(other)), "{other}");
            assert!(!file(&files, "go.mod").contains(other_module), "{other_module}");
        }
    }
    // pgx's own requirements come along with it.
    let mut settings = settings();
    settings.datasources[0].kind = SourceKind::Postgres;
    has(file(&generate(&project, &settings).unwrap(), "go.mod"), "github.com/jackc/puddle/v2 ");
}

#[test]
fn is_deterministic() {
    assert_eq!(files(), files());
}

#[test]
fn enums_and_entities_become_go_types() {
    let files = files();
    let sales = file(&files, "sales_module.go");
    has(sales, "package main\n");
    has(sales, "type OrderStatus string\n");
    has(sales, "\tOrderStatusApproved OrderStatus = \"Approved\"\n");
    has(sales, "func (v OrderStatus) Label() string {");
    has(sales, "\tcase OrderStatusApproved:\n\t\treturn \"Approved by sales\"\n");
    has(sales, "type Customer struct {\n\tID          int64           `json:\"id\"`\n\tName        string          `json:\"name\"`\n\tCreditLimit decimal.Decimal `json:\"credit_limit\"`\n\tActive      bool            `json:\"active\"`\n\tSince       *clarity.Date   `json:\"since\"`\n}");
    // A new record has its defaults.
    has(sales, "func NewCustomer() Customer {\n\treturn Customer{CreditLimit: decimal.NewFromInt(100), Active: true}\n}");
    has(sales, "func (*Customer) Table() string {\n\treturn \"customer\"\n}");
    has(sales, "func (r *Order) Values() []any {\n\treturn []any{r.Number, string(r.Status), r.Customer, r.Total, r.Event}\n}");
    has(sales, "\tr.Status = OrderStatus(read.Name(1, \"status\", \"Pending\", \"Approved\"))\n");
    has(sales, "\tr.Customer = read.Reference(2, \"customer\")\n");
}

#[test]
fn the_schema_comes_from_the_shared_migration_steps() {
    let files = files();
    let schema = file(&files, "schema.go");
    has(schema, "var Sources = []SourceSteps{");
    has(schema, "CREATE TABLE \"customer\"");
}

#[test]
fn functions_and_actions_return_their_errors() {
    let files = files();
    let sales = file(&files, "sales_module.go");
    // A reference read in an expression is loaded before it, and its error returned.
    has(
        sales,
        "func WithinCredit(db *clarity.Db, order *Order) (bool, error) {
\tv1, err := clarity.Ref[Customer](db, order.Customer)
\tif err != nil {
\t\treturn false, err
\t}
\treturn order.Total.LessThanOrEqual(v1.CreditLimit), nil
}",
    );
    has(sales, "func Approve(db *clarity.Db, order *Order) error {");
    has(sales, "\tv1, err := WithinCredit(db, order)\n\tif err != nil {\n\t\treturn err\n\t}\n\tif err := clarity.Require(v1, \"within_credit(order)\"); err != nil {\n\t\treturn err\n\t}");
    has(sales, "\torder.Status = OrderStatusApproved\n\tif err := clarity.Save(db, order); err != nil {\n\t\treturn err\n\t}\n\treturn nil\n}");
    // An ensure checks the result of the body.
    has(sales, "func Doubled(db *clarity.Db, n int64) (decimal.Decimal, error) {");
    has(sales, "\tresult, err := body()\n\tif err != nil {\n\t\treturn decimal.Zero, err\n\t}\n\tif err := clarity.Ensure(result.GreaterThanOrEqual(decimal.Zero), \"result >= 0\"); err != nil {");
}

#[test]
fn queries_text_and_loops() {
    let files = files();
    let sales = file(&files, "sales_module.go");
    has(sales, "clarity.Find[Order](db, clarity.Query{}.Filter(clarity.And(clarity.Number(\"total\", clarity.GtEq, limit), clarity.OneOf(\"status\", []any{string(OrderStatusPending), string(OrderStatusApproved)}))).OrderNumber(\"total\", true).Order(\"number\", false))");
    has(sales, "for i := range all {\n\t\tother := &all[i]\n");
    has(sales, "clarity.TextContains(other.Number, \"X\")");
    has(sales, "shown := ((\"Order \" + order.Number) + \" is \") + order.Status.Label()");
}

/// `and` and `or` stop at the first side that decides: a call on the right
/// that can fail runs only when the left does not decide.
#[test]
fn and_or_keep_short_circuiting() {
    let source = format!("{SALES}\nfunction both(order: Order) -> bool\n    return order.total > 10 and within_credit(order)\n");
    let files = generate(&load_sources([("sales.clarity", source.as_str())]), &settings()).unwrap_or_else(|e| panic!("{e:#?}"));
    has(
        file(&files, "sales_module.go"),
        "func Both(db *clarity.Db, order *Order) (bool, error) {
\tv1 := order.Total.GreaterThan(decimal.NewFromInt(10))
\tif v1 {
\t\tv2, err := WithinCredit(db, order)
\t\tif err != nil {
\t\t\treturn false, err
\t\t}
\t\tv1 = v2
\t}
\treturn v1, nil
}",
    );
}

#[test]
fn data_events_run_from_the_record() {
    let files = files();
    let sales = file(&files, "sales_module.go");
    has(sales, "func (r *Order) BeforeSave(db *clarity.Db) error {\n\treturn beforeSaveOrder(db, r)\n}");
    has(sales, "return clarity.Refuse(clarity.NewText(\"Order {order.number} is over its customer's credit limit\").With(\"order.number\", clarity.SlotText(order.Number)))");
}

#[test]
fn the_program_opens_and_migrates_every_database() {
    let files = files();
    let main = file(&files, "main.go");
    has(main, "var databases = []clarity.Source{{Name: \"main\", Dialect: clarity.Sqlite, At: \"data/shop.db\"}}");
    has(main, "\"Shop: the databases are ready.\"");
}

const WINDOWS: &str = "module shop

enum OrderStatus
    Pending
    Approved label \"Approved by sales\"

entity Customer
    name: text(80), required, help \"The name on invoices\"

entity Order
    number: text(12), required
    status: OrderStatus
    customer: ref Customer
    total: money
    ordered_on: date
    paid: bool
    discount: decimal(6, 4), format percent

function within_credit(order: Order) -> bool
    return order.total < 1000

app Shop
    title \"Shop\"
    main OrderForm
    menu
        item \"File\"
            item \"Orders\" key \"Ctrl+O\" on click -> OrderForm
            item \"Another order\" on click -> OrderForm in new window
            separator
            item \"Quit\" on click -> quit
            item \"Language\" on click -> language
            item \"Theme\" on click -> theme
    toolbar
        button \"Look\" on click -> theme
    footer
        status \"Ready\"

page OrderForm: window
    title \"Order\"
    data order: Order
    data orders: list<Order>
    on open: window_opened
    on return: window_returned
    label \"Number\" at 1,1 span 2 for number_input
    textbox name number_input at 3,1 span 4 bind order.number placeholder \"ORD-1\"
    help at 3,2 span 4 for number_input
    number at 1,3 span 3 bind order.total
    checkbox \"Paid\" at 4,3 span 3 bind order.paid
    switch \"Rush\" at 7,3 span 3 bind order.paid
    dropdown at 1,4 span 4 bind order.status
    dropdown name customer_pick at 5,4 span 4 bind order.customer
    date at 9,4 span 3 bind order.ordered_on
    text at 1,5 span 4 bind order.status
    table name grid at 1,6 span 12 rows 4 bind orders columns number, status, total on select: grid_selected on open: grid_opened
    textbox name number_filter at 1,10 span 4 on change: filter_changed
    button \"Approve\" name approve_button at 9,10 span 2 default on click: approve_clicked
    button \"New\" at 11,10 span 2 on click -> OrderForm
    tabs at 1,11 span 12 rows 2
        tab \"Notes\"
            memo at 1,1 span 12 bind order.number

code OrderForm
    handler window_opened
        number_input.readonly = order.paid

    handler window_returned
        reload orders
        reload order

    handler approve_clicked
        if not within_credit(order)
            message warning \"Order {order.number} is too large\"
            approve_button.enabled = false
            return
        order.status = Approved
        save order
        number_input.error = \"\"
        message \"Approved\"
        filter_changed()

    handler filter_changed
        orders = find o in Order where o.number contains number_filter.text order by o.number

    handler grid_selected
        order = grid.selected

    handler grid_opened
        open OrderForm with order = grid.selected
        close
";

fn windows() -> Vec<OutFile> {
    let project = load_sources([("shop.clarity", WINDOWS)]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    generate(&project, &settings()).unwrap_or_else(|e| panic!("{e:#?}"))
}

/// The renderer is the one every web stack ships; the Go shell serves it
/// with Wails (G3b).
#[test]
fn a_wails_application_with_the_web_renderer() {
    let files = windows();
    for path in ["ui/index.html", "ui/runtime.js", "ui/runtime.css", "ui/themes/clarity.css", "model.json"] {
        assert!(files.iter().any(|f| f.path == path), "no {path}");
    }
    has(file(&files, "go.mod"), "github.com/wailsapp/wails/v3 v3.0.0-alpha.84");
    let main = file(&files, "main.go");
    has(main, "//go:embed ui\nvar renderer embed.FS");
    has(main, "//go:embed model.json\nvar model []byte");
    has(main, "\tassets, err := fs.Sub(renderer, \"ui\")");
    // The renderer calls the service the program binds: main.Clarity.Invoke.
    has(main, "type Clarity struct {\n\t*shell.Service\n}");
    has(main, "shell.Run(shell.Options{Name: \"shop\", Title: \"Shop\", Main: \"OrderForm\", Model: model, Assets: assets, Db: db, Windows: windows{}}, bind)");
    has(main, "func bind(s *shell.Service) application.Service {\n\treturn application.NewService(&Clarity{s})\n}");
}

#[test]
fn a_window_is_a_struct_of_its_data() {
    let files = windows();
    let shop = file(&files, "shop_module.go");
    has(shop, "type OrderFormWindow struct {\n\tOrder  *Order  `json:\"order\"`\n\tOrders []Order `json:\"orders\"`\n}");
    has(
        shop,
        "func loadOrderFormWindow(db *clarity.Db) (*OrderFormWindow, error) {
\tw := &OrderFormWindow{}
\torder := NewOrder()
\tw.Order = &order
\torders, err := clarity.All[Order](db)
\tif err != nil {
\t\treturn nil, err
\t}
\tw.Orders = orders
\treturn w, nil
}",
    );
}

#[test]
fn handlers_are_methods_of_their_window() {
    let files = windows();
    let shop = file(&files, "shop_module.go");
    has(shop, "func (w *OrderFormWindow) approveClicked(db *clarity.Db, ui *clarity.Ui) error {");
    has(shop, "\t\tui.Message(clarity.Warning, clarity.NewText(\"Order {order.number} is too large\").With(\"order.number\", clarity.SlotText(w.Order.Number)))\n\t\tui.SetEnabled(\"approve_button\", false)\n\t\treturn nil\n");
    has(shop, "\tui.SetError(\"number_input\", clarity.PlainText(\"\"))\n\tui.Message(clarity.Info, clarity.PlainText(\"Approved\"))\n\tif err := w.filterChanged(db, ui); err != nil {\n\t\treturn err\n\t}\n");
    has(shop, "\tv1, err := clarity.Find[Order](db, clarity.Query{}.Filter(clarity.Contains(\"number\", ui.Text(\"number_filter\"))).Order(\"number\", false))\n\tif err != nil {\n\t\treturn err\n\t}\n\tw.Orders = v1\n");
    has(shop, "\tv1, err := clarity.Selected(ui, \"grid\", w.Orders)\n\tif err != nil {\n\t\treturn err\n\t}\n\tw.Order = v1\n");
    has(shop, "\tui.Open(\"OrderForm\", map[string]any{\"order\": v1})\n\tui.Close()\n");
    has(shop, "\tw.Orders = v1\n\tui.Deselect(\"grid\")\n\tif w.Order.ID != 0 {\n\t\tv2, err := clarity.Get[Order](db, w.Order.ID)\n\t\tif err != nil {\n\t\t\treturn err\n\t\t}\n\t\tw.Order = &v2\n\t}\n");
    has(shop, "\tui.SetReadonly(\"number_input\", w.Order.Paid)\n");
}

#[test]
fn the_program_dispatches_to_the_handlers() {
    let files = windows();
    let main = file(&files, "main.go");
    has(main, "func (windows) Open(db *clarity.Db, window string, with map[string]any) (clarity.WindowState, error) {");
    has(main, "\tcase \"OrderForm\":\n\t\tw, err := loadOrderFormWindow(db)\n\t\tif err != nil {\n\t\t\treturn clarity.WindowState{}, err\n\t\t}\n\t\treturn clarity.Opened(window, w, with)\n");
    has(main, "\tcase \"OrderForm\":\n\t\tswitch event {\n\t\tcase \"open\":\n\t\t\treturn \"window_opened\", true\n");
    has(main, "\t\tcase \"approve_clicked\":\n\t\t\tw := &OrderFormWindow{}\n\t\t\tif err := state.Decode(w); err != nil {\n\t\t\t\treturn clarity.Reply{}, err\n\t\t\t}\n\t\t\tif err := w.approveClicked(db, ui); err != nil {\n\t\t\t\treturn clarity.Reply{}, err\n\t\t\t}\n\t\t\treturn ui.Reply(state.Window, w)\n");
    has(main, "\tcase \"Customer\":\n\t\trows, err := clarity.All[Customer](db)\n\t\tif err != nil {\n\t\t\treturn nil, err\n\t\t}\n\t\tfor _, r := range rows {\n\t\t\toptions = append(options, clarity.RefOption{ID: r.ID, Text: r.Name})\n\t\t}\n");
}

/// An external source (D1e) on Go: opened with its own driver but never
/// migrated, its records read and written by its own table, key and column
/// names, with a read-only one on a server database.
#[test]
fn an_external_source_keeps_its_names_and_is_never_migrated() {
    use clarity_project::DataSource;
    use clarity_spec::SourceRef;
    let erp = "module erp\nuse sales\nsource erp\n\nentity Invoice\n    table \"tblInvoice\"\n    key \"InvoiceID\"\n    number: text(20), column \"InvNo\"\n    total: money\n\nfunction big(limit: money) -> list<Invoice>\n    return find i in Invoice where i.number contains \"INV\" and i.total > limit order by i.number\n";
    let mut project = load_sources([("sales.clarity", SALES), ("erp.clarity", erp)]);
    project.sources = vec![SourceRef::sql("main"), SourceRef { readonly: true, ..SourceRef::sql("erp").external() }];
    let mut settings = settings();
    settings.datasources.push(DataSource {
        name: "erp".into(),
        kind: SourceKind::Mssql,
        url: "env:ERP_DATABASE".into(),
        auth: None,
        external: true,
        readonly: true,
    });
    let files = generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"));
    has(file(&files, "main.go"), "{Name: \"erp\", Dialect: clarity.Mssql, At: \"env:ERP_DATABASE\"}");
    let schema = file(&files, "schema.go");
    assert!(!schema.contains("\"erp\"") && !schema.contains("tblInvoice"), "{schema}");
    let erp_go = file(&files, "erp_module.go");
    has(erp_go, "func (*Invoice) Source() string {\n\treturn \"erp\"\n}");
    has(erp_go, "func (*Invoice) Table() string {\n\treturn \"tblInvoice\"\n}");
    has(erp_go, "func (*Invoice) Key() string {\n\treturn \"InvoiceID\"\n}");
    has(erp_go, "[]string{\"InvNo\", \"total\"}");
    has(erp_go, "clarity.Contains(\"InvNo\", \"INV\")");
    has(erp_go, ".Order(\"InvNo\", false)");
    // Its database's driver comes along, though Clarity owns no table there.
    has(file(&files, "main.go"), &format!("\t_ \"{RUNTIME}/sqlserver\"\n"));
}

/// X2: a query through a reference: the condition and the order read the
/// referred record in a subquery.
#[test]
fn a_query_goes_through_a_reference() {
    let source = format!(
        "{SALES}\nfunction of_customers(name: text) -> list<Order>\n    return find o in Order where o.customer.name contains name and o.customer.credit_limit > 10 order by o.customer.name desc, o.customer.credit_limit\n"
    );
    let files = generate(&load_sources([("sales.clarity", source.as_str())]), &settings()).unwrap_or_else(|e| panic!("{e:#?}"));
    let sales = file(&files, "sales_module.go");
    has(sales, ".Filter(clarity.And(clarity.Through(\"customer\", \"customer\", \"id\", clarity.Contains(\"name\", name)), clarity.Through(\"customer\", \"customer\", \"id\", clarity.Number(\"credit_limit\", clarity.Gt, ");
    has(sales, ".OrderThrough([]clarity.Hop{{Column: \"customer\", Table: \"customer\", Key: \"id\"}}, \"name\", true, false).OrderThrough([]clarity.Hop{{Column: \"customer\", Table: \"customer\", Key: \"id\"}}, \"credit_limit\", false, true)");
}

/// Rows read again, or replaced by a query, are other records: a table or
/// list showing them loses its selected row, as on every stack.
#[test]
fn rows_read_again_lose_their_selected_row() {
    let files = windows();
    let shop = file(&files, "shop_module.go");
    has(shop, ".Order(\"number\", false))\n\tif err != nil {\n\t\treturn err\n\t}\n\tw.Orders = v1\n\tui.Deselect(\"grid\")\n");
    has(shop, "\tw.Orders = v1\n\tui.Deselect(\"grid\")\n\tif w.Order.ID != 0 {");
    assert_eq!(shop.matches("ui.Deselect(").count(), 2, "{shop}");
}
