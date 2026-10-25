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
    button \"Approve\" name approveButton at 1,1 span 2 on click: approveClicked
    table name grid at 1,2 span 6 rows 4 bind orders columns number on open: openClicked
    button \"Refresh\" name refreshButton at 3,1 span 2 on click: refreshClicked

code OrderForm
    handler approveClicked
        approve(order)
        message \"Approved {order.number} of {order.total}\"
        approveButton.enabled = false

    handler openClicked
        open OrderForm with order = grid.selected

    handler refreshClicked
        reload orders
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
    has(sales, "void OrderFormWindow::approve_clicked(clarity::Db& db, clarity::Ui& ui) {\n    approve(db, order);\n    ui.message(clarity::Level::Info, clarity::Text(\"Approved {order.number} of {order.total}\").with(\"order.number\", clarity::slot_text(order.number)).with(\"order.total\", clarity::slot_money(order.total, \"\")));\n    ui.set_enabled(\"approveButton\", false);\n}");
    has(sales, "ui.open(\"OrderForm\", nlohmann::json::object({{\"order\", ui.selected(\"grid\", orders)}}));");
    let main = file(&files, "src/main.cpp");
    has(main, "        if (state.window == \"OrderForm\") {\n            auto w = state.data.get<OrderFormWindow>();\n            if (handler == \"approveClicked\") w.approve_clicked(db, ui);");
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

/// Rows read again are other records: a table or list showing them loses its
/// selected row, as on every stack.
#[test]
fn rows_read_again_lose_their_selected_row() {
    let files = files();
    let code: String = files.iter().filter(|f| f.path.starts_with("src/")).map(|f| f.text.as_str()).collect();
    has(&code, "ui.deselect(\"grid\");");
    assert_eq!(code.matches("deselect").count(), 1, "{code}");
}

/// The desktop window opens at the main window's size.
#[test]
fn the_desktop_window_opens_at_the_main_windows_size() {
    let code: String = files().iter().filter(|f| f.path.ends_with(".cpp")).map(|f| f.text.as_str()).collect();
    has(&code, "::page(), &db, &windows, {1100, 760}};");
    let sized = SALES.replace("    title \"Order\"\n", "    title \"Order\"\n    size 640 x 480\n");
    assert_ne!(sized, SALES);
    let project = load_sources([("sales.clarity", sized.as_str())]);
    let code: String = generate(&project, &settings()).unwrap().iter().filter(|f| f.path.ends_with(".cpp")).map(|f| f.text.clone()).collect();
    has(&code, "::page(), &db, &windows, {640, 480}};");
}

