//! Clarity's table part against the cases every stack shares (C#
//! TablePartTests and its neighbours): edited cells (conformance/tables.json),
//! the order rows show in (table-order.json) and pages (table-pages.json),
//! run through the screen as the person uses it: clicks on rows, checks,
//! choices and headers, typing into cells, Tab between cells. And what it
//! draws: its header in the header look, only the rows in sight, color cells.

use super::data_rows_host;

use std::rc::Rc;

use clarity_runtime::NaiveDate;
use clarity_ui::element::find_type;
use clarity_ui::geometry::Rect;
use clarity_ui::input::{Key, KeyInput, Modifiers, PointerButton};
use clarity_ui::parts::data::table::{CellChoices, CellKind, PageButton, TablePart};
use clarity_ui::parts::FieldFacts;
use clarity_ui::screen::Screen;
use clarity_ui::testing::{part_named, window_of};
use data_rows_host::{case_file, click, click_at, laid, shot_path, DataHost};
use serde_json::{json, Value};

fn table(screen: &Screen) -> &TablePart {
    find_type::<TablePart>(screen.root(), |_| true).expect("a table")
}

fn act(screen: &mut Screen, name: &str, act: impl FnOnce(&mut TablePart)) {
    let id = part_named(screen, name);
    screen.act(id, |e, _| act(e.as_any_mut().downcast_mut::<TablePart>().unwrap()));
}

fn made(markup: &str, host: &Rc<DataHost>, width: i32, height: i32) -> Screen {
    let mut screen = window_of(markup, host.clone());
    laid(&mut screen, width, height);
    screen
}

fn k(key: Key) -> KeyInput {
    KeyInput::new(key, Modifiers::NONE)
}

fn cell(screen: &Screen, place: usize, field: &str) -> Rect {
    table(screen).cell_box(place, field).unwrap_or_else(|| panic!("no cell {field} of row {place}"))
}

/// The choices of the cell open now.
fn open_list(screen: &Screen) -> Option<&CellChoices> {
    let id = table(screen).open_choices()?;
    screen.overlays().iter().find(|o| o.id() == id).and_then(|o| o.content.as_any().downcast_ref::<CellChoices>())
}

// -- tables.json --------------------------------------------------------------

fn task_host(tables: &Value) -> Rc<DataHost> {
    let host = DataHost::new();
    host.set("tasks", tables["rows"].clone());
    for column in tables["columns"].as_array().unwrap() {
        let field = column["field"].as_str().unwrap();
        let path = format!("tasks.{field}");
        if let Some(choices) = column["choices"].as_array() {
            let offered: Vec<(Value, &str)> = choices.iter().map(|c| (c[0].clone(), c[1].as_str().unwrap())).collect();
            host.offer(&path, &offered);
        }
        let kind = match column["kind"].as_str().unwrap() {
            "number" => "decimal",
            other => other,
        };
        host.types.borrow_mut().insert(path, kind.to_string());
    }
    host
}

