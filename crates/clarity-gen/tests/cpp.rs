//! The C++ backend (G4): a CMake project with the runtime copied in, the
//! records of every module, and each module's functions, actions, data
//! events and windows, as a C++ developer writes them.

use clarity_gen::{OutFile, generate};
use clarity_project::{Backend, Settings, SourceKind, Ui, starter};
use clarity_spec::load_sources;

const SALES: &str = "module sales

enum OrderStatus
    Pending
    Approved label \"Approved by us\"

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

action approve(order: Order)
    require order.status == Pending
    effects db.write
    order.status = Approved
    save order

function big(limit: money) -> list<Order>
    return find o in Order where o.total >= limit and o.status in [Pending, Approved] order by o.total desc, o.number

code Order
    before save
        if order.number == \"\"
            refuse \"An order needs a number\"

page OrderForm: window
    title \"Order\"
    data order: Order
    data orders: list<Order>
    button \"Approve\" name approve_button at 1,1 span 2 on click: approve_clicked
    table name grid at 1,2 span 6 rows 4 bind orders columns number on open: open_clicked

code OrderForm
    handler approve_clicked
        approve(order)
        message \"Approved {order.number} of {order.total}\"
        approve_button.enabled = false

    handler open_clicked
        open OrderForm with order = grid.selected
";

fn settings() -> Settings {
    let mut settings = starter("Shop", "shop");
    settings.stack.backend = Backend::Cpp;
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

fn has(text: &str, expected: &str) {
    assert!(text.contains(expected), "expected\n{expected}\nin\n{text}");
}

#[test]
fn writes_a_cmake_project_with_the_runtime_copied_in() {
    let files = files();
    has(file(&files, "CMakeLists.txt"), "add_executable(shop ${PROGRAM_SOURCES})");
    has(file(&files, "CMakeLists.txt"), "include(clarity-runtime/clarity.cmake)");
    has(file(&files, "CMakePresets.json"), "\"workflowPresets\"");
    assert_eq!(file(&files, "clarity-runtime/clarity/db.hpp"), include_str!("../../../runtimes/cpp/clarity/db.hpp"));
    assert!(files.iter().any(|f| f.path == "clarity-runtime/third_party/webview/webview.h"));
    assert!(files.iter().any(|f| f.path == "README.md"));
    // The runtime's own tests stay behind.
    assert!(!files.iter().any(|f| f.path.contains("tests/")));
}

/// A runtime file added later must reach generated projects too.
#[test]
fn every_cpp_runtime_source_file_is_copied() {
    let files = files();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../runtimes/cpp");
    for folder in ["clarity", "shell", "third_party/webview"] {
        for entry in std::fs::read_dir(root.join(folder)).unwrap() {
            let name = entry.unwrap().file_name().to_string_lossy().to_string();
            assert!(files.iter().any(|f| f.path == format!("clarity-runtime/{folder}/{name}")), "{folder}/{name} is not copied");
        }
    }
}

#[test]
fn is_deterministic() {
    assert_eq!(files(), files());
}

#[test]
fn enums_and_entities_become_cpp_types() {
    let files = files();
    let records = file(&files, "src/records.hpp");
    has(records, "enum class OrderStatus { Pending, Approved };");
    has(records, "struct Customer {\n    int64_t id = 0;\n    std::string name;\n    clarity::Decimal credit_limit = clarity::Decimal::parse(\"100\");\n    bool active = true;\n    std::optional<clarity::Date> since;\n");
    has(records, "    static constexpr const char* table = \"order\";");
    let source = file(&files, "src/records.cpp");
    has(source, "    case OrderStatus::Approved: return \"Approved by us\";");
    has(source, "    r.status = order_status_from(clarity::sql::get_text(\"status\", values[1]));");
    has(source, "    out[\"customer\"] = r.customer ? nlohmann::json(*r.customer) : nlohmann::json(nullptr);");
}

#[test]
fn functions_and_actions_throw_what_fails() {
    let files = files();
    let sales = file(&files, "src/modules/sales.cpp");
    has(sales, "bool within_credit(clarity::Db& db, Order& order) {\n    return (order.total <= db.load<Customer>(order.customer).credit_limit);\n}");
    has(sales, "void approve(clarity::Db& db, Order& order) {\n    clarity::require((order.status == OrderStatus::Pending), \"order.status == Pending\");\n    order.status = OrderStatus::Approved;\n    db.save(order);\n}");
    has(sales, "return db.find<Order>(clarity::Query().filter((clarity::Cond::number(\"total\", clarity::Op::GtEq, clarity::sql::decimal(clarity::Decimal(limit))) && clarity::Cond::one_of(\"status\", {clarity::sql::text(name_of(OrderStatus::Pending)), clarity::sql::text(name_of(OrderStatus::Approved))}))).order_number(\"total\", true).order(\"number\", false));");
    has(sales, "void Order::before_save(clarity::Db& db) {\n    before_save_order(db, *this);\n}");
    has(file(&files, "src/app.hpp"), "void approve(clarity::Db& db, Order& order);");
}

#[test]
fn handlers_are_member_functions_of_their_window() {
    let files = files();
    let app = file(&files, "src/app.hpp");
    has(app, "struct OrderFormWindow {\n    Order order;\n    std::vector<Order> orders;\n");
    let sales = file(&files, "src/modules/sales.cpp");
    has(sales, "void OrderFormWindow::approve_clicked(clarity::Db& db, clarity::Ui& ui) {\n    approve(db, order);\n    ui.message(clarity::Level::Info, clarity::Text(\"Approved {order.number} of {order.total}\").with(\"order.number\", clarity::slot_text(order.number)).with(\"order.total\", clarity::slot_money(order.total, \"\")));\n    ui.set_enabled(\"approve_button\", false);\n}");
    has(sales, "ui.open(\"OrderForm\", nlohmann::json::object({{\"order\", ui.selected(\"grid\", orders)}}));");
    let main = file(&files, "src/main.cpp");
    has(main, "        if (state.window == \"OrderForm\") {\n            auto w = state.data.get<OrderFormWindow>();\n            if (handler == \"approve_clicked\") w.approve_clicked(db, ui);");
    has(main, "        if (entity == \"Customer\") for (const auto& r : db.all<Customer>()) out.push_back({r.id, r.name});");
}

/// The renderer's page carries its script and styles, since the webview
/// shows a page and no folder.
#[test]
fn the_page_carries_the_renderer() {
    let files = files();
    let page = file(&files, "src/page.cpp");
    has(page, "<script type=\"module\">");
    assert!(!page.contains("src=\"./runtime.js\""), "the script is in the page");
    has(page, "const std::string& page() {");
}

#[test]
fn server_databases_come_later() {
    let project = load_sources([("sales.clarity", SALES)]);
    let mut settings = settings();
    settings.datasources[0].kind = SourceKind::Postgres;
    settings.datasources[0].url = "env:SHOP_DATABASE".into();
    assert_eq!(generate(&project, &settings).unwrap_err(), ["data source 'main' is a PostgreSQL database; the C++ backend stores records in SQLite for now"]);
}