/// A click can pass values to its handler: each written call is a case of
/// its own, with its values written into the code.
#[test]
fn a_click_passes_values_to_its_handler() {
    let source = SALES
        .replace(
            "    button \"Refresh\" name refreshButton at 3,1 span 2 on click: refreshClicked\n",
            "    button \"Refresh\" name refreshButton at 3,1 span 2 on click: refreshClicked\n    button \"7\" name d7 at 5,1 span 1 on click: digit(7)\n    button \"Half\" name half at 6,1 span 1 on click: scale(0.5, \"half\", Approved)\n",
        )
        .replace(
            "    handler refreshClicked\n",
            "    handler digit(d: integer)\n        approveButton.enabled = false\n\n    handler scale(by: decimal, word: text, status: OrderStatus)\n        digit(3)\n\n    handler refreshClicked\n",
        );
    assert_ne!(source, SALES);
    let project = load_sources([("sales.clarity", source.as_str())]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let files = generate(&project, &settings()).unwrap_or_else(|e| panic!("{e:#?}"));
    let app = file(&files, "src/app.hpp");
    has(app, "    void digit(clarity::Db& db, clarity::Ui& ui, int64_t d);");
    has(app, "    void scale(clarity::Db& db, clarity::Ui& ui, clarity::Decimal by, std::string word, OrderStatus status);");
    let sales = file(&files, "src/modules/sales.cpp");
    has(sales, "void OrderFormWindow::digit(clarity::Db& db, clarity::Ui& ui, int64_t d) {");
    has(sales, "    digit(db, ui, int64_t{3});\n");
    let main = file(&files, "src/main.cpp");
    has(main, "else if (handler == \"digit(7)\") w.digit(db, ui, 7);");
    has(main, "else if (handler == \"scale(0.5, \\\"half\\\", Approved)\") w.scale(db, ui, clarity::Decimal::parse(\"0.5\"), \"half\", OrderStatus::Approved);");
    // A handler that takes values is only called with them.
    assert!(!main.contains("(handler == \"digit\")"), "{main}");
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
    has(file(&files, "src/app.hpp"), "struct CalcWindow {\n    clarity::Decimal shown{};\n    OrderStatus pending{};\n    bool fresh = true;\n    std::string word = clarity::text_of(\"calcWord\");\n    int64_t count = 2;\n    clarity::Decimal rate = clarity::Decimal::parse(\"0.5\");\n");
    let sales = file(&files, "src/modules/sales.cpp");
    has(sales, "    fresh = false;\n");
    has(sales, "    pending = OrderStatus::Approved;\n");
    // A text value shows its text in the person's language.
    has(sales, "ui.set_shown(\"word\", clarity::Text::key(\"x\"));\n");
    // A key in code takes its value in the language the screen picked.
    has(sales, "clarity::Text::key(\"draw\").show()");
    has(sales, "clarity::Text::key(\"turnN\").with(\"n\", ");
}

/// round(value, places) rounds through the runtime, as every stack does.
#[test]
fn round_rounds_through_the_runtime() {
    let source = format!("{SALES}\nfunction rounded(x: decimal) -> decimal\n    return round(x, 2)\n");
    let project = load_sources([("m.clarity", source.as_str())]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let files = generate(&project, &settings()).unwrap_or_else(|e| panic!("{e:#?}"));
    let code: String = files.iter().map(|f| f.text.as_str()).collect();
    assert!(code.contains("x.rounded(int(int64_t{2}))"), "{code}");
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
    let code = file(&files, "src/modules/sales.cpp");
    has(code, "ui.draw(\"dial\", nlohmann::json{{\"op\", \"clear\"}, {\"color\", ");
    has(code, "{\"x1\", double(int64_t{50})}");
    has(code, "clarity::cos(angle)");
    has(code, "{\"width\", double(int64_t{3})}});");
    // Left out: the text's color and size.
    has(code, "{\"color\", \"text\"}, {\"size\", 10.0}});");
    let main = file(&files, "src/main.cpp");
    has(main, "clarity::Reply redraw([[maybe_unused]] clarity::Db& db, clarity::Reply reply) override {");
    has(main, "            ui.begin_drawing(\"dial\");\n            w.draw_dial(db, ui);");
}

/// An application with an engine (DESIGN 5.11): the host and wasm3 copied
/// in, the engine's bytes in the program, checked at start, and a call in
/// code that gives the value as its declared type.
fn engine_files() -> Vec<OutFile> {
    let mut project = load_sources([("shop.clarity", include_str!("../../../conformance/engines/shop.clarity"))]);
    let bytes = include_bytes!("../../../conformance/engines/test.wasm").to_vec();
    clarity_spec::engines::attach(&mut project, vec![clarity_spec::engines::EngineFile { name: "test.wasm".into(), bytes }]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    generate(&project, &settings()).unwrap_or_else(|e| panic!("{e:#?}"))
}

#[test]
fn an_engine_brings_its_host_and_wasm3() {
    let files = engine_files();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../runtimes");
    for (folder, from) in [("engine", root.join("cpp/engine")), ("third_party/wasm3", root.join("c/third_party/wasm3"))] {
        for entry in std::fs::read_dir(from).unwrap() {
            let name = entry.unwrap().file_name().to_string_lossy().to_string();
            assert!(files.iter().any(|f| f.path == format!("clarity-runtime/{folder}/{name}")), "{folder}/{name} is not copied");
        }
    }
    has(file(&files, "CMakeLists.txt"), "clarity_shell clarity_engine)");
    // Without an engine, neither comes along.
    assert!(!self::files().iter().any(|f| f.path.contains("engine") || f.path.contains("wasm3")));
}

#[test]
fn an_engine_call_gives_its_declared_type() {
    let files = engine_files();
    let engines = file(&files, "src/engines.cpp");
    has(engines, "const clarity::Engine& pricing()");
    has(engines, "clarity::Engine::load(\"Pricing\"");
    has(engines, "void check()");
    has(file(&files, "src/modules/shop.cpp"), "return engines::pricing().call_as<int64_t>(\"add\", nlohmann::json::array({a, int64_t{40}}));");
    has(file(&files, "src/modules/shop.cpp"), "engines::pricing().call_as<clarity::Decimal>(\"total\", nlohmann::json::array({lines}))");
    has(file(&files, "src/main.cpp"), "shop::engines::check();");
}

/// A canvas with `on click` runs its handler, which reads where it was clicked.
#[test]
fn a_click_on_a_canvas_runs_its_handler_with_where() {
    let source = format!("{SALES}{DIAL}").replace("on draw: drawDial\n", "on draw: drawDial on click: dialClicked\n") + "\n    handler dialClicked\n        speed = dial.x + dial.y\n";
    let project = load_sources([("sales.clarity", source.as_str())]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let files = generate(&project, &settings()).unwrap_or_else(|e| panic!("{e:#?}"));
    let code = file(&files, "src/modules/sales.cpp");
    assert!(code.contains("ui.x(\"dial\") + ui.y(\"dial\")"), "{code}");
}
