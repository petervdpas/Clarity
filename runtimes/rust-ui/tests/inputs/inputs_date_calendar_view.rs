//! The calendar's year and decade views, as Avalonia's Calendar zoomed out
//! (DisplayMode): a click on the title shows the year's months and then the
//! decade's years; a picked year shows its months and a picked month its days.
//! The month buttons turn a month, a year or a decade in each view. Keys:
//! Ctrl+Up zooms out, Ctrl+Down or Enter zooms in on the marked month or year,
//! arrows move the mark (Up and Down a row) and Escape closes.

use std::cell::Cell;
use std::rc::Rc;

use clarity_ui::element::{Cx, Element, ElementId, Node};
use clarity_ui::geometry::Rect;
use clarity_ui::input::{Key, KeyInput, Modifiers, PointerButton};
use clarity_ui::parts::inputs::calendar::{Calendar, Day, DayCalendar, View};
use clarity_ui::parts::inputs::date::{DatePart, Which};
use clarity_ui::parts::inputs::formats::Formats;
use clarity_ui::parts::inputs::testing::InputsHost;
use clarity_ui::testing::{part_named, window_of, Shot};
use clarity_ui::theme::ThemeLook;
use clarity_ui::Screen;
use serde_json::json;
use skia_safe::Color;

const DUE: &str = r#"<Date at="1,1" span="4" name="due" bind="task.due" onchange="dueChanged" />"#;

fn day(y: i32, m: u32, d: u32) -> Day {
    Day::new(y, m, d).unwrap()
}

/// An element that opens a calendar of its own and hears the day picked.
struct Opener {
    node: Node,
    calendar: Option<Calendar>,
    picked: Rc<Cell<Option<Day>>>,
}

impl Element for Opener {
    clarity_ui::element_node!();

    fn on_overlay_closed(&mut self, _overlay: ElementId, _cx: &mut Cx) {
        self.picked.set(self.calendar.take().and_then(|c| c.take_picked()));
    }
}

/// A calendar alone on a screen, opened on `on` in `language`, with today 2026-11-02.
fn open(on: Day, language: &str) -> (Screen, Calendar, Rc<Cell<Option<Day>>>) {
    let picked = Rc::new(Cell::new(None));
    let opener = Opener { node: Node::new(), calendar: None, picked: picked.clone() };
    let mut screen = Screen::new(Box::new(opener), ThemeLook::clarity(false));
    screen.resize(800.0, 400.0, 1.0);
    let root = screen.root().node().id();
    let formats = Formats::new(Some(language), None);
    let calendar = screen
        .act_root(|o: &mut Opener, cx| {
            let (_, calendar) = DayCalendar::open(cx, Rect::new(10.0, 10.0, 100.0, 24.0), Some(on), &formats, None, None, Some(root));
            o.calendar = Some(calendar.clone());
            calendar
        })
        .unwrap();
    calendar.set_today(day(2026, 11, 2));
    (screen, calendar, picked)
}

fn click(screen: &mut Screen, r: Rect) {
    let (x, y) = (r.x + r.width / 2.0, r.y + r.height / 2.0);
    screen.pointer_pressed(x, y, PointerButton::Left, 1);
    screen.pointer_released(x, y, PointerButton::Left);
}

fn press(screen: &mut Screen, key: Key, held: Modifiers) {
    screen.key_pressed(KeyInput::new(key, held));
}

/// Whether some pixel in `area` is near `color`.
fn inked(shot: &mut Shot, area: Rect, color: Color) -> bool {
    for y in area.y as i32..(area.y + area.height) as i32 {
        for x in area.x as i32..(area.x + area.width) as i32 {
            let p = shot.pixel(x, y);
            let near = |a: u8, b: u8| (a as i32 - b as i32).abs() < 6;
            if near(p.r(), color.r()) && near(p.g(), color.g()) && near(p.b(), color.b()) {
                return true;
            }
        }
    }
    false
}

#[test]
fn a_click_on_the_title_shows_the_year_and_then_the_decade() {
    let (mut screen, calendar, picked) = open(day(2026, 10, 20), "en-US");
    assert_eq!(View::Month, calendar.view());
    click(&mut screen, calendar.title_box());
    assert_eq!(View::Year, calendar.view());
    assert_eq!("2026", calendar.title());
    assert!(calendar.cell_of(day(2026, 1, 1)).is_some());
    assert!(calendar.cell_of(day(2026, 12, 1)).is_some());
    assert!(calendar.cell_of(day(2027, 1, 1)).is_none());
    click(&mut screen, calendar.title_box());
    assert_eq!(View::Decade, calendar.view());
    assert_eq!("2020-2029", calendar.title());
    // The decade shows a year before and after it too.
    assert!(calendar.cell_of(day(2019, 1, 1)).is_some());
    assert!(calendar.cell_of(day(2030, 1, 1)).is_some());
    assert!(calendar.cell_of(day(2031, 1, 1)).is_none());
    click(&mut screen, calendar.title_box());
    assert_eq!(View::Decade, calendar.view());
    assert_eq!(1, screen.overlays().len());
    assert_eq!(None, picked.get());
}

