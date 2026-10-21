//! The C backend (G4): a CMake project with the runtime copied in, the
//! records of every module, and each module's functions, actions, data
//! events and windows, as a C developer writes them: every call that can
//! fail says whether it worked.

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
    settings.stack.backend = Backend::C;
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
    let cmake = file(&files, "CMakeLists.txt");
    has(cmake, "project(shop LANGUAGES C CXX)");
    has(cmake, "set(CMAKE_C_STANDARD 17)");
    has(cmake, "add_executable(shop ${PROGRAM_SOURCES})");
    has(cmake, "include(clarity-runtime/clarity.cmake)");
    has(file(&files, "CMakePresets.json"), "\"workflowPresets\"");
    assert_eq!(file(&files, "clarity-runtime/clarity/db.h"), include_str!("../../../runtimes/c/clarity/db.h"));
    assert!(files.iter().any(|f| f.path == "clarity-runtime/third_party/webview/webview.h"));
    assert!(files.iter().any(|f| f.path == "clarity-runtime/third_party/cjson/cJSON.c"));
    assert!(files.iter().any(|f| f.path == "README.md"));
    // The runtime's own tests stay behind.
    assert!(!files.iter().any(|f| f.path.contains("tests/")));
}

/// A runtime file added later must reach generated projects too.
#[test]
fn every_c_runtime_source_file_is_copied() {
    let files = files();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../runtimes/c");
    for folder in ["clarity", "shell", "third_party/webview", "third_party/cjson"] {
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
fn enums_and_entities_become_c_types() {
    let files = files();
    let records = file(&files, "src/records.h");
    has(records, "typedef enum { ORDER_STATUS_PENDING, ORDER_STATUS_APPROVED } OrderStatus;");
    has(records, "typedef struct {\n    Order* items;\n    size_t count;\n} OrderList;");
    has(records, "struct Customer {\n    int64_t id;\n    const char* name;\n    ClarityDecimal credit_limit;\n    bool active;\n    ClarityDate since;\n};");
    has(records, "bool order_save(ClarityDb* db, Order* record);");
    has(records, "bool order_find(ClarityDb* db, const ClarityQuery* query, OrderList* out);");
    let source = file(&files, "src/records.c");
    has(source, "    case ORDER_STATUS_APPROVED: return \"Approved by us\";");
    has(source, "    if (!clarity_get_text(\"status\", values[1], &status_name)) return false;\n    r->status = order_status_from(status_name);");
    has(source, "    cJSON_AddItemToObject(out, \"customer\", clarity_json_reference(r->customer));");
    has(source, "Customer customer_new(void) {\n    return (Customer){\n        .name = \"\",\n        .credit_limit = clarity_decimal(100, 0),\n        .active = true,\n    };\n}");
    has(source, "    .table = \"order\",");
    has(source, "    .before_save = order_before_save,");
    has(source, "static bool order_before_save(ClarityDb* db, void* record) {\n    return before_save_order(db, record);\n}");
}

#[test]
fn functions_and_actions_say_whether_they_worked() {
    let files = files();
    let sales = file(&files, "src/modules/sales.c");
    has(sales, "bool within_credit(ClarityDb* db, Order* order, bool* result) {\n    Customer v1 = {0};\n    if (!customer_load(db, order->customer, &v1)) return false;\n    *result = clarity_decimal_compare(order->total, v1.credit_limit) <= 0;\n    return true;\n}");
    has(sales, "bool approve(ClarityDb* db, Order* order) {\n    if (!clarity_require(order->status == ORDER_STATUS_PENDING, \"order.status == Pending\")) return false;\n    order->status = ORDER_STATUS_APPROVED;\n    if (!order_save(db, order)) return false;\n    return true;\n}");
    has(sales, "if (!order_find(db, clarity_order_by(clarity_order_by_number(clarity_where(clarity_query(), clarity_and(clarity_number(\"total\", CLARITY_GT_EQ, clarity_decimal_value(limit)), clarity_one_of(\"status\", 2, (ClarityValue[]){clarity_text_value(order_status_name(ORDER_STATUS_PENDING)), clarity_text_value(order_status_name(ORDER_STATUS_APPROVED))}))), \"total\", true), \"number\", false), &v1)) return false;");
    has(sales, "bool before_save_order(ClarityDb* db, Order* order) {\n    (void)db;\n    if (clarity_same_text(order->number, \"\")) {\n        return clarity_refuse(clarity_plain_text(\"An order needs a number\"));\n    }\n    return true;\n}");
    has(file(&files, "src/app.h"), "bool approve(ClarityDb* db, Order* order);");
}

#[test]
fn handlers_are_functions_of_their_window() {
    let files = files();
    let app = file(&files, "src/app.h");
    has(app, "typedef struct {\n    Order order;\n    OrderList orders;\n} OrderFormWindow;");
    has(app, "bool order_form_window_approve_clicked(ClarityDb* db, ClarityUi* ui, OrderFormWindow* w);");
    let sales = file(&files, "src/modules/sales.c");
    has(sales, "bool order_form_window_approve_clicked(ClarityDb* db, ClarityUi* ui, OrderFormWindow* w) {\n    if (!approve(db, &w->order)) return false;\n    clarity_ui_message(ui, CLARITY_INFO, clarity_text_with(clarity_text_with(clarity_text(\"Approved {order.number} of {order.total}\"), \"order.number\", clarity_slot_text(w->order.number)), \"order.total\", clarity_slot_money(w->order.total, NULL)));\n    clarity_ui_set_enabled(ui, \"approveButton\", false);\n    return true;\n}");
    has(sales, "    size_t i1 = 0;\n    if (!clarity_ui_selected(ui, \"grid\", w->orders.count, &i1)) return false;\n    Order v1 = w->orders.items[i1];\n    clarity_ui_open(ui, \"OrderForm\", clarity_json_object(1, \"order\", order_json(&v1)));");
    let main = file(&files, "src/main.c");
    has(main, "    if (strcmp(state->window, \"OrderForm\") == 0) {\n        OrderFormWindow w = {0};\n        if (!order_form_window_from_json(state->data, &w)) return false;\n        bool ok = false;\n        if (strcmp(handler, \"approveClicked\") == 0) ok = order_form_window_approve_clicked(db, ui, &w);");
    has(main, "        for (size_t i = 0; i < all.count; i++) clarity_add_option(*out, all.items[i].id, all.items[i].name);");
}

/// The renderer's page carries its script and styles, since the webview
/// shows a page and no folder.
#[test]
fn the_page_carries_the_renderer() {
    let files = files();
    let page = file(&files, "src/page.c");
    has(page, "<script type=\\\"module\\\">");
    assert!(!page.contains("src=\\\"./runtime.js\\\""), "the script is in the page");
    has(page, "const char* page_html(void) {");
}

#[test]
fn server_databases_come_later() {
    let project = load_sources([("sales.clarity", SALES)]);
    let mut settings = settings();
    settings.datasources[0].kind = SourceKind::Postgres;
    settings.datasources[0].url = "env:SHOP_DATABASE".into();
    assert_eq!(generate(&project, &settings).unwrap_err(), ["data source 'main' is a PostgreSQL database; the C backend stores records in SQLite for now"]);
}

/// Rows read again are other records: a table or list showing them loses its
/// selected row, as on every stack.
#[test]
fn rows_read_again_lose_their_selected_row() {
    let files = files();
    let code: String = files.iter().filter(|f| f.path.starts_with("src/")).map(|f| f.text.as_str()).collect();
    has(&code, "clarity_ui_deselect(ui, \"grid\");");
    assert_eq!(code.matches("deselect").count(), 1, "{code}");
}

/// The desktop window opens at the main window's size.
#[test]
fn the_desktop_window_opens_at_the_main_windows_size() {
    let code: String = files().iter().filter(|f| f.path.ends_with(".c")).map(|f| f.text.as_str()).collect();
    has(&code, "page_model(), page_html(), db, &windows, 1100, 760};");
    let sized = SALES.replace("    title \"Order\"\n", "    title \"Order\"\n    size 640 x 480\n");
    assert_ne!(sized, SALES);
    let project = load_sources([("sales.clarity", sized.as_str())]);
    let code: String = generate(&project, &settings()).unwrap().iter().filter(|f| f.path.ends_with(".c")).map(|f| f.text.clone()).collect();
    has(&code, "page_model(), page_html(), db, &windows, 640, 480};");
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
    let app = file(&files, "src/app.h");
    has(app, "bool order_form_window_digit(ClarityDb* db, ClarityUi* ui, OrderFormWindow* w, int64_t d);");
    has(app, "bool order_form_window_scale(ClarityDb* db, ClarityUi* ui, OrderFormWindow* w, ClarityDecimal by, const char* word, OrderStatus status);");
    let sales = file(&files, "src/modules/sales.c");
    has(sales, "if (!order_form_window_digit(db, ui, w, 3)) return false;");
    let main = file(&files, "src/main.c");
    has(main, "else if (strcmp(handler, \"digit(7)\") == 0) ok = order_form_window_digit(db, ui, &w, 7);");
    // A handler that takes values is only called with them.
    assert!(!main.contains("(strcmp(handler, \"digit\") == 0)"), "{main}");
    has(main, "else if (strcmp(handler, \"scale(0.5, \\\"half\\\", Approved)\") == 0) ok = order_form_window_scale(db, ui, &w, clarity_decimal(5, 1), \"half\", ORDER_STATUS_APPROVED);");
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
    has(file(&files, "src/app.h"), "typedef struct {\n    ClarityDecimal shown;\n    OrderStatus pending;\n    bool fresh;\n    const char* word;\n    int64_t count;\n    ClarityDecimal rate;\n} CalcWindow;");
    let sales = file(&files, "src/modules/sales.c");
    for load in ["    w->shown = (ClarityDecimal){0};\n", "    w->fresh = true;\n", "    w->word = clarity_text_of(\"calcWord\");\n", "    w->count = 2;\n", "    w->rate = clarity_decimal(5, 1);\n"] {
        has(sales, load);
    }
    has(sales, "cJSON_AddItemToObject(out, \"fresh\", clarity_json_bool(w->fresh));");
    has(sales, "cJSON_AddItemToObject(out, \"pending\", clarity_json_text(order_status_name(w->pending)));");
    has(sales, "if (!clarity_json_get_decimal(in, \"shown\", &w->shown)) return false;");
    has(sales, "if (pending_name) w->pending = order_status_from(pending_name);");
    has(sales, "w->fresh = false;\n");
    has(sales, "w->pending = ORDER_STATUS_APPROVED;\n");
    // A text value shows its text in the person's language.
    has(sales, "clarity_ui_set_shown(ui, \"word\", clarity_key_text(\"x\"));\n");
    // A key in code takes its value in the language the screen picked.
    has(sales, "clarity_text_show(clarity_key_text(\"draw\"))");
    has(sales, "clarity_text_show(clarity_text_with(clarity_key_text(\"turnN\"), \"n\", clarity_slot_integer(n, NULL)))");
}

/// round(value, places) rounds through the runtime, as every stack does.
#[test]
fn round_rounds_through_the_runtime() {
    let source = format!("{SALES}\nfunction rounded(x: decimal) -> decimal\n    return round(x, 2)\n");
    let project = load_sources([("m.clarity", source.as_str())]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let files = generate(&project, &settings()).unwrap_or_else(|e| panic!("{e:#?}"));
    let code: String = files.iter().map(|f| f.text.as_str()).collect();
    assert!(code.contains("clarity_decimal_round(x, (int)2)"), "{code}");
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
    let code = file(&files, "src/modules/sales.c");
    has(code, "cJSON_AddStringToObject(shape1, \"op\", \"clear\");\n    cJSON_AddStringToObject(shape1, \"color\", \"#101810\");\n    clarity_ui_draw(ui, \"dial\", shape1);");
    has(code, "cJSON_AddNumberToObject(shape2, \"x1\", (double)(50));");
    has(code, "clarity_decimal_cos(w->angle)");
    has(code, "cJSON_AddNumberToObject(shape2, \"width\", (double)(3));");
    // Left out: the text's color and size.
    has(code, "cJSON_AddStringToObject(shape3, \"color\", \"text\");\n    cJSON_AddNumberToObject(shape3, \"size\", 10);");
    let main = file(&files, "src/main.c");
    has(main, "static bool redraw(ClarityDb* db, ClarityReply* reply) {");
    has(main, "        clarity_ui_begin_drawing(ui, \"dial\");\n        if (!dial_window_draw_dial(db, ui, &w)) return false;");
    has(main, "picture, redraw};");
}

/// An application with an engine gets the host and wasm3; one without, neither.
#[test]
fn an_engine_brings_its_host_and_wasm3() {
    let mut project = clarity_spec::load_sources([("shop.clarity", include_str!("../../../conformance/engines/shop.clarity"))]);
    let bytes = include_bytes!("../../../conformance/engines/test.wasm").to_vec();
    clarity_spec::engines::attach(&mut project, vec![clarity_spec::engines::EngineFile { name: "test.wasm".into(), bytes }]);
    let mut settings = clarity_project::starter("Shop", "shop");
    settings.stack.backend = clarity_project::Backend::C;
    settings.stack.ui = clarity_project::Ui::Web;
    let with = clarity_gen::generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"));
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../runtimes/c");
    for folder in ["engine", "third_party/wasm3"] {
        for entry in std::fs::read_dir(root.join(folder)).unwrap() {
            let name = entry.unwrap().file_name().to_string_lossy().to_string();
            assert!(with.iter().any(|f| f.path == format!("clarity-runtime/{folder}/{name}")), "{folder}/{name} is not copied");
        }
    }
    assert!(!files().iter().any(|f| f.path.contains("engine") || f.path.contains("wasm3")));
}

/// A canvas with `on click` runs its handler, which reads where it was clicked.
#[test]
fn a_click_on_a_canvas_runs_its_handler_with_where() {
    let source = format!("{SALES}{DIAL}").replace("on draw: drawDial\n", "on draw: drawDial on click: dialClicked\n") + "\n    handler dialClicked\n        speed = dial.x + dial.y\n";
    let project = load_sources([("sales.clarity", source.as_str())]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let files = generate(&project, &settings()).unwrap_or_else(|e| panic!("{e:#?}"));
    let code = file(&files, "src/modules/sales.c");
    assert!(code.contains("clarity_ui_x(ui, \"dial\") + clarity_ui_y(ui, \"dial\")"), "{code}");
}
