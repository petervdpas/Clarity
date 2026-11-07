//! A list of records (C# ListPartTests): its rows show the fields of `show`,
//! or as cards; the person chooses a row with the pointer or Up and Down,
//! which runs `select` only when another row is chosen; a long list scrolls
//! in its own place, with the wheel and its bar.

use super::data_rows_host;

use clarity_ui::element::find_type;
use clarity_ui::input::{Key, KeyInput, Modifiers, PointerButton};
use clarity_ui::parts::data::list::ListPart;
use clarity_ui::parts::data::rows::Scroller;
use clarity_ui::screen::Screen;
use clarity_ui::testing::{part_named, window_of};
use data_rows_host::{click, click_at, laid, shot_path, DataHost};
use serde_json::{json, Value};

fn folders(count: usize) -> Value {
    Value::Array((1..=count).map(|i| json!({ "id": i, "name": format!("Folder {i}"), "position": i })).collect())
}

fn made(fields: &str, count: usize) -> (Screen, std::rc::Rc<DataHost>) {
    let host = DataHost::new();
    host.set("folders", folders(count));
    let markup = format!(r#"<List at="1,1" span="6" rows="5" name="folders" bind="folders" show="name, position" onselect="picked" onopen="opened" {fields} />"#);
    let mut screen = window_of(&markup, host.clone());
    laid(&mut screen, 800, 400);
    (screen, host)
}

fn list(screen: &Screen) -> &ListPart {
    find_type::<ListPart>(screen.root(), |_| true).expect("a list")
}

fn row(screen: &Screen, place: usize) -> clarity_ui::geometry::Rect {
    list(screen).row_box(place).unwrap_or_else(|| panic!("row {place} does not show"))
}

fn key(screen: &mut Screen, k: Key) {
    screen.key_pressed(KeyInput::new(k, Modifiers::NONE));
}

#[test]
fn a_row_shows_the_fields_of_show() {
    let (screen, _) = made("", 3);
    let list = list(&screen);
    assert!(clarity_ui::parts::Part::grows(list));
    let shown: Vec<String> = list.records().iter().map(|r| list.text_of(r)).collect();
    assert_eq!(vec!["Folder 1 | 1", "Folder 2 | 2", "Folder 3 | 3"], shown);
}

#[test]
fn choosing_a_row_selects_it_once() {
    let (mut screen, host) = made("", 3);
    let b = row(&screen, 1);
    click(&mut screen, b);
    let b = row(&screen, 1);
    click(&mut screen, b);
    assert_eq!(Some(1), list(&screen).chosen());
    assert_eq!(vec!["run folders select picked".to_string()], host.logged());
    let first = row(&screen, 0);
    click_at(&mut screen, first.x + 20.0, first.y + 5.0, 2);
    assert_eq!(vec!["select", "select", "open"], host.events());
}

#[test]
fn up_and_down_choose_the_row_before_and_after() {
    let (mut screen, host) = made("", 3);
    let id = part_named(&screen, "folders");
    screen.focus(Some(id));
    key(&mut screen, Key::Down);
    assert_eq!(Some(0), list(&screen).chosen());
    key(&mut screen, Key::Down);
    key(&mut screen, Key::Down);
    key(&mut screen, Key::Down);
    assert_eq!(Some(2), list(&screen).chosen());
    key(&mut screen, Key::Up);
    assert_eq!(Some(1), list(&screen).chosen());
    assert_eq!(vec!["select"; 4], host.events());
    key(&mut screen, Key::Enter);
    assert_eq!("open", host.events().last().unwrap());
}

#[test]
fn rows_that_are_fewer_have_no_row_chosen_past_them_and_that_is_no_select() {
    let (mut screen, host) = made("", 3);
    let b = row(&screen, 2);
    click(&mut screen, b);
    // The same records again keep the chosen one.
    host.change();
    laid(&mut screen, 800, 400);
    assert_eq!(Some(2), list(&screen).chosen());
    host.set("folders", folders(1));
    laid(&mut screen, 800, 400);
    assert_eq!(None, list(&screen).chosen());
    // The window holds no chosen row of it either.
    assert_eq!(None, host.selected.borrow().get("folders").copied());
    assert_eq!(vec!["select"], host.events());
}

#[test]
fn rows_read_again_have_no_row_chosen_and_a_row_chosen_by_code_is_no_select() {
    let (mut screen, host) = made("", 3);
    let b = row(&screen, 0);
    click(&mut screen, b);
    assert_eq!(Some(0), host.selected.borrow().get("folders").copied());
    host.reload("folders", "folders", folders(3));
    laid(&mut screen, 800, 400);
    assert_eq!(None, list(&screen).chosen());
    host.choose("folders", Some(2));
    laid(&mut screen, 800, 400);
    assert_eq!(Some(2), list(&screen).chosen());
    assert_eq!(vec!["select"], host.events());
}

#[test]
fn a_list_draws_its_rows_in_the_list_look() {
    let (mut screen, host) = made("", 3);
    host.choose("folders", Some(1));
    let mut shot = laid(&mut screen, 800, 400);
    let look = screen.look().clone();
    let chosen = row(&screen, 1);
    assert_eq!(look.part("list", Some("selected")).background.unwrap_or(look.color("selection")), shot.pixel((chosen.right() - 20.0) as i32, (chosen.y + chosen.height / 2.0) as i32));
    let other = row(&screen, 2);
    assert_eq!(look.part("list", None).background.unwrap_or(look.color("input")), shot.pixel((other.right() - 20.0) as i32, (other.y + other.height / 2.0) as i32));
    // Its border, in the list look.
    let list = list(&screen);
    assert!(list.framed());
    let b = clarity_ui::element::Element::node(list).bounds;
    assert_eq!(look.part("list", None).border.unwrap_or(look.color("strong_border")), shot.pixel((b.x + 30.0) as i32, b.y as i32));
    if let Some(path) = shot_path("data-list-rows.png") {
        shot.save(path).unwrap();
    }
}

#[test]
fn cards_show_each_row_in_the_card_look() {
    let (mut screen, host) = made(r#"cards="true""#, 3);
    host.choose("folders", Some(0));
    let mut shot = laid(&mut screen, 800, 400);
    let look = screen.look().clone();
    let chosen = row(&screen, 0);
    let plain = row(&screen, 1);
    // A card is higher than a row: the title, then its lines.
    assert!(chosen.height > 40.0, "a card is {} high", chosen.height);
    assert_eq!(look.part("card", Some("selected")).background.unwrap(), shot.pixel((chosen.right() - 10.0) as i32, chosen.y as i32 + 6));
    assert_eq!(look.part("card", None).background.unwrap_or(look.color("raised")), shot.pixel((plain.right() - 10.0) as i32, plain.y as i32 + 6));
    // No frame of its own: between the cards, the window.
    assert_eq!(look.color("background"), shot.pixel((plain.right() - 10.0) as i32, plain.y as i32 - 2));
    if let Some(path) = shot_path("data-list-cards.png") {
        shot.save(path).unwrap();
    }
}

#[test]
fn a_long_lists_rows_leave_room_for_its_bar() {
    let (screen, _) = made(r#"cards="true""#, 40);
    let list = list(&screen);
    assert!(list.scroller().max() > 0.0);
    assert!((list.scroller().bounds.width - Scroller::BAR - row(&screen, 0).width).abs() < 0.01);
    let (few, _) = made(r#"cards="true""#, 1);
    assert!((list_of(&few).scroller().bounds.width - row(&few, 0).width).abs() < 0.01);
}

fn list_of(screen: &Screen) -> &ListPart {
    list(screen)
}

#[test]
fn a_long_list_scrolls_in_its_own_place() {
    let (mut screen, _) = made("", 40);
    assert!(list(&screen).row_box(0).is_some());
    assert!(list(&screen).row_box(39).is_none());
    let at = clarity_ui::element::Element::node(list(&screen)).bounds;
    screen.wheel(at.x + 10.0, at.y + 10.0, 0.0, 10000.0, Modifiers::NONE);
    assert!(list(&screen).row_box(0).is_none());
    assert!(list(&screen).row_box(39).is_some());
    // Down to the last row shows it.
    screen.wheel(at.x + 10.0, at.y + 10.0, 0.0, -10000.0, Modifiers::NONE);
    let id = part_named(&screen, "folders");
    screen.focus(Some(id));
    key(&mut screen, Key::End);
    assert_eq!(Some(39), list(&screen).chosen());
    assert!(list(&screen).row_box(39).is_some());
}

#[test]
fn a_long_list_scrolls_with_its_bar() {
    let (mut screen, _) = made("", 40);
    let view = list(&screen).scroller().bounds;
    let x = view.right() - 3.0;
    // A press on the track below the thumb shows the next page.
    screen.pointer_pressed(x, view.bottom() - 3.0, PointerButton::Left, 1);
    screen.pointer_released(x, view.bottom() - 3.0, PointerButton::Left);
    assert!((view.height - list(&screen).scroller().scrolled()).abs() < 3.0);
    assert!(list(&screen).row_box(0).is_none());
    // A row under the pointer is still the list's: a press chooses it.
    let place = (0..40).find(|p| list(&screen).row_box(*p).is_some_and(|b| b.y >= view.y)).unwrap();
    let b = row(&screen, place);
    click(&mut screen, b);
    assert_eq!(Some(place), list(&screen).chosen());
    assert_eq!(Some(part_named(&screen, "folders")), screen.focused());
}

#[test]
fn a_row_shows_its_numbers_in_the_person_s_format() {
    // C# ListPart.TextOf through ShownText: an integer with its thousands, as the person writes it.
    let host = DataHost::new();
    host.set("folders", json!([{ "id": 1, "name": "Big", "position": 1234567 }, { "id": 2, "name": "Half", "position": 2500.5 }]));
    let mut screen = window_of(r#"<List at="1,1" span="6" rows="5" name="folders" bind="folders" show="name, position" />"#, host.clone());
    laid(&mut screen, 800, 400);
    let list = list(&screen);
    let shown: Vec<String> = list.records().iter().map(|r| list.text_of(r)).collect();
    assert_eq!(vec!["Big | 1,234,567", "Half | 2,500.5"], shown);
}

#[test]
fn a_money_value_keeps_its_decimals_as_it_reaches_a_list() {
    // A runtime sends an exact decimal as its text ("12"); as C# shows a money value, it reads 12.00.
    let host = DataHost::new();
    host.set("folders", json!([{ "id": 1, "name": "Tea", "position": "12" }, { "id": 2, "name": "Cake", "position": 3.5 }]));
    host.types.borrow_mut().insert("folders.position".into(), "money".into());
    let mut screen = window_of(r#"<List at="1,1" span="6" rows="5" name="folders" bind="folders" show="name, position" />"#, host.clone());
    laid(&mut screen, 800, 400);
    let list = list(&screen);
    let shown: Vec<String> = list.records().iter().map(|r| list.text_of(r)).collect();
    assert!(shown[0].ends_with("12.00"), "{shown:?}");
    assert!(shown[1].ends_with("3.50"), "{shown:?}");
}

/// Rows that open something (`on open`) show the hand and say how to open
/// them; a list without `on open` leaves the pointer and says nothing.
#[test]
fn rows_that_open_show_the_hand_and_say_how() {
    use clarity_ui::Element;
    let (screen, _) = made("", 3);
    let r = row(&screen, 0);
    assert_eq!(Some(clarity_ui::input::Cursor::Hand), list(&screen).cursor_at(r.x + 4.0, r.y + 4.0));
    assert_eq!(Some("Double-click or press Enter to open".to_string()), list(&screen).tooltip_at(r.x + 4.0, r.y + 4.0));
    let host = DataHost::new();
    host.set("folders", folders(3));
    let mut plain = window_of(r#"<List at="1,1" span="6" rows="5" name="folders" bind="folders" show="name" />"#, host);
    laid(&mut plain, 800, 400);
    let r = list(&plain).row_box(0).expect("a row");
    assert_eq!(None, list(&plain).cursor_at(r.x + 4.0, r.y + 4.0));
    assert_eq!(None, list(&plain).tooltip_at(r.x + 4.0, r.y + 4.0));
}

/// `on drop from folders: filed`: a row dragged from one list and let go on a
/// row of another chooses both rows and runs that list's handler for it.
#[test]
fn a_row_dragged_onto_another_list_runs_its_drop_handler() {
    let host = DataHost::new();
    host.set("folders", folders(3));
    host.set("shelves", folders(2));
    let markup = r#"<List at="1,1" span="6" rows="5" name="folders" bind="folders" show="name" /><List at="7,1" span="6" rows="5" name="shelves" bind="shelves" show="name" drops='[{"from":"folders","handler":"filed"}]' />"#;
    let mut screen = window_of(markup, host.clone());
    laid(&mut screen, 800, 400);
    let lists: Vec<&ListPart> = clarity_ui::element::in_order(screen.root()).into_iter().filter_map(|e| e.as_any().downcast_ref::<ListPart>()).collect();
    let (from, onto) = (lists[0].row_box(2).expect("a folder"), lists[1].row_box(1).expect("a shelf"));
    screen.pointer_pressed(from.x + 10.0, from.y + from.height / 2.0, PointerButton::Left, 1);
    screen.pointer_moved(from.x + 60.0, from.y + from.height / 2.0);
    screen.pointer_moved(onto.x + 10.0, onto.y + onto.height / 2.0);
    screen.pointer_released(onto.x + 10.0, onto.y + onto.height / 2.0, PointerButton::Left);
    assert!(host.logged().iter().any(|e| e.contains("run shelves drop filed")), "{:?}", host.logged());
    assert_eq!(Some(2), clarity_ui::PartHost::selected(&*host, "folders"), "the dragged row is chosen in its list");
    assert_eq!(Some(1), clarity_ui::PartHost::selected(&*host, "shelves"), "the row it landed on is chosen");
}
