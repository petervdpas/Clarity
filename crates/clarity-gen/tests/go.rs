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
    menu name mainMenu
        item \"File\" name fileItem
            item \"Orders\" name ordersItem key \"Ctrl+O\" on click -> OrderForm
            item \"Another order\" name anotherOrderItem on click -> OrderForm in new window
            separator
            item \"Quit\" name quitItem on click -> quit
            item \"Language\" name languageItem on click -> language
            item \"Theme\" name themeItem on click -> theme
    toolbar name mainToolbar
        button \"Look\" name lookButton on click -> theme
    footer
        status \"Ready\"

page OrderForm: window
    title \"Order\"
    data order: Order
    data orders: list<Order>
    on open: windowOpened
    on return: windowReturned
    label \"Number\" name label1 at 1,1 span 2 for numberInput
    textbox name numberInput at 3,1 span 4 bind order.number placeholder \"ORD-1\"
    help name help1 at 3,2 span 4 for numberInput
    number name number1 at 1,3 span 3 bind order.total
    checkbox \"Paid\" name checkbox1 at 4,3 span 3 bind order.paid
    switch \"Rush\" name switch1 at 7,3 span 3 bind order.paid
    dropdown name dropdown1 at 1,4 span 4 bind order.status
    dropdown name customerPick at 5,4 span 4 bind order.customer
    date name date1 at 9,4 span 3 bind order.ordered_on
    text name text1 at 1,5 span 4 bind order.status
    table name grid at 1,6 span 12 rows 4 bind orders columns number, status, total on select: gridSelected on open: gridOpened
    textbox name numberFilter at 1,10 span 4 on change: filterChanged
    button \"Approve\" name approveButton at 9,10 span 2 default on click: approveClicked
    button \"New\" name newOrder at 11,10 span 2 on click -> OrderForm
    tabs name tabs1 at 1,11 span 12 rows 2
        tab \"Notes\"
            memo name memo1 at 1,1 span 12 bind order.number

code OrderForm
    handler windowOpened
        numberInput.readonly = order.paid

    handler windowReturned
        reload orders
        reload order

    handler approveClicked
        if not within_credit(order)
            message warning \"Order {order.number} is too large\"
            approveButton.enabled = false
            return
        order.status = Approved
        save order
        numberInput.error = \"\"
        message \"Approved\"
        filterChanged()

    handler filterChanged
        orders = find o in Order where o.number contains numberFilter.text order by o.number

    handler gridSelected
        order = grid.selected

    handler gridOpened
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
    has(main, "shell.Run(shell.Options{Name: \"shop\", Title: \"Shop\", Main: \"OrderForm\", Width: 1100, Height: 760, Model: model, Assets: assets, Db: db, Windows: windows{}}, bind)");
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
    has(shop, "\t\tui.Message(clarity.Warning, clarity.NewText(\"Order {order.number} is too large\").With(\"order.number\", clarity.SlotText(w.Order.Number)))\n\t\tui.SetEnabled(\"approveButton\", false)\n\t\treturn nil\n");
    has(shop, "\tui.SetError(\"numberInput\", clarity.PlainText(\"\"))\n\tui.Message(clarity.Info, clarity.PlainText(\"Approved\"))\n\tif err := w.filterChanged(db, ui); err != nil {\n\t\treturn err\n\t}\n");
    has(shop, "\tv1, err := clarity.Find[Order](db, clarity.Query{}.Filter(clarity.Contains(\"number\", ui.Text(\"numberFilter\"))).Order(\"number\", false))\n\tif err != nil {\n\t\treturn err\n\t}\n\tw.Orders = v1\n");
    has(shop, "\tv1, err := clarity.Selected(ui, \"grid\", w.Orders)\n\tif err != nil {\n\t\treturn err\n\t}\n\tw.Order = v1\n");
    has(shop, "\tui.Open(\"OrderForm\", map[string]any{\"order\": v1})\n\tui.Close()\n");
    has(shop, "\tw.Orders = v1\n\tui.Deselect(\"grid\")\n\tif w.Order.ID != 0 {\n\t\tv2, err := clarity.Get[Order](db, w.Order.ID)\n\t\tif err != nil {\n\t\t\treturn err\n\t\t}\n\t\tw.Order = &v2\n\t}\n");
    has(shop, "\tui.SetReadonly(\"numberInput\", w.Order.Paid)\n");
}

