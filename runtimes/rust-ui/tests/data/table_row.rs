//! What a table's rows do under the pointer, beside its cases (C#
//! TableRowTests): a double click on a row opens it (not on the header), a
//! line runs under every row, a press on a toggle or an input starts no
//! drag, a row moved off its page shows on its new page, and a list the host
//! keeps as a plain list shows its new order. And an edited color cell
//! offers the palette the color part has.

use super::data_rows_host;

use std::rc::Rc;

use clarity_ui::element::find_type;
use clarity_ui::input::{Key, KeyInput, Modifiers, PointerButton};
use clarity_ui::parts::data::list::ListPart;
use clarity_ui::parts::data::table::TablePart;
use clarity_ui::parts::inputs::color::PaletteGrid;
use clarity_ui::parts::FieldFacts;
use clarity_ui::screen::Screen;
use clarity_ui::testing::{part_named, window_of};
use data_rows_host::{click, laid, DataHost};
use serde_json::{json, Value};

fn table(screen: &Screen) -> &TablePart {
    find_type::<TablePart>(screen.root(), |_| true).expect("a table")
}

fn folders(count: usize) -> Value {
    Value::Array((1..=count).map(|i| json!({ "id": i, "name": format!("Folder {i}"), "position": i })).collect())
}

fn made(markup: &str, rows: Value) -> (Screen, Rc<DataHost>) {
    let host = DataHost::new();
    host.set("folders", rows);
    let mut screen = window_of(markup, host.clone());
    laid(&mut screen, 800, 400);
    (screen, host)
}