#[test]
fn every_case_of_edited_cells_behaves_as_on_every_stack() {
    let tables = case_file("tables.json");
    let cases = tables["cases"].as_array().unwrap();
    assert!(!cases.is_empty());
    let columns = tables["columns"].as_array().unwrap();
    let names = |only: &dyn Fn(&Value) -> bool| columns.iter().filter(|c| only(c)).map(|c| c["field"].as_str().unwrap().to_string()).collect::<Vec<_>>().join(", ");
    let fields = names(&|_| true);
    let edited = names(&|c| c["edit"].as_bool() == Some(true));
    let switches = names(&|c| c["switch"].as_bool() == Some(true));
    for case in cases {
        let name = case["name"].as_str().unwrap();
        let host = task_host(&tables);
        let markup = format!(r#"<Table at="1,1" span="12" rows="6" name="tasks" bind="tasks" columns="{fields}" edit="{edited}" switches="{switches}" onselect="picked" onchange="changed" />"#);
        let mut screen = made(&markup, &host, 1200, 400);
        for step in case["steps"].as_array().unwrap() {
            let what = step[0].as_str().unwrap();
            let row = || step[1].as_u64().unwrap() as usize;
            let field = || step[2].as_str().unwrap().to_string();
            match what {
                "click" => {
                    let b = cell(&screen, row(), "notes");
                    click(&mut screen, b)
                }
                "tick" => {
                    let b = cell(&screen, row(), &field());
                    click(&mut screen, b)
                }
                "pick" => {
                    let b = cell(&screen, row(), &field());
                    click(&mut screen, b);
                    laid(&mut screen, 1200, 400);
                    let list = open_list(&screen).unwrap_or_else(|| panic!("{name}: no choices open"));
                    let at = list.choices().iter().position(|c| clarity_ui::parts::data::rows::same(&c.value, &step[3])).unwrap();
                    let item = list.item_box(at);
                    click(&mut screen, item);
                    assert!(table(&screen).open_choices().is_none(), "{name}: the choices stay open");
                }
                "pickDay" => {
                    let day = NaiveDate::parse_from_str(step[3].as_str().unwrap(), "%Y-%m-%d").unwrap();
                    act(&mut screen, "tasks", |t| t.pick_day(row(), &field(), day));
                }
                "type" => {
                    let c = cell(&screen, row(), &field());
                    // A moment's time is typed at its right, beside its day.
                    click_at(&mut screen, c.right() - 12.0, c.y + c.height / 2.0, 1);
                    screen.text_typed(step[3].as_str().unwrap());
                }
                "enter" => {
                    screen.key_pressed(k(Key::Enter));
                }
                "leave" => screen.focus(None),
                "handler" => host.set(&format!("tasks[{}].{}", row(), field()), step[3].clone()),
                "reload" => host.reload("tasks", "tasks", tables["rows"].clone()),
                other => panic!("{name}: unknown step {other}"),
            }
            laid(&mut screen, 1200, 400);
        }
        let expect = &case["expect"];
        let wanted: Vec<String> = expect["events"].as_array().unwrap().iter().map(|e| e.as_str().unwrap().to_string()).collect();
        assert_eq!(wanted, host.events_at(), "{name}: the events");
        // Every value as it started, but those the case changes.
        let start = tables["rows"].as_array().unwrap();
        let changed: Vec<&Value> = expect.get("values").and_then(Value::as_array).map(|v| v.iter().collect()).unwrap_or_default();
        for column in columns {
            let field = column["field"].as_str().unwrap();
            for (row, record) in start.iter().enumerate() {
                let wanted = changed.iter().find(|v| v[0] == json!(row) && v[1] == json!(field)).map(|v| v[2].clone()).unwrap_or_else(|| record[field].clone());
                let now = host.get(&format!("tasks[{row}].{field}"));
                assert_eq!(wanted, now, "{name}: {field} of row {row}");
            }
        }
        if let Some(marked) = expect.get("marked").and_then(Value::as_array) {
            for m in marked {
                assert!(table(&screen).marked(m[0].as_u64().unwrap() as usize, m[1].as_str().unwrap()), "{name}: {} of row {} is not marked", m[1], m[0]);
            }
        }
        if let Some(shown) = expect.get("shown").and_then(Value::as_array) {
            for s in shown {
                assert_eq!(s[2].as_str().unwrap(), table(&screen).shown(s[0].as_u64().unwrap() as usize, s[1].as_str().unwrap()), "{name}: {} of row {}", s[1], s[0]);
            }
        }
        if let Some(chosen) = expect.get("chosen") {
            assert_eq!(chosen.as_u64().map(|c| c as usize), table(&screen).chosen(), "{name}: the chosen row");
        }
        if let Some(choices) = expect.get("choices").and_then(Value::as_object) {
            for (field, labels) in choices {
                let column = table(&screen).columns().iter().find(|c| &c.field == field).unwrap();
                let got: Vec<&str> = column.choices.iter().map(|c| c.label.as_str()).collect();
                let wanted: Vec<&str> = labels.as_array().unwrap().iter().map(|l| l.as_str().unwrap()).collect();
                assert_eq!(wanted, got, "{name}: the choices of {field}");
            }
        }
        // Drawing it is no edit either.
        let before = host.events().len();
        laid(&mut screen, 1200, 400);
        assert_eq!(before, host.events().len());
    }
}

#[test]
fn the_columns_kinds_come_from_the_types_and_the_values() {
    let host = task_host(&case_file("tables.json"));
    let screen = made(r#"<Table at="1,1" span="12" rows="6" name="tasks" bind="tasks" columns="title, done, priority, due, day, hours" />"#, &host, 1200, 400);
    let kinds: Vec<CellKind> = table(&screen).columns().iter().map(|c| c.kind).collect();
    assert_eq!(vec![CellKind::Text, CellKind::Bool, CellKind::Choice, CellKind::Moment, CellKind::Date, CellKind::Number], kinds);
    // Without a type, a value says what it is.
    host.types.borrow_mut().clear();
    host.change();
    let screen = made(r#"<Table at="1,1" span="12" rows="6" name="tasks" bind="tasks" columns="done, due, day" />"#, &host, 1200, 400);
    let kinds: Vec<CellKind> = table(&screen).columns().iter().map(|c| c.kind).collect();
    assert_eq!(vec![CellKind::Bool, CellKind::Moment, CellKind::Date], kinds);
}

// -- table-order.json -----------------------------------------------------------

fn order_host(order: &Value) -> Rc<DataHost> {
    let host = DataHost::new();
    host.set("orders", order["rows"].clone());
    let owner = order["columns"].as_array().unwrap().iter().find(|c| c["field"] == "owner").unwrap();
    let offered: Vec<(Value, &str)> = owner["choices"].as_array().unwrap().iter().map(|c| (c[0].clone(), c[1].as_str().unwrap())).collect();
    host.offer("orders.owner", &offered);
    host.types.borrow_mut().insert("orders.hours".into(), "decimal".into());
    host
}

#[test]
fn every_order_case_behaves_as_on_every_stack() {
    let order = case_file("table-order.json");
    let cases = order["cases"].as_array().unwrap();
    assert!(!cases.is_empty());
    for case in cases {
        let name = case["name"].as_str().unwrap();
        let host = order_host(&order);
        let sortable = if case["sortable"].as_bool().unwrap() { r#"sortable="true""# } else { "" };
        let ordered = match &case["order"] {
            Value::Object(o) => format!("order='{}'", Value::Object(o.clone())),
            _ => String::new(),
        };
        let markup = format!(r#"<Table at="1,1" span="12" rows="6" name="orders" bind="orders" columns="title, hours, owner" {sortable} {ordered} onselect="picked" />"#);
        let mut screen = made(&markup, &host, 800, 400);
        for step in case["steps"].as_array().unwrap() {
            match step[0].as_str().unwrap() {
                "sort" => {
                    let b = table(&screen).header_box(step[1].as_str().unwrap()).unwrap();
                    click(&mut screen, b)
                }
                "click" => {
                    let place = table(&screen).shown_places()[step[1].as_u64().unwrap() as usize];
                    let b = cell(&screen, place, "title");
                    click(&mut screen, b);
                }
                other => panic!("{name}: unknown step {other}"),
            }
            laid(&mut screen, 800, 400);
        }
        let expect = &case["expect"];
        let shown: Vec<usize> = expect["shown"].as_array().unwrap().iter().map(|s| s.as_u64().unwrap() as usize).collect();
        assert_eq!(shown, table(&screen).shown_places(), "{name}: the rows shown");
        let wanted: Vec<String> = expect["events"].as_array().unwrap().iter().map(|e| e.as_str().unwrap().to_string()).collect();
        assert_eq!(wanted, host.events_at(), "{name}: the events");
        if let Some(chosen) = expect.get("chosen") {
            assert_eq!(chosen.as_u64().map(|c| c as usize), table(&screen).chosen(), "{name}: the chosen row");
        }
    }
}

#[test]
fn the_person_drags_a_header_edge_and_a_double_click_gives_the_designed_widths() {
    let order = case_file("table-order.json");
    let host = order_host(&order);
    let mut screen = made(r#"<Table at="1,1" span="12" rows="6" name="orders" bind="orders" columns="title 150, hours" resizable="true" />"#, &host, 800, 400);
    let title = table(&screen).header_box("title").unwrap();
    assert_eq!(150.0, title.width);
    screen.pointer_pressed(title.right() - 2.0, title.y + 10.0, PointerButton::Left, 1);
    screen.pointer_moved(title.right() + 48.0, title.y + 10.0);
    screen.pointer_released(title.right() + 48.0, title.y + 10.0, PointerButton::Left);
    assert_eq!(200.0, table(&screen).header_box("title").unwrap().width);
    assert_eq!(200.0, cell(&screen, 0, "title").width);
    let edge = table(&screen).header_box("title").unwrap();
    screen.pointer_pressed(edge.right() - 2.0, edge.y + 10.0, PointerButton::Left, 2);
    screen.pointer_released(edge.right() - 2.0, edge.y + 10.0, PointerButton::Left);
    assert_eq!(150.0, table(&screen).header_box("title").unwrap().width);
}

#[test]
fn the_header_is_drawn_in_the_header_look() {
    let order = case_file("table-order.json");
    let host = order_host(&order);
    let mut screen = made(r#"<Table at="1,1" span="12" rows="6" name="orders" bind="orders" columns="title, hours" />"#, &host, 800, 400);
    let look = screen.look().clone();
    let mut shot = laid(&mut screen, 800, 400);
    let header = table(&screen).header_box("hours").unwrap();
    assert_eq!(look.part("header", None).background.unwrap_or(look.color("raised")), shot.pixel(header.right() as i32 - 6, header.y as i32 + 4));
    host.choose("orders", Some(1));
    let mut chosen = laid(&mut screen, 800, 400);
    let row = table(&screen).row_box(1).unwrap();
    assert_eq!(look.part("list", Some("selected")).background.unwrap_or(look.color("selection")), chosen.pixel(row.right() as i32 - 6, (row.y + row.height / 2.0) as i32));
    if let Some(path) = shot_path("data-table-header.png") {
        chosen.save(path).unwrap();
    }
}

// -- table-pages.json -----------------------------------------------------------

fn albums(count: usize, from: usize) -> Value {
    Value::Array((from..from + count).map(|i| json!({ "title": format!("Album {i:03}") })).collect())
}

#[test]
fn a_page_shows_the_rows_every_stack_shows() {
    let pages = case_file("table-pages.json");
    for c in pages["pages"].as_array().unwrap() {
        let host = DataHost::new();
        host.set("albums", albums(c["count"].as_u64().unwrap() as usize, 1));
        let markup = format!(r#"<Table at="1,1" span="12" rows="6" name="albums" bind="albums" columns="title" page="{}" />"#, c["size"]);
        let mut screen = made(&markup, &host, 800, 2000);
        act(&mut screen, "albums", |t| t.turn_to(c["page"].as_u64().unwrap() as usize));
        let paged = table(&screen).paged().unwrap();
        assert_eq!(
            (c["shows"].as_u64().unwrap() as usize, c["pages"].as_u64().unwrap() as usize, c["from"].as_u64().unwrap() as usize, c["to"].as_u64().unwrap() as usize, c["text"].as_str().unwrap().to_string()),
            (paged.page, paged.pages, paged.from, paged.to, paged.text())
        );
        assert_eq!(if paged.from == 0 { 0 } else { paged.to - paged.from + 1 }, table(&screen).shown_places().len());
        // First and previous work past the first page, next and last before the last.
        assert_eq!(paged.page > 1, table(&screen).page_button_works(PageButton::First));
        assert_eq!(paged.page < paged.pages, table(&screen).page_button_works(PageButton::Last));
    }
    for c in pages["selected"].as_array().unwrap() {
        let host = DataHost::new();
        let at = c["at"].as_u64().unwrap() as usize;
        host.set("albums", albums(at + 1, 0));
        let markup = format!(r#"<Table at="1,1" span="12" rows="6" name="albums" bind="albums" columns="title" page="{}" />"#, c["size"]);
        let mut screen = made(&markup, &host, 800, 400);
        host.choose("albums", Some(at));
        laid(&mut screen, 800, 400);
        assert_eq!(c["page"].as_u64().unwrap() as usize, table(&screen).paged().unwrap().page);
    }
}

#[test]
fn the_bar_turns_the_pages_and_a_selected_row_shows_its_page() {
    let host = DataHost::new();
    host.set("albums", Value::Array((1..=7).map(|i| json!({ "title": format!("Album {i:02}") })).collect()));
    let mut screen = made(r#"<Table at="1,1" span="12" rows="6" name="albums" bind="albums" columns="title" page="3" sortable="true" />"#, &host, 800, 400);
    let titles = |screen: &Screen| -> Vec<String> {
        let t = table(screen);
        t.shown_places().iter().map(|p| t.records()[*p]["title"].as_str().unwrap().to_string()).collect()
    };
    assert_eq!(vec!["Album 01", "Album 02", "Album 03"], titles(&screen));
    assert_eq!("1-3 / 7", table(&screen).paged().unwrap().text());
    assert!(!table(&screen).page_button_works(PageButton::First));
    assert_eq!("Next page", TablePart::page_word(PageButton::Next));
    let b = table(&screen).pager_box(PageButton::Next).unwrap();
    click(&mut screen, b);
    assert_eq!(vec!["Album 04", "Album 05", "Album 06"], titles(&screen));
    laid(&mut screen, 800, 400);
    let b = table(&screen).pager_box(PageButton::Last).unwrap();
    click(&mut screen, b);
    assert_eq!(vec!["Album 07"], titles(&screen));
    assert!(!table(&screen).page_button_works(PageButton::Next));
    // Rows that come again keep the page, within the pages there are.
    let mut rows = host.get("albums").as_array().unwrap().clone();
    rows.pop();
    host.set("albums", Value::Array(rows));
    laid(&mut screen, 800, 400);
    assert_eq!("4-6 / 6", table(&screen).paged().unwrap().text());
    // A sort shows the first page.
    let b = table(&screen).header_box("title").unwrap();
    click(&mut screen, b);
    assert_eq!("1-3 / 6", table(&screen).paged().unwrap().text());
    // A row selected by code shows its page.
    host.choose("albums", Some(4));
    laid(&mut screen, 800, 400);
    assert_eq!(vec!["Album 04", "Album 05", "Album 06"], titles(&screen));
    let mut shot = laid(&mut screen, 800, 400);
    assert!(table(&screen).pager_box(PageButton::First).is_some());
    if let Some(path) = shot_path("data-table-pages.png") {
        shot.save(path).unwrap();
    }
}

#[test]
fn only_the_rows_in_sight_are_drawn_and_the_rows_scroll_under_the_header() {
    let host = DataHost::new();
    host.set("albums", Value::Array((1..=5000).map(|i| json!({ "title": format!("Album {i:04}") })).collect()));
    let mut screen = made(r#"<Table at="1,1" span="12" rows="6" name="albums" bind="albums" columns="title" />"#, &host, 800, 400);
    assert!(table(&screen).row_box(0).is_some());
    assert!(table(&screen).row_box(100).is_none());
    let header = table(&screen).header_box("title").unwrap();
    screen.wheel(header.x + 20.0, header.bottom() + 20.0, 0.0, 1000.0, Modifiers::NONE);
    assert!(table(&screen).row_box(0).is_none());
    assert_eq!(Some(header), table(&screen).header_box("title"));
    let id = part_named(&screen, "albums");
    screen.focus(Some(id));
    screen.key_pressed(k(Key::End));
    assert_eq!(Some(4999), table(&screen).chosen());
    assert!(table(&screen).row_box(4999).is_some());
    laid(&mut screen, 800, 400);
}

#[test]
fn the_rows_scroll_with_the_bar() {
    let host = DataHost::new();
    host.set("albums", albums(500, 1));
    let mut screen = made(r#"<Table at="1,1" span="12" rows="6" name="albums" bind="albums" columns="title" />"#, &host, 800, 400);
    let view = table(&screen).scroller().bounds;
    // The rows scroll under the header: their place is the body.
    assert_eq!(table(&screen).header_box("title").unwrap().bottom(), view.y);
    let x = view.right() - 3.0;
    // A press on the track below the thumb shows the next page.
    screen.pointer_pressed(x, view.bottom() - 3.0, PointerButton::Left, 1);
    screen.pointer_released(x, view.bottom() - 3.0, PointerButton::Left);
    assert!((view.height - table(&screen).scroller().scrolled()).abs() < 3.0);
    assert!(table(&screen).row_box(0).is_none());
    // The wheel over the header scrolls the rows too; a turn up shows what is above.
    let header = table(&screen).header_box("title").unwrap();
    screen.wheel(header.x + 20.0, header.y + 5.0, 0.0, -10000.0, Modifiers::NONE);
    assert_eq!(0.0, table(&screen).scroller().scrolled());
}

// -- Typing into cells ------------------------------------------------------------

fn album_table() -> (Screen, Rc<DataHost>) {
    let host = DataHost::new();
    host.set("albums", json!([{ "title": "Blue" }, { "title": "Kind of Blue" }]));
    let screen = made(r#"<Table at="1,1" span="12" rows="6" name="albums" bind="albums" columns="title" edit="title" onselect="picked" onchange="changed" />"#, &host, 800, 400);
    (screen, host)
}

#[test]
fn a_cell_takes_caret_moves_a_selection_and_pasting() {
    let (mut screen, host) = album_table();
    let b = cell(&screen, 0, "title");
    click(&mut screen, b);
    // All of it is selected, so what is typed first replaces it.
    assert_eq!("Blue", table(&screen).cell_editor().unwrap().selected_text());
    screen.key_pressed(k(Key::End));
    screen.key_pressed(KeyInput::new(Key::Left, Modifiers::SHIFT));
    screen.key_pressed(KeyInput::new(Key::Left, Modifiers::SHIFT));
    assert_eq!("ue", table(&screen).cell_editor().unwrap().selected_text());
    screen.set_clipboard(Box::new(clarity_ui::overlay::OwnClipboard::default()));
    let id = part_named(&screen, "albums");
    screen.act(id, |_, cx| cx.clipboard().set_text("ack"));
    screen.key_pressed(KeyInput::new(Key::V, Modifiers::CONTROL));
    assert_eq!("Black", table(&screen).shown(0, "title"));
    assert_eq!(json!("Blue"), host.get("albums[0].title"));
    // Enter takes it, and the table keeps the focus.
    screen.key_pressed(k(Key::Enter));
    assert_eq!(json!("Black"), host.get("albums[0].title"));
    assert_eq!(Some(id), screen.focused());
    assert!(table(&screen).cell_editor().is_none());
    assert_eq!(vec!["select", "change"], host.events());
    let mut shot = laid(&mut screen, 800, 400);
    if let Some(path) = shot_path("data-table-edited.png") {
        shot.save(path).unwrap();
    }
}

#[test]
fn escape_leaves_the_cell_and_its_value_stays() {
    let (mut screen, host) = album_table();
    let b = cell(&screen, 1, "title");
    click(&mut screen, b);
    screen.text_typed("Red");
    assert_eq!("Red", table(&screen).shown(1, "title"));
    screen.key_pressed(k(Key::Escape));
    assert_eq!("Kind of Blue", table(&screen).shown(1, "title"));
    assert_eq!(Some(part_named(&screen, "albums")), screen.focused());
    screen.focus(None);
    assert_eq!(json!("Kind of Blue"), host.get("albums[1].title"));
    assert!(host.events().is_empty());
}

#[test]
fn a_press_in_the_cell_typed_into_places_the_caret_and_tab_takes_the_text() {
    let (mut screen, host) = album_table();
    let c = cell(&screen, 0, "title");
    click(&mut screen, c);
    laid(&mut screen, 800, 400);
    click_at(&mut screen, c.x + 6.0, c.y + c.height / 2.0, 1);
    assert_eq!((0, 0), table(&screen).cell_editor().unwrap().selection());
    screen.text_typed("True ");
    screen.key_pressed(k(Key::Tab));
    assert_eq!(json!("True Blue"), host.get("albums[0].title"));
    // The next row's cell is typed into now.
    assert_eq!("Kind of Blue", table(&screen).cell_editor().unwrap().text());
}

fn relations() -> (Screen, Rc<DataHost>) {
    let host = DataHost::new();
    host.set("relations", json!([{ "multiplicity": "1", "label": "" }, { "multiplicity": "2", "label": "" }]));
    let screen = made(r#"<Table at="1,1" span="12" rows="6" name="grid" bind="relations" columns="multiplicity, label" edit="multiplicity, label" /><Textbox at="1,7" span="4" name="after" />"#, &host, 800, 400);
    (screen, host)
}

#[test]
fn tab_takes_the_text_and_types_into_the_next_cell() {
    let (mut screen, host) = relations();
    let b = cell(&screen, 0, "multiplicity");
    click(&mut screen, b);
    screen.text_typed("1..*");
    screen.key_pressed(k(Key::Tab));
    assert_eq!(json!("1..*"), host.get("relations[0].multiplicity"));
    assert!(table(&screen).cell_editor().is_some());
    screen.text_typed("has");
    screen.key_pressed(k(Key::Enter));
    assert_eq!(json!("has"), host.get("relations[0].label"));
}

#[test]
fn tab_from_the_last_cell_of_a_row_goes_to_the_next_row_and_shift_tab_comes_back() {
    let (mut screen, host) = relations();
    let b = cell(&screen, 0, "label");
    click(&mut screen, b);
    screen.text_typed("owns");
    screen.key_pressed(k(Key::Tab));
    assert_eq!(json!("owns"), host.get("relations[0].label"));
    assert_eq!("2", table(&screen).cell_editor().unwrap().text());
    screen.key_pressed(KeyInput::new(Key::Tab, Modifiers::SHIFT));
    assert_eq!("owns", table(&screen).cell_editor().unwrap().text());
}

#[test]
fn tab_from_the_last_cell_leaves_the_table() {
    let (mut screen, host) = relations();
    let b = cell(&screen, 1, "label");
    click(&mut screen, b);
    screen.text_typed("ends");
    screen.key_pressed(k(Key::Tab));
    assert_eq!(json!("ends"), host.get("relations[1].label"));
    assert!(table(&screen).cell_editor().is_none());
    assert_eq!(Some(part_named(&screen, "after")), screen.focused());
}

// -- Pressing in a cell's margin ------------------------------------------------------

fn tasks() -> (Screen, Rc<DataHost>) {
    let host = DataHost::new();
    host.set("tasks", json!([{ "title": "Milk", "done": false }, { "title": "Bread", "done": false }]));
    let screen = made(r#"<Table at="1,1" span="12" rows="6" name="tasks" bind="tasks" columns="title, done" edit="title, done" switches="done" />"#, &host, 800, 400);
    (screen, host)
}

#[test]
fn a_press_beside_a_switch_chooses_the_row_and_leaves_the_value() {
    let (mut screen, host) = tasks();
    let c = cell(&screen, 1, "done");
    click_at(&mut screen, c.x + 2.0, c.y + c.height / 2.0, 1);
    assert_eq!(Some(1), table(&screen).chosen());
    assert_eq!(json!(false), host.get("tasks[1].done"));
}

#[test]
fn a_press_on_the_switch_turns_it() {
    let (mut screen, host) = tasks();
    let b = cell(&screen, 1, "done");
    click(&mut screen, b);
    assert_eq!(json!(true), host.get("tasks[1].done"));
}

#[test]
fn a_press_in_the_margin_of_a_typed_cell_chooses_the_row_without_typing() {
    let (mut screen, _) = tasks();
    let c = cell(&screen, 1, "title");
    click_at(&mut screen, c.x + 1.0, c.y + c.height / 2.0, 1);
    assert_eq!(Some(1), table(&screen).chosen());
    assert!(table(&screen).cell_editor().is_none());
    click(&mut screen, c);
    assert!(table(&screen).cell_editor().is_some());
}

// -- Color cells ------------------------------------------------------------------

fn people() -> Rc<DataHost> {
    let host = DataHost::new();
    host.set("people", json!([{ "name": "Ann", "color": "#ff0000" }, { "name": "Bob", "color": "" }]));
    host
}

fn color_facts(host: &DataHost) {
    host.fields.borrow_mut().insert("people.color".into(), FieldFacts { label: "Color".into(), color: true, ..FieldFacts::default() });
}

#[test]
fn a_column_of_a_color_field_shows_a_swatch_of_it() {
    let host = people();
    color_facts(&host);
    let mut screen = made(r#"<Table at="1,1" span="12" rows="6" name="peopleGrid" bind="people" columns="name, color" />"#, &host, 800, 400);
    assert_eq!(CellKind::Color, table(&screen).columns()[1].kind);
    assert_eq!("Color", table(&screen).columns()[1].label);
    assert_eq!("#ff0000", table(&screen).shown(0, "color"));
    let mut shot = laid(&mut screen, 800, 400);
    let swatch = table(&screen).swatch_box(0, "color").unwrap();
    assert_eq!(skia_safe::Color::from_rgb(255, 0, 0), shot.pixel((swatch.x + swatch.width / 2.0) as i32, (swatch.y + swatch.height / 2.0) as i32));
    // A cell without a color has no swatch.
    assert!(table(&screen).swatch_box(1, "color").is_none());
}

#[test]
fn an_edited_color_cell_shows_its_swatch_and_takes_typed_text() {
    let host = people();
    color_facts(&host);
    let mut screen = made(r#"<Table at="1,1" span="12" rows="6" name="peopleGrid" bind="people" columns="name, color" edit="color" />"#, &host, 800, 400);
    let mut shot = laid(&mut screen, 800, 400);
    let swatch = table(&screen).swatch_box(0, "color").unwrap();
    assert_eq!(skia_safe::Color::from_rgb(255, 0, 0), shot.pixel((swatch.x + swatch.width / 2.0) as i32, (swatch.y + swatch.height / 2.0) as i32));
    let b = cell(&screen, 1, "color");
    click(&mut screen, b);
    screen.text_typed("#00ff00");
    screen.key_pressed(k(Key::Enter));
    assert_eq!(json!("#00ff00"), host.get("people[1].color"));
}

#[test]
fn a_column_of_a_field_that_is_no_color_stays_text() {
    let host = people();
    let screen = made(r#"<Table at="1,1" span="12" rows="6" name="peopleGrid" bind="people" columns="name, color" />"#, &host, 800, 400);
    assert_eq!(CellKind::Text, table(&screen).columns()[1].kind);
    assert!(table(&screen).swatch_box(0, "color").is_none());
}

// -- Choice cells -------------------------------------------------------------------

fn priorities(value: &str) -> (Screen, Rc<DataHost>) {
    let host = DataHost::new();
    host.set("tasks", json!([{ "title": "Write", "priority": value }]));
    let offered: Vec<(Value, String)> = (0..12).map(|i| (json!(format!("P{i}")), format!("Priority {i}"))).collect();
    let offered: Vec<(Value, &str)> = offered.iter().map(|(v, l)| (v.clone(), l.as_str())).collect();
    host.offer("tasks.priority", &offered);
    let screen = made(r#"<Table name="tasks" at="1,1" span="12" rows="8" bind="tasks" columns="title, priority" edit="priority" />"#, &host, 800, 600);
    (screen, host)
}

#[test]
fn a_choice_cell_with_more_choices_than_show_scrolls_to_the_last() {
    let (mut screen, host) = priorities("P0");
    let b = cell(&screen, 0, "priority");
    click(&mut screen, b);
    laid(&mut screen, 800, 600);
    let list = open_list(&screen).unwrap();
    let item = list.item_box(0).height;
    assert_eq!(CellChoices::height_for(12, item), clarity_ui::element::Element::node(list).bounds.height);
    // The wheel over the list scrolls it, and the last choice can be picked.
    let b = clarity_ui::element::Element::node(list).bounds;
    screen.wheel(b.x + 20.0, b.y + 20.0, 0.0, 1000.0, Modifiers::NONE);
    let list = open_list(&screen).unwrap();
    assert!(list.scroller().scrolled() > 0.0);
    assert_eq!(list.scroller().max(), list.scroller().scrolled());
    let last = list.item_box(11);
    assert!(last.bottom() <= b.bottom() + 0.5);
    let mut shot = laid(&mut screen, 800, 600);
    if let Some(path) = shot_path("data-table-choices.png") {
        shot.save(path).unwrap();
    }
    click(&mut screen, last);
    assert!(table(&screen).open_choices().is_none());
    assert_eq!(json!("P11"), host.get("tasks[0].priority"));
}

#[test]
fn a_choice_cell_opens_at_its_value_marked() {
    let (mut screen, _) = priorities("P10");
    let b = cell(&screen, 0, "priority");
    click(&mut screen, b);
    laid(&mut screen, 800, 600);
    let list = open_list(&screen).unwrap();
    // The value's choice is in view when the list opens, without scrolling to it.
    assert!(list.scroller().scrolled() > 0.0);
    assert_eq!(Some(10), list.marked());
    let shown = list.scroller().bounds;
    let at = list.item_box(10);
    assert!(at.y >= shown.y - 0.5 && at.bottom() <= shown.bottom() + 0.5);
}

// -- Date cells -------------------------------------------------------------------

#[test]
fn a_date_cell_opens_the_calendar_and_a_picked_day_is_its_edit() {
    use clarity_ui::parts::inputs::calendar::Day;
    let host = DataHost::new();
    host.set("tasks", json!([{ "title": "Write", "day": "2026-10-05", "due": "2026-10-05T09:30:00" }]));
    host.types.borrow_mut().insert("tasks.day".into(), "date".into());
    host.types.borrow_mut().insert("tasks.due".into(), "datetime".into());
    let mut screen = made(r#"<Table name="tasks" at="1,1" span="12" rows="8" bind="tasks" columns="title, day, due" edit="day, due" />"#, &host, 800, 600);
    let b = table(&screen).calendar_button_box(0, "day").expect("a date cell has its calendar button");
    click(&mut screen, b);
    laid(&mut screen, 800, 600);
    assert_eq!(1, screen.overlays().len());
    let calendar = table(&screen).open_calendar().unwrap().clone();
    assert_eq!(Day::new(2026, 10, 5), calendar.chosen());
    let mut shot = laid(&mut screen, 800, 600);
    if let Some(path) = shot_path("data-table-calendar.png") {
        shot.save(path).unwrap();
    }
    click(&mut screen, calendar.cell_of(Day::new(2026, 10, 20).unwrap()).unwrap());
    assert!(screen.overlays().is_empty());
    assert_eq!(json!("2026-10-20"), host.get("tasks[0].day"));
    // A moment's day is picked the same way, and keeps its time.
    let due = cell(&screen, 0, "due");
    click_at(&mut screen, due.x + 8.0, due.y + due.height / 2.0, 1);
    laid(&mut screen, 800, 600);
    let calendar = table(&screen).open_calendar().unwrap().clone();
    click(&mut screen, calendar.cell_of(Day::new(2026, 10, 21).unwrap()).unwrap());
    assert_eq!(json!("2026-10-21T09:30:00"), host.get("tasks[0].due"));
}

#[test]
fn a_color_cell_reads_its_color_as_the_color_input_does() {
    // C# Palette.Read: three digits, any case, without '#' are a color too.
    let host = DataHost::new();
    host.set("people", json!([{ "name": "Ann", "color": "F00" }, { "name": "Bob", "color": "#00F" }, { "name": "Cy", "color": "blue" }]));
    color_facts(&host);
    let mut screen = made(r#"<Table at="1,1" span="12" rows="6" name="peopleGrid" bind="people" columns="name, color" />"#, &host, 800, 400);
    let mut shot = laid(&mut screen, 800, 400);
    let swatch = table(&screen).swatch_box(0, "color").expect("F00 is red");
    assert_eq!(skia_safe::Color::from_rgb(255, 0, 0), shot.pixel((swatch.x + swatch.width / 2.0) as i32, (swatch.y + swatch.height / 2.0) as i32));
    let swatch = table(&screen).swatch_box(1, "color").expect("#00F is blue");
    assert_eq!(skia_safe::Color::from_rgb(0, 0, 255), shot.pixel((swatch.x + swatch.width / 2.0) as i32, (swatch.y + swatch.height / 2.0) as i32));
    // A name is no color: the palette has no names.
    assert!(table(&screen).swatch_box(2, "color").is_none());
}
