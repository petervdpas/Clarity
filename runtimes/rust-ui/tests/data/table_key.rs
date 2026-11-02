//! Every kind of edited cell is reached and changed with the keyboard (C#
//! TableKeyTests): Tab walks checks, switches, choices and dates as well as
//! typed cells; Space ticks a check; a choice cell's list opens with Enter,
//! Space or Alt+Down and is worked with the arrows, Home, End, Enter, Escape
//! and the first letters of a choice; a date is typed as well as picked
//! from its calendar.

use super::data_rows_host;

use std::rc::Rc;

use clarity_ui::element::find_type;
use clarity_ui::geometry::Rect;
use clarity_ui::input::{Key, KeyInput, Modifiers};
use clarity_ui::parts::data::table::TablePart;
use clarity_ui::parts::inputs::calendar::Day;
use clarity_ui::parts::inputs::formats::{FieldFormat, FormatKind, Formats};
use clarity_ui::screen::Screen;
use clarity_ui::testing::{part_named, window_of};
use data_rows_host::{click, laid, DataHost};
use serde_json::json;

fn table(screen: &Screen) -> &TablePart {
    find_type::<TablePart>(screen.root(), |_| true).expect("a table")
}

fn k(key: Key) -> KeyInput {
    KeyInput::new(key, Modifiers::NONE)
}

fn cell(screen: &Screen, place: usize, field: &str) -> Rect {
    table(screen).cell_box(place, field).unwrap_or_else(|| panic!("no cell {field} of row {place}"))
}

fn focused_cell(screen: &Screen) -> Option<(usize, String)> {
    table(screen).focused_cell()
}

fn at(place: usize, field: &str) -> Option<(usize, String)> {
    Some((place, field.to_string()))
}

