//! A table's own Add and trash buttons (`add`, `remove`): the Add button
//! stands above the header at the right and runs `on add`; every row has a
//! trash button in a last column, which picks the row and runs `on remove`.
//! Insert and Delete do the same from the keys; a read-only table runs neither.

use super::data_rows_host;

use std::rc::Rc;

use clarity_ui::element::{find_type, Element};
use clarity_ui::input::{Key, KeyInput, Modifiers};
use clarity_ui::parts::data::table::TablePart;
use clarity_ui::screen::Screen;
use clarity_ui::testing::window_of;
use data_rows_host::{click, laid, DataHost};
use serde_json::{json, Value};

const TABLE: &str = r#"<Table at="1,1" span="12" rows="6" name="folders" bind="folders" columns="name, id" add="true" remove="true" onadd="added" onremove="removed" onselect="picked" />"#;

fn table(screen: &Screen) -> &TablePart {
    find_type::<TablePart>(screen.root(), |_| true).expect("a table")
}

fn folders() -> Value {
    Value::Array((1..=3).map(|i| json!({ "id": i, "name": format!("Folder {i}") })).collect())
}

fn made(markup: &str) -> (Screen, Rc<DataHost>) {
    let host = DataHost::new();
    host.set("folders", folders());
    let mut screen = window_of(markup, host.clone());
    laid(&mut screen, 800, 400);
    (screen, host)
}

#[test]
fn its_add_button_stands_above_the_header_at_the_right_and_runs_on_add() {
    let (mut screen, host) = made(TABLE);
    let t = table(&screen);
    let add = t.add_button().expect("an Add button");
    let header = t.header_box("name").unwrap();
    assert!(add.bottom() <= header.y, "{add:?} above {header:?}");
    assert!((add.right() - t.node().bounds.right()).abs() < 0.5, "{add:?} at the right");
    click(&mut screen, add);
    assert_eq!(vec!["add"], host.events());
}

#[test]
fn a_rows_trash_button_in_the_last_column_picks_the_row_and_runs_on_remove() {
    let (mut screen, host) = made(TABLE);
    let t = table(&screen);
    let trash = t.remove_box(1).expect("a trash button");
    let id = t.cell_box(1, "id").unwrap();
    assert!(trash.x >= id.right() - 0.5, "{trash:?} after {id:?}");
    click(&mut screen, trash);
    assert_eq!(Some(1), table(&screen).chosen());
    assert_eq!(vec!["select", "remove"], host.events());
}

#[test]
fn delete_and_insert_run_them_from_the_keys() {
    let (mut screen, host) = made(TABLE);
    let cell = table(&screen).cell_box(2, "name").unwrap();
    click(&mut screen, cell);
    screen.key_pressed(KeyInput::new(Key::Delete, Modifiers::NONE));
    screen.key_pressed(KeyInput::new(Key::Insert, Modifiers::NONE));
    assert_eq!(vec!["select", "remove", "add"], host.events());
}

#[test]
fn without_them_it_has_neither_and_read_only_it_runs_neither() {
    let (screen, _) = made(r#"<Table at="1,1" span="12" rows="6" name="folders" bind="folders" columns="name, id" />"#);
    assert!(table(&screen).add_button().is_none() && table(&screen).remove_box(0).is_none());
    let (mut screen, host) = made(&TABLE.replace(" add=", " readonly=\"true\" add="));
    let add = table(&screen).add_button().unwrap();
    click(&mut screen, add);
    let trash = table(&screen).remove_box(0).unwrap();
    click(&mut screen, trash);
    assert!(!host.events().contains(&"add".to_string()) && !host.events().contains(&"remove".to_string()), "{:?}", host.events());
}

#[test]
fn with_confirm_it_asks_first_and_removes_on_yes_only() {
    use clarity_ui::dialogs::ask::AskDialog;
    let (mut screen, host) = made(&TABLE.replace(" remove=\"true\"", " remove=\"true\" confirm=\"true\""));
    let trash = table(&screen).remove_box(1).unwrap();
    click(&mut screen, trash);
    assert_eq!(vec!["select"], host.events());
    let no = screen.find::<AskDialog>().expect("it asks").cancel().node().bounds;
    click(&mut screen, no);
    assert!(screen.find::<AskDialog>().is_none());
    assert_eq!(vec!["select"], host.events());
    let trash = table(&screen).remove_box(1).unwrap();
    click(&mut screen, trash);
    let yes = screen.find::<AskDialog>().expect("it asks again").ok().node().bounds;
    click(&mut screen, yes);
    laid(&mut screen, 800, 400);
    assert_eq!(vec!["select", "remove"], host.events());
}
