//! A folder column (its field a `folder`) is edited in its cell: the path is
//! typed, and a button in the cell opens the system's folder picker, the
//! chosen folder being the cell's edit; where nothing picks folders (a
//! browser) the path is only typed.

use super::data_rows_host;
use super::inputs_test_picker::TestPicker;

use clarity_ui::element::find_type;
use clarity_ui::input::PointerButton;
use clarity_ui::parts::data::table::TablePart;
use clarity_ui::parts::FieldFacts;
use clarity_ui::screen::Screen;
use clarity_ui::testing::window_of;
use data_rows_host::{laid, DataHost};
use serde_json::json;

fn table(screen: &Screen) -> &TablePart {
    find_type::<TablePart>(screen.root(), |_| true).expect("a table")
}

fn made() -> (Screen, std::rc::Rc<DataHost>) {
    let host = DataHost::new();
    host.set("categories", json!([{ "id": 1, "name": "Manual", "folder": "" }]));
    host.fields.borrow_mut().insert("categories.folder".into(), FieldFacts { label: "Folder".into(), folder: true, ..FieldFacts::default() });
    let mut screen = window_of(r#"<Table at="1,1" span="12" rows="6" name="categoryList" bind="categories" columns="name, folder" edit="name, folder" onchange="categoryChanged" />"#, host.clone());
    laid(&mut screen, 800, 400);
    (screen, host)
}

#[test]
fn a_folder_cell_takes_the_folder_its_button_chooses() {
    let picker = TestPicker::installed();
    let (mut screen, host) = made();
    picker.next_folder("/home/ada/manual");
    let button = table(&screen).folder_button_box(0, "folder").expect("the cell's button");
    screen.pointer_pressed(button.x + 4.0, button.y + 4.0, PointerButton::Left, 1);
    screen.pointer_released(button.x + 4.0, button.y + 4.0, PointerButton::Left);
    screen.finish_later();
    laid(&mut screen, 800, 400);
    assert_eq!(json!("/home/ada/manual"), host.get("categories[0].folder"));
    assert_eq!(1, picker.asked().len(), "{:?}", picker.asked());
}

#[test]
fn where_nothing_picks_folders_the_path_is_typed_only() {
    let picker = TestPicker::installed();
    picker.no_folders();
    let (screen, _) = made();
    assert!(table(&screen).folder_button_box(0, "folder").is_none());
}