#[test]
fn the_program_dispatches_to_the_handlers() {
    let files = windows();
    let main = file(&files, "main.go");
    has(main, "func (windows) Open(db *clarity.Db, window string, with map[string]any) (clarity.WindowState, error) {");
    has(main, "\tcase \"OrderForm\":\n\t\tw, err := loadOrderFormWindow(db)\n\t\tif err != nil {\n\t\t\treturn clarity.WindowState{}, err\n\t\t}\n\t\treturn clarity.Opened(window, w, with)\n");
    has(main, "\tcase \"OrderForm\":\n\t\tswitch event {\n\t\tcase \"open\":\n\t\t\treturn \"windowOpened\", true\n");
    has(main, "\t\tcase \"approveClicked\":\n\t\t\tw := &OrderFormWindow{}\n\t\t\tif err := state.Decode(w); err != nil {\n\t\t\t\treturn clarity.Reply{}, err\n\t\t\t}\n\t\t\tif err := w.approveClicked(db, ui); err != nil {\n\t\t\t\treturn clarity.Reply{}, err\n\t\t\t}\n\t\t\treturn ui.Reply(state.Window, w)\n");
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

/// The desktop window opens at the main window's size.
#[test]
fn the_desktop_window_opens_at_the_main_windows_size() {
    has(file(&windows(), "main.go"), "Main: \"OrderForm\", Width: 1100, Height: 760,");
    let sized = WINDOWS.replace("    title \"Order\"\n", "    title \"Order\"\n    size 640 x 480\n");
    let project = load_sources([("shop.clarity", sized.as_str())]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    has(file(&generate(&project, &settings()).unwrap(), "main.go"), "Main: \"OrderForm\", Width: 640, Height: 480,");
}

/// A click passes values: each written call is dispatched with its values
/// compiled in, and a handler may call another with values.
#[test]
fn a_click_passes_values_to_its_handler() {
    let source = WINDOWS
        .replace(
            "    button \"New\" name newOrder at 11,10 span 2 on click -> OrderForm\n",
            "    button \"New\" name newOrder at 11,10 span 2 on click -> OrderForm\n    button \"7\" name d7 at 5,10 span 1 on click: digit(7)\n    button \"Half\" name half at 6,10 span 1 on click: scale(0.5, \"half\", Approved)\n",
        )
        .replace(
            "    handler gridSelected\n",
            "    handler digit(d: integer)\n        numberInput.error = \"\"\n\n    handler scale(by: decimal, word: text, status: OrderStatus)\n        digit(3)\n\n    handler gridSelected\n",
        );
    assert_ne!(source, WINDOWS);
    let project = load_sources([("shop.clarity", source.as_str())]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let files = generate(&project, &settings()).unwrap_or_else(|e| panic!("{e:#?}"));
    let shop = file(&files, "shop_module.go");
    has(shop, "func (w *OrderFormWindow) digit(db *clarity.Db, ui *clarity.Ui, d int64) error {");
    has(shop, "func (w *OrderFormWindow) scale(db *clarity.Db, ui *clarity.Ui, by decimal.Decimal, word string, status OrderStatus) error {");
    has(shop, "if err := w.digit(db, ui, 3); err != nil {");
    let main = file(&files, "main.go");
    has(main, "\t\tcase \"digit(7)\":\n");
    // A handler that takes values is only called with them.
    assert!(!main.contains("\t\tcase \"digit\":\n"), "{main}");
    has(main, "if err := w.digit(db, ui, 7); err != nil {");
    has(main, "\t\tcase \"scale(0.5, \\\"half\\\", Approved)\":\n");
    has(main, "if err := w.scale(db, ui, decimal.RequireFromString(\"0.5\"), \"half\", OrderStatusApproved); err != nil {");
    // The dispatch writes a decimal, so main.go imports it.
    has(main, "\t\"github.com/shopspring/decimal\"\n");
    assert!(!file(&windows(), "main.go").contains("shopspring/decimal"));
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

/// A function that returns texts, plain and with a slot: both are translated.
const VERDICT: &str = "
function verdict(n: integer) -> text
    if n > 2
        return \"Draw\"
    return \"Turn {n}\"
";

/// A window holds plain values: fields of its data, at their default or
/// empty when it opens, read and changed by its handlers.
#[test]
fn a_window_holds_plain_values() {
    let source = format!("{SALES}{CALC}{VERDICT}");
    let project = clarity_spec::keyed::load_keyed([("sales.clarity", source.as_str())], &[], "en");
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let files = generate(&project, &settings()).unwrap_or_else(|e| panic!("{e:#?}"));
    let code: String = files.iter().filter(|f| f.path.ends_with("_module.go")).map(|f| f.text.as_str()).collect();
    for field in [["Shown", "decimal.Decimal", "`json:\"shown\"`"], ["Pending", "OrderStatus", "`json:\"pending\"`"], ["Fresh", "bool", "`json:\"fresh\"`"], ["Word", "string", "`json:\"word\"`"], ["Count", "int64", "`json:\"count\"`"]] {
        assert!(code.lines().any(|l| l.split_whitespace().eq(field)), "{field:?} in {code}");
    }
    for load in ["\tw.Fresh = true\n", "\tw.Word = clarity.TextOf(\"calcWord\")\n", "\tw.Count = 2\n", "\tw.Rate = decimal.RequireFromString(\"0.5\")\n", "\tw.Pending = OrderStatusPending\n"] {
        has(&code, load);
    }
    has(&code, "w.Fresh = false\n");
    has(&code, "w.Pending = OrderStatusApproved\n");
    // A text value shows its text in the person's language.
    has(&code, "ui.SetShown(\"word\", clarity.KeyText(\"x\"))\n");
    // A key in code takes its value in the language the screen picked.
    has(&code, "clarity.KeyText(\"draw\").Show()");
    has(&code, "clarity.KeyText(\"turnN\").With(\"n\", ");
}

/// round(value, places) rounds through the runtime, as every stack does.
#[test]
fn round_rounds_through_the_runtime() {
    let source = format!("{SALES}\nfunction rounded(x: decimal) -> decimal\n    return round(x, 2)\n");
    let project = load_sources([("m.clarity", source.as_str())]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let files = generate(&project, &settings()).unwrap_or_else(|e| panic!("{e:#?}"));
    let code: String = files.iter().map(|f| f.text.as_str()).collect();
    assert!(code.contains("clarity.Round(x, 2)"), "{code}");
}

/// A window with a canvas its draw handler draws, as each stack's tests use it.
const DIAL: &str = "
page Dial: window
    data angle: decimal, default 30
    data speed: integer, default 80
    canvas name dial at 1,1 span 6 rows 6 units 200 x 100 on draw: drawDial
    button \"Faster\" name faster at 7,1 span 2 on click: fasterClicked

code Dial
    handler drawDial
        dial.clear(\"#101810\")
        dial.line(50, 50, 50 + 35 * cos(angle), 50 - 35 * sin(angle), \"text\", 3)
        dial.text(50, 90, \"{speed} km/h\")

    handler fasterClicked
        speed = speed + 10
";

/// A draw handler adds shapes to its canvas's drawing; the shell draws the
/// canvas anew when the window opens and after every handler.
#[test]
fn a_canvas_is_drawn_by_its_draw_handler() {
    let source = format!("{SALES}{DIAL}");
    let project = load_sources([("sales.clarity", source.as_str())]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let files = generate(&project, &settings()).unwrap_or_else(|e| panic!("{e:#?}"));
    let code = file(&files, "sales_module.go");
    has(code, "ui.Draw(\"dial\", map[string]any{\"op\": \"clear\", \"color\": \"#101810\"})");
    has(code, "\"op\": \"line\", \"x1\": float64(50), \"y1\": float64(50), \"x2\": clarity.Units(");
    has(code, "clarity.Cos(w.Angle)");
    has(code, "\"width\": float64(3)})");
    // Left out: the text's color and size.
    has(code, "\"color\": \"text\", \"size\": 10.0})");
    let main = file(&files, "main.go");
    has(main, "func (windows) Redraw(db *clarity.Db, reply clarity.Reply) (clarity.Reply, error) {");
    has(main, "\t\tui.BeginDrawing(\"dial\")\n\t\tif err := w.drawDial(db, ui); err != nil {");
}

/// A function whose Go name another top-level name already has (a type,
/// an enum value, a constructor, a window) takes a trailing `_`, in its
/// declaration and in every call.
#[test]
fn a_function_never_takes_the_name_of_a_type() {
    let project = load_sources([("shop.clarity", include_str!("fixtures/name_clash.clarity"))]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let files = generate(&project, &settings()).unwrap_or_else(|e| panic!("{e:#?}"));
    let code = file(&files, "shop_module.go");
    for declared in ["func Size_(", "func SizeSmall_(", "func NewOrder_(", "func MainWindow_(", "func Sources_("] {
        has(code, declared);
    }
    has(code, "Size_(db, 1)");
    has(code, "MainWindow_(db)");
    // The types keep their own names.
    has(code, "type Size string");
    has(code, "func NewOrder() Order");
}

/// A canvas with `on click` runs its handler, which reads where it was clicked.
#[test]
fn a_click_on_a_canvas_runs_its_handler_with_where() {
    let source = format!("{SALES}{DIAL}").replace("on draw: drawDial\n", "on draw: drawDial on click: dialClicked\n") + "\n    handler dialClicked\n        speed = dial.x + dial.y\n";
    let project = load_sources([("sales.clarity", source.as_str())]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let files = generate(&project, &settings()).unwrap_or_else(|e| panic!("{e:#?}"));
    let code = file(&files, "sales_module.go");
    assert!(code.contains("ui.X(\"dial\") + ui.Y(\"dial\")"), "{code}");
}

/// A local an integer starts in is an int64, as Clarity's integers are,
/// though Go would make a bare number an int.
#[test]
fn a_local_from_a_number_is_an_int64() {
    let source = format!("{SALES}{DIAL}") + "\n    handler countUp\n        let step = 1\n        speed = speed + step\n";
    let project = load_sources([("sales.clarity", source.as_str())]);
    let files = generate(&project, &settings()).unwrap_or_else(|e| panic!("{e:#?}"));
    let code = file(&files, "sales_module.go");
    assert!(code.contains("var step int64 = 1\n"), "{code}");
}

/// A step with no statement for a database (SQL Server keeps no search
/// index, so `add search` writes nothing there) is an empty list on one
/// line, as gofmt writes it.
#[test]
fn a_step_without_statements_is_written_as_gofmt_writes_it() {
    let module = "module music\n\nentity Album\n    title: text(80), search\n\napp Music\n    main Albums\n\npage Albums: window\n    size 400 x 300\n";
    let first = "migration \"Albums\"\n    add entity Album\n        title: text(80)\n";
    let second = "migration \"Search albums\"\n    add search Album.title\n";
    let project = clarity_spec::load_with_migrations([("music.clarity", module)], [("0001.clarity", first), ("0002.clarity", second)]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let mut settings = settings();
    settings.datasources[0].kind = SourceKind::Mssql;
    settings.datasources[0].url = "env:MUSIC_DATABASE".into();
    let files = generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"));
    has(file(&files, "schema.go"), "\t\t// 2: Search albums\n\t\t{},\n");
}