#[test]
fn a_double_click_on_a_row_opens_it_and_on_the_header_does_not() {
    let (mut screen, host) = made(r#"<Table at="1,1" span="12" rows="6" name="folders" bind="folders" columns="name, id" onopen="opened" onselect="picked" />"#, folders(3));
    let header = table(&screen).header_box("name").unwrap();
    click(&mut screen, header);
    data_rows_host::click_at(&mut screen, header.x + header.width / 2.0, header.y + header.height / 2.0, 2);
    assert!(host.events().is_empty(), "{:?}", host.events());
    let cell = table(&screen).cell_box(1, "name").unwrap();
    click(&mut screen, cell);
    data_rows_host::click_at(&mut screen, cell.x + cell.width / 2.0, cell.y + cell.height / 2.0, 2);
    assert_eq!(vec!["select", "open"], host.events());
}

#[test]
fn a_right_click_chooses_the_row_under_the_pointer() {
    let (mut screen, host) = made(r#"<Table at="1,1" span="12" rows="6" name="folders" bind="folders" columns="name, id" onselect="picked" />"#, folders(3));
    let cell = table(&screen).cell_box(1, "name").unwrap();
    let (x, y) = (cell.x + cell.width / 2.0, cell.y + cell.height / 2.0);
    screen.pointer_pressed(x, y, PointerButton::Right, 1);
    screen.pointer_released(x, y, PointerButton::Right);
    assert_eq!(Some(1), table(&screen).chosen());
    assert_eq!(vec!["select"], host.events());
}

#[test]
fn a_line_runs_under_every_row() {
    let (mut screen, _) = made(r#"<Table at="1,1" span="12" rows="6" name="folders" bind="folders" columns="name, id" />"#, folders(3));
    let mut shot = laid(&mut screen, 800, 400);
    let border = screen.look().color("border");
    let line = screen.look().border();
    for place in 0..3 {
        let row = table(&screen).row_box(place).unwrap();
        let x = (row.x + row.width / 2.0) as i32;
        assert_eq!(border, shot.pixel(x, (row.bottom() - line).ceil() as i32), "the line under row {place}");
        assert_ne!(border, shot.pixel(x, (row.y + 3.0) as i32), "row {place} itself");
    }
}

#[test]
fn a_press_on_a_toggle_or_an_input_starts_no_drag() {
    let rows = json!([
        { "id": 1, "name": "Documents", "parent": null, "position": 1 },
        { "id": 2, "name": "Invoices", "parent": 1, "position": 2 },
        { "id": 3, "name": "Music", "parent": null, "position": 3 }
    ]);
    let (mut screen, host) = made(r#"<TreeTable at="1,1" span="12" rows="8" name="folders" bind="folders" columns="name, position" edit="position" reorder="position" />"#, rows);
    let before = host.get("folders");
    let drag_from = |screen: &mut Screen, from: clarity_ui::geometry::Rect| {
        let onto = table(screen).row_box(2).unwrap();
        let (x, y) = (from.x + from.width / 2.0, from.y + from.height / 2.0);
        screen.pointer_pressed(x, y, PointerButton::Left, 1);
        screen.pointer_moved(x, y + 10.0);
        screen.pointer_moved(onto.x + 300.0, onto.bottom() - 3.0);
        assert!(table(screen).drop().is_none());
        screen.pointer_released(onto.x + 300.0, onto.bottom() - 3.0, PointerButton::Left);
        laid(screen, 800, 400);
    };
    let toggle = table(&screen).toggle_box(0).unwrap();
    drag_from(&mut screen, toggle);
    let id = part_named(&screen, "folders");
    screen.act(id, |e, _| e.as_any_mut().downcast_mut::<TablePart>().unwrap().toggle(0));
    laid(&mut screen, 800, 400);
    // The number typed into is no row to drag either.
    let number = table(&screen).cell_box(0, "position").unwrap();
    drag_from(&mut screen, number);
    screen.focus(None);
    assert_eq!(before, host.get("folders"));
    assert!(!host.logged().iter().any(|l| l.contains("reorder")), "{:?}", host.logged());
}

#[test]
fn on_a_paged_table_a_row_moved_off_its_page_shows_on_the_next() {
    let (mut screen, host) = made(r#"<Table at="1,1" span="12" rows="6" name="folders" bind="folders" columns="name" page="2" reorder="position" />"#, folders(3));
    host.choose("folders", Some(1));
    laid(&mut screen, 800, 400);
    let id = part_named(&screen, "folders");
    screen.focus(Some(id));
    screen.key_pressed(KeyInput::new(Key::Down, Modifiers::ALT));
    laid(&mut screen, 800, 400);
    let now: Vec<String> = host.get("folders").as_array().unwrap().iter().map(|f| format!("{}:{}", f["name"].as_str().unwrap(), f["position"])).collect();
    assert_eq!(vec!["Folder 1:1", "Folder 3:2", "Folder 2:3"], now);
    assert_eq!(2, table(&screen).paged().unwrap().page);
    assert_eq!(Some(2), table(&screen).chosen());
    assert_eq!("Folder 2", table(&screen).records()[2]["name"]);
    assert!(table(&screen).row_box(2).is_some());
}

#[test]
fn a_list_of_a_plain_list_shows_its_new_order() {
    let host = DataHost::new();
    host.set("folders", folders(3));
    host.quiet.set(true);
    let mut screen = window_of(r#"<List at="1,1" span="12" rows="8" name="folders" bind="folders" show="name" reorder="position" />"#, host.clone());
    laid(&mut screen, 800, 400);
    host.choose("folders", Some(0));
    let id = part_named(&screen, "folders");
    screen.focus(Some(id));
    screen.key_pressed(KeyInput::new(Key::Down, Modifiers::ALT));
    laid(&mut screen, 800, 400);
    let list = find_type::<ListPart>(screen.root(), |_| true).unwrap();
    let names: Vec<&str> = list.records().iter().map(|r| r["name"].as_str().unwrap()).collect();
    assert_eq!(vec!["Folder 2", "Folder 1", "Folder 3"], names);
    assert_eq!(Some(1), list.chosen());
}

fn palette(screen: &Screen) -> Option<&PaletteGrid> {
    let id = table(screen).open_palette_overlay()?;
    screen.overlays().iter().find(|o| o.id() == id).and_then(|o| o.content.as_any().downcast_ref::<PaletteGrid>())
}

#[test]
fn an_edited_color_cell_offers_the_palette_of_the_color_part() {
    let host = DataHost::new();
    host.set("people", json!([{ "name": "Ada", "color": "#ff0000" }]));
    host.fields.borrow_mut().insert("people.color".into(), FieldFacts { label: "Color".into(), color: true, ..FieldFacts::default() });
    let mut screen = window_of(r#"<Table at="1,1" span="12" rows="6" name="people" bind="people" columns="name, color" edit="color" onselect="picked" onchange="changed" />"#, host.clone());
    laid(&mut screen, 800, 400);
    let swatch = table(&screen).swatch_box(0, "color").unwrap();
    click(&mut screen, swatch);
    laid(&mut screen, 800, 400);
    assert_eq!(1, screen.overlays().len());
    let grid = palette(&screen).expect("the palette opens");
    assert_eq!(Some("#ff0000"), grid.chosen());
    assert!(table(&screen).cell_editor().is_none());
    let color = PaletteGrid::colors()[3];
    let place = grid.place(color).unwrap();
    click(&mut screen, place);
    laid(&mut screen, 800, 400);
    assert!(screen.overlays().is_empty());
    assert_eq!(json!(color), host.get("people[0].color"));
    assert_eq!(vec!["select", "change"], host.events());
    // Alt+Down in its typed text opens the palette too.
    let cell = table(&screen).cell_box(0, "color").unwrap();
    click(&mut screen, cell);
    assert!(table(&screen).cell_editor().is_some());
    screen.key_pressed(KeyInput::new(Key::Down, Modifiers::ALT));
    laid(&mut screen, 800, 400);
    assert_eq!(1, screen.overlays().len());
    assert!(palette(&screen).is_some());
}