#[test]
fn the_buttons_turn_a_year_in_the_year_view_and_a_decade_in_the_decade_view() {
    let (mut screen, calendar, _) = open(day(2026, 10, 20), "en-US");
    click(&mut screen, calendar.title_box());
    click(&mut screen, calendar.previous());
    assert_eq!("2025", calendar.title());
    click(&mut screen, calendar.next());
    click(&mut screen, calendar.next());
    assert_eq!("2027", calendar.title());
    let next = calendar.next();
    assert_eq!(Some("Next year".to_string()), calendar.tooltip_at(next.x + 2.0, next.y + 2.0));
    click(&mut screen, calendar.title_box());
    click(&mut screen, calendar.next());
    assert_eq!("2030-2039", calendar.title());
    click(&mut screen, calendar.previous());
    click(&mut screen, calendar.previous());
    assert_eq!("2010-2019", calendar.title());
    let previous = calendar.previous();
    assert_eq!(Some("Previous decade".to_string()), calendar.tooltip_at(previous.x + 2.0, previous.y + 2.0));
}

#[test]
fn picking_a_year_and_a_month_turns_the_calendar_there() {
    let (mut screen, calendar, picked) = open(day(2026, 10, 31), "en-US");
    click(&mut screen, calendar.title_box());
    click(&mut screen, calendar.title_box());
    click(&mut screen, calendar.next());
    click(&mut screen, calendar.cell_of(day(2033, 1, 1)).unwrap());
    assert_eq!(View::Year, calendar.view());
    assert_eq!("2033", calendar.title());
    click(&mut screen, calendar.cell_of(day(2033, 2, 1)).unwrap());
    assert_eq!(View::Month, calendar.view());
    assert_eq!(day(2033, 2, 1), calendar.month());
    // The marked day keeps its number where the month has it, else the month's last day.
    assert_eq!(day(2033, 2, 28), calendar.day());
    assert_eq!(None, picked.get());
    assert_eq!(1, screen.overlays().len());
    click(&mut screen, calendar.cell_of(day(2033, 2, 14)).unwrap());
    assert_eq!(Some(day(2033, 2, 14)), picked.get());
}

#[test]
fn keys_zoom_out_and_in_and_move_the_marked_month_and_year() {
    let host = InputsHost::new();
    host.use_formats("en-US", None);
    host.set("task.due", json!("2026-10-20"));
    let mut screen = window_of(DUE, host.clone());
    screen.resize(800.0, 400.0, 1.0);
    let part = |screen: &Screen| -> Option<Calendar> {
        screen.element(part_named(screen, "due")).and_then(|e| e.as_any().downcast_ref::<DatePart>()).and_then(|p| p.calendar())
    };
    let input = screen.element(part_named(&screen, "due")).and_then(|e| e.as_any().downcast_ref::<DatePart>()).unwrap().input(Which::Day);
    screen.focus(input);
    press(&mut screen, Key::Down, Modifiers::NONE);
    assert_eq!(1, screen.overlays().len());
    let calendar = part(&screen).expect("the calendar is open");
    press(&mut screen, Key::Up, Modifiers::CONTROL);
    assert_eq!(View::Year, calendar.view());
    press(&mut screen, Key::Right, Modifiers::NONE);
    assert_eq!(day(2026, 11, 20), calendar.day());
    press(&mut screen, Key::Down, Modifiers::NONE);
    assert_eq!(day(2027, 3, 20), calendar.day());
    assert_eq!("2027", calendar.title());
    press(&mut screen, Key::Up, Modifiers::CONTROL);
    assert_eq!(View::Decade, calendar.view());
    press(&mut screen, Key::Left, Modifiers::NONE);
    assert_eq!(2026, calendar.day().year);
    press(&mut screen, Key::Up, Modifiers::NONE);
    assert_eq!(2022, calendar.day().year);
    press(&mut screen, Key::Enter, Modifiers::NONE);
    assert_eq!(View::Year, calendar.view());
    assert_eq!("2022", calendar.title());
    press(&mut screen, Key::Down, Modifiers::CONTROL);
    assert_eq!(View::Month, calendar.view());
    assert_eq!(day(2022, 3, 1), calendar.month());
    assert_eq!(1, screen.overlays().len());
    assert!(host.logged().is_empty());
    press(&mut screen, Key::Up, Modifiers::CONTROL);
    press(&mut screen, Key::Escape, Modifiers::NONE);
    assert!(screen.overlays().is_empty());
    assert!(host.logged().is_empty());
    assert_eq!(input, screen.focused());
}

#[test]
fn the_year_and_decade_views_mark_the_chosen_one_in_the_accent_and_draw_their_names() {
    let (mut screen, calendar, _) = open(day(2026, 10, 20), "en-US");
    click(&mut screen, calendar.title_box());
    assert_eq!("Oct", calendar.name_of(day(2026, 10, 1)));
    let accent = screen.look().color("accent");
    let mut shot = Shot::of(&mut screen, 800, 400, 1.0);
    assert!(inked(&mut shot, calendar.cell_of(day(2026, 10, 1)).unwrap(), accent));
    assert!(!inked(&mut shot, calendar.cell_of(day(2026, 3, 1)).unwrap(), accent));
    click(&mut screen, calendar.title_box());
    assert_eq!("2026", calendar.name_of(day(2026, 1, 1)));
    let mut shot = Shot::of(&mut screen, 800, 400, 1.0);
    assert!(inked(&mut shot, calendar.cell_of(day(2026, 1, 1)).unwrap(), accent));
    assert!(!inked(&mut shot, calendar.cell_of(day(2023, 1, 1)).unwrap(), accent));
}

#[test]
fn the_month_names_of_the_year_view_are_the_persons_language() {
    let (_, calendar, _) = open(day(2026, 3, 4), "nl-NL");
    calendar.zoom_out();
    assert_eq!("mrt", calendar.name_of(day(2026, 3, 1)));
}