fn made(extra: &str) -> (Screen, Rc<DataHost>) {
    let host = DataHost::new();
    host.set("tasks", json!([
        { "title": "Write", "done": false, "priority": "Low", "day": "2026-10-05" },
        { "title": "Read", "done": false, "priority": "High", "day": null }
    ]));
    host.types.borrow_mut().insert("tasks.day".into(), "date".into());
    host.offer("tasks.priority", &[(json!("Low"), "Low"), (json!("Medium"), "Medium"), (json!("High"), "High"), (json!("Highest"), "Highest")]);
    let markup = format!(r#"<Table name="tasks" at="1,1" span="12" rows="8" bind="tasks" columns="title, done, priority, day" edit="title, done, priority, day" onopen="opened" onselect="picked" onchange="changed" {extra} />"#);
    let mut screen = window_of(&markup, host.clone());
    laid(&mut screen, 900, 600);
    (screen, host)
}

fn press(screen: &mut Screen, key: KeyInput) {
    screen.key_pressed(key);
    laid(screen, 900, 600);
}

#[test]
fn tab_walks_checks_choices_and_dates_too_and_space_ticks_a_check() {
    let (mut screen, host) = made("");
    let me = part_named(&screen, "tasks");
    let b = cell(&screen, 0, "title");
    click(&mut screen, b);
    press(&mut screen, k(Key::Tab));
    assert_eq!(at(0, "done"), focused_cell(&screen));
    assert_eq!(Some(me), screen.focused());
    assert!(table(&screen).cell_editor().is_none());
    press(&mut screen, k(Key::Space));
    assert_eq!(json!(true), host.get("tasks[0].done"));
    press(&mut screen, k(Key::Tab));
    assert_eq!(at(0, "priority"), focused_cell(&screen));
    press(&mut screen, k(Key::Enter));
    assert!(table(&screen).open_choices().is_some());
    press(&mut screen, k(Key::Down));
    press(&mut screen, k(Key::Enter));
    assert!(table(&screen).open_choices().is_none());
    assert_eq!(json!("Medium"), host.get("tasks[0].priority"));
    // A date is typed in its cell.
    press(&mut screen, k(Key::Tab));
    assert!(table(&screen).cell_editor().is_some());
    assert_eq!(at(0, "day"), focused_cell(&screen));
    screen.text_typed("10/22/2026");
    press(&mut screen, k(Key::Enter));
    assert_eq!(json!("2026-10-22"), host.get("tasks[0].day"));
    // Shift+Tab goes back through them.
    let b = cell(&screen, 0, "day");
    click(&mut screen, b);
    press(&mut screen, KeyInput::new(Key::Tab, Modifiers::SHIFT));
    assert_eq!(at(0, "priority"), focused_cell(&screen));
    press(&mut screen, KeyInput::new(Key::Tab, Modifiers::SHIFT));
    assert_eq!(at(0, "done"), focused_cell(&screen));
    // A press on a check ticks it and leaves it the cell the keys work on.
    let b = cell(&screen, 0, "done");
    click(&mut screen, b);
    assert_eq!(json!(false), host.get("tasks[0].done"));
    assert_eq!(at(0, "done"), focused_cell(&screen));
    laid(&mut screen, 900, 600);
}

#[test]
fn a_choice_list_is_worked_with_the_keyboard() {
    let (mut screen, host) = made("");
    let me = part_named(&screen, "tasks");
    let b = cell(&screen, 0, "title");
    click(&mut screen, b);
    press(&mut screen, k(Key::Tab));
    press(&mut screen, k(Key::Tab));
    assert_eq!(at(0, "priority"), focused_cell(&screen));
    press(&mut screen, KeyInput::new(Key::Down, Modifiers::ALT));
    assert!(table(&screen).open_choices().is_some());
    assert_eq!(Some(0), table(&screen).open_choices_highlighted());
    press(&mut screen, k(Key::End));
    assert_eq!(Some(3), table(&screen).open_choices_highlighted());
    press(&mut screen, k(Key::Home));
    assert_eq!(Some(0), table(&screen).open_choices_highlighted());
    // The first letters of a choice go to it.
    screen.text_typed("h");
    assert_eq!(Some(2), table(&screen).open_choices_highlighted());
    screen.text_typed("ighe");
    assert_eq!(Some(3), table(&screen).open_choices_highlighted());
    // Escape closes the list and the value stays.
    press(&mut screen, k(Key::Escape));
    assert!(table(&screen).open_choices().is_none());
    assert_eq!(json!("Low"), host.get("tasks[0].priority"));
    assert_eq!(Some(me), screen.focused());
    press(&mut screen, k(Key::Space));
    press(&mut screen, k(Key::Up));
    assert_eq!(Some(0), table(&screen).open_choices_highlighted());
    press(&mut screen, k(Key::Down));
    press(&mut screen, k(Key::Down));
    press(&mut screen, k(Key::Enter));
    assert_eq!(json!("High"), host.get("tasks[0].priority"));
    assert_eq!(vec!["select", "change"], host.events());
}

#[test]
fn a_date_cell_is_typed_and_its_button_or_alt_down_opens_the_calendar() {
    let (mut screen, host) = made("");
    let b = cell(&screen, 0, "day");
    click(&mut screen, b);
    let shown = Formats::default().show(&FieldFormat::new(FormatKind::Date), &json!("2026-10-05"));
    assert_eq!(shown, table(&screen).cell_editor().expect("the day is typed into").text());
    screen.text_typed("soon");
    press(&mut screen, k(Key::Enter));
    assert!(table(&screen).marked(0, "day"));
    assert_eq!(json!("2026-10-05"), host.get("tasks[0].day"));
    // Alt+Down in the typed date opens its calendar.
    click(&mut screen, b);
    press(&mut screen, KeyInput::new(Key::Down, Modifiers::ALT));
    assert!(table(&screen).open_calendar().is_some());
    press(&mut screen, k(Key::Escape));
    assert!(table(&screen).open_calendar().is_none());
    // So does a press on its button.
    let button = table(&screen).calendar_button_box(0, "day").expect("its button");
    click(&mut screen, button);
    laid(&mut screen, 900, 600);
    let calendar = table(&screen).open_calendar().expect("the calendar opens").clone();
    click(&mut screen, calendar.cell_of(Day::new(2026, 10, 20).unwrap()).unwrap());
    assert_eq!(json!("2026-10-20"), host.get("tasks[0].day"));
    assert!(!table(&screen).marked(0, "day"));
}

#[test]
fn enter_on_a_chosen_row_opens_it_and_alt_and_an_arrow_without_a_chosen_row_passes_on() {
    let (mut screen, host) = made(r#"reorder="position""#);
    let me = part_named(&screen, "tasks");
    screen.focus(Some(me));
    let took = screen.act(me, |e, cx| e.on_key(KeyInput::new(Key::Down, Modifiers::ALT), cx));
    assert_eq!(Some(false), took);
    assert_eq!(None, table(&screen).chosen());
    press(&mut screen, k(Key::Down));
    press(&mut screen, k(Key::Enter));
    assert_eq!(vec!["select", "open"], host.events());
}
