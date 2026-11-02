//! The date part, a day or a moment (conformance/forms.json, tables.json):
//! the day shows in the person's format (en-US here) and typed text is taken
//! when the input is left or Enter is pressed; a day is also picked from a
//! calendar under it. A moment has its time typed beside the day as hh:mm on
//! a 24-hour clock. Only the person's pick or typed change is a change; the
//! same day again and what code sets are not; text that is no day is marked
//! and writes nothing.

use std::rc::Rc;

use clarity_ui::geometry::Rect;
use clarity_ui::input::{Key, KeyInput, PointerButton};
use clarity_ui::parts::inputs::calendar::{Calendar, Day, DayCalendar};
use clarity_ui::parts::inputs::date::{read_time, DatePart, Which};
use clarity_ui::parts::inputs::formats::Formats;
use clarity_ui::parts::inputs::testing::InputsHost;
use clarity_ui::parts::{FieldFacts, Part, PartHost};
use clarity_ui::testing::{part_named, window_of, Shot};
use clarity_ui::Screen;
use serde_json::{json, Value};

const DUE: &str = r#"<Date at="1,1" span="4" name="due" bind="task.due" onchange="dueChanged" />"#;

fn day(y: i32, m: u32, d: u32) -> Day {
    Day::new(y, m, d).unwrap()
}

fn make(value: Value, markup: &str, moment: bool) -> (Screen, Rc<InputsHost>) {
    let host = InputsHost::new();
    host.use_formats("en-US", None);
    host.set("task.due", value);
    if moment {
        host.facts("task.due", FieldFacts { label: "Due".into(), format: Some("dateTime".into()), ..Default::default() });
    }
    let mut screen = window_of(markup, host.clone());
    screen.resize(800.0, 400.0, 1.0);
    (screen, host)
}

fn part<'a>(screen: &'a Screen, name: &str) -> &'a DatePart {
    screen.element(part_named(screen, name)).and_then(|e| e.as_any().downcast_ref::<DatePart>()).expect("a date part")
}

fn click(screen: &mut Screen, x: f64, y: f64) {
    screen.pointer_pressed(x, y, PointerButton::Left, 1);
    screen.pointer_released(x, y, PointerButton::Left);
}

fn middle(screen: &mut Screen, r: Rect) {
    click(screen, r.x + r.width / 2.0, r.y + r.height / 2.0);
}

fn press(screen: &mut Screen, key: Key) {
    screen.key_pressed(KeyInput::plain(key));
}

fn type_in(screen: &mut Screen, name: &str, which: Which, text: &str) {
    let id = part(screen, name).input(which).unwrap();
    screen.focus(Some(id));
    match which {
        Which::Day => part(screen, name).editor_mut().select_all(),
        Which::Time => part(screen, name).time_mut().unwrap().select_all(),
    }
    press(screen, Key::Delete);
    screen.text_typed(text);
}

fn click_button(screen: &mut Screen, name: &str) {
    let b = part(screen, name).button();
    middle(screen, b);
}

fn calendar(screen: &Screen) -> Calendar {
    part(screen, "due").calendar().expect("the calendar is open")
}

fn click_day(screen: &mut Screen, calendar: &Calendar, d: Day) {
    let cell = calendar.cell_of(d).expect("the grid shows the day");
    middle(screen, cell);
}

fn text(screen: &Screen) -> String {
    part(screen, "due").editor().text()
}

fn keep(shot: &mut Shot, name: &str) {
    if let Ok(dir) = std::env::var("CLARITY_SHOTS") {
        shot.save(std::path::Path::new(&dir).join(name)).expect("the picture is written");
    }
}

#[test]
fn a_typed_time_reads_on_a_24_hour_clock() {
    assert_eq!(Some((9, 30)), read_time("9:30"));
    assert_eq!(Some((14, 5)), read_time(" 14:05 "));
    assert_eq!(Some((0, 0)), read_time(""));
    assert_eq!(None, read_time("25:99"));
    assert_eq!(None, read_time("9:3"));
    assert_eq!(None, read_time("noon"));
}

#[test]
fn days_count_and_turn_as_a_calendar_does() {
    assert_eq!(day(2026, 11, 2), day(2026, 10, 31).add_days(2));
    assert_eq!(0, day(1970, 1, 1).number());
    assert_eq!(day(2024, 2, 29), Day::from_number(day(2024, 2, 29).number()));
    assert_eq!(1, day(2026, 11, 2).weekday());
    assert_eq!(day(2026, 2, 28), day(2026, 1, 31).add_months(1));
    assert_eq!(day(2025, 12, 20), day(2026, 1, 20).add_months(-1));
}

#[test]
fn it_shows_its_day_in_the_persons_format_and_what_code_sets_is_no_change() {
    let (mut screen, host) = make(json!("2026-10-20"), DUE, false);
    assert!(!part(&screen, "due").is_moment());
    assert!(part(&screen, "due").time().is_none());
    assert_eq!("10/20/2026", text(&screen));
    host.set("task.due", json!("2026-11-01"));
    Shot::of(&mut screen, 100, 40, 1.0);
    assert_eq!("11/01/2026", text(&screen));
    assert!(host.logged().is_empty());
}

#[test]
fn a_typed_day_is_taken_once_when_the_input_is_left() {
    let (mut screen, host) = make(json!("2026-10-20"), DUE, false);
    type_in(&mut screen, "due", Which::Day, "10/21/2026");
    assert!(host.logged().is_empty());
    screen.focus(None);
    assert_eq!(Some(json!("2026-10-21")), host.read("task.due"));
    assert_eq!(vec![r#"write due task.due "2026-10-21""#, "run due change dueChanged"], host.logged());
    assert_eq!("10/21/2026", text(&screen));
    let id = part(&screen, "due").input(Which::Day);
    screen.focus(id);
    screen.focus(None);
    assert_eq!(2, host.logged().len());
}

#[test]
fn enter_takes_a_typed_day_and_an_iso_and_a_long_day_read_too() {
    let (mut screen, host) = make(json!("2026-10-20"), DUE, false);
    type_in(&mut screen, "due", Which::Day, "2026-10-25");
    press(&mut screen, Key::Enter);
    assert_eq!(Some(json!("2026-10-25")), host.read("task.due"));
    assert_eq!("10/25/2026", text(&screen));
    type_in(&mut screen, "due", Which::Day, "October 26, 2026");
    press(&mut screen, Key::Enter);
    assert_eq!(Some(json!("2026-10-26")), host.read("task.due"));
}

#[test]
fn the_same_day_typed_again_is_no_change() {
    let (mut screen, host) = make(json!("2026-10-20"), DUE, false);
    type_in(&mut screen, "due", Which::Day, "10/20/2026");
    screen.focus(None);
    type_in(&mut screen, "due", Which::Day, " 10/20/2026 ");
    screen.focus(None);
    assert!(host.logged().is_empty());
    assert_eq!("10/20/2026", text(&screen));
}

#[test]
fn text_that_is_no_day_is_marked_and_writes_nothing() {
    let (mut screen, host) = make(json!("2026-10-20"), DUE, false);
    type_in(&mut screen, "due", Which::Day, "someday");
    screen.focus(None);
    assert!(host.logged().is_empty());
    assert_eq!(Some("someday".to_string()), part(&screen, "due").bad());
    assert_eq!("someday", text(&screen));
    type_in(&mut screen, "due", Which::Day, "13/45/2026");
    screen.focus(None);
    assert!(host.logged().is_empty());
    host.set("task.due", json!("2026-10-22"));
    Shot::of(&mut screen, 100, 40, 1.0);
    assert_eq!(None, part(&screen, "due").bad());
    assert_eq!("10/22/2026", text(&screen));
}

#[test]
fn clearing_the_text_clears_the_day() {
    let (mut screen, host) = make(json!("2026-10-20"), DUE, false);
    type_in(&mut screen, "due", Which::Day, "");
    screen.focus(None);
    assert_eq!(Some(Value::Null), host.read("task.due"));
    assert_eq!(vec!["write due task.due null", "run due change dueChanged"], host.logged());
}

#[test]
fn the_calendar_opens_below_on_its_value_month_and_a_click_on_a_day_picks_it() {
    let (mut screen, host) = make(json!("2026-10-20"), DUE, false);
    click_button(&mut screen, "due");
    assert_eq!(1, screen.overlays().len());
    assert!(screen.overlays()[0].place.y >= part(&screen, "due").base().node.bounds.bottom());
    assert!(screen.overlays()[0].content.as_any().downcast_ref::<DayCalendar>().is_some());
    let cal = calendar(&screen);
    assert_eq!(day(2026, 10, 1), cal.month());
    assert_eq!(Some(day(2026, 10, 20)), cal.chosen());
    assert_eq!(day(2026, 10, 20), cal.day());
    click_day(&mut screen, &cal, day(2026, 10, 21));
    assert!(screen.overlays().is_empty());
    assert_eq!(Some(json!("2026-10-21")), host.read("task.due"));
    assert_eq!(vec![r#"write due task.due "2026-10-21""#, "run due change dueChanged"], host.logged());
    assert_eq!("10/21/2026", text(&screen));
}

#[test]
fn picking_the_day_it_has_or_a_click_outside_is_no_change_and_the_button_closes_it_again() {
    let (mut screen, host) = make(json!("2026-10-20"), DUE, false);
    click_button(&mut screen, "due");
    let cal = calendar(&screen);
    click_day(&mut screen, &cal, day(2026, 10, 20));
    assert!(screen.overlays().is_empty());
    click_button(&mut screen, "due");
    assert_eq!(1, screen.overlays().len());
    click_button(&mut screen, "due");
    assert!(screen.overlays().is_empty());
    click_button(&mut screen, "due");
    click(&mut screen, 700.0, 380.0);
    assert!(screen.overlays().is_empty());
    assert!(host.logged().is_empty());
}

#[test]
fn the_months_buttons_turn_the_month() {
    let (mut screen, _) = make(json!("2026-01-20"), DUE, false);
    click_button(&mut screen, "due");
    let cal = calendar(&screen);
    middle(&mut screen, cal.previous());
    assert_eq!(day(2025, 12, 1), cal.month());
    middle(&mut screen, cal.next());
    middle(&mut screen, cal.next());
    assert_eq!(day(2026, 2, 1), cal.month());
    assert_eq!(1, screen.overlays().len());
    assert_eq!("February 2026", cal.title());
}

#[test]
fn keys_move_the_day_turn_the_month_pick_and_close() {
    let (mut screen, host) = make(json!("2026-10-20"), DUE, false);
    let input = part(&screen, "due").input(Which::Day);
    screen.focus(input);
    press(&mut screen, Key::Down);
    assert_eq!(1, screen.overlays().len());
    let cal = calendar(&screen);
    press(&mut screen, Key::Right);
    assert_eq!(day(2026, 10, 21), cal.day());
    press(&mut screen, Key::Down);
    assert_eq!(day(2026, 10, 28), cal.day());
    press(&mut screen, Key::Up);
    press(&mut screen, Key::Left);
    assert_eq!(day(2026, 10, 20), cal.day());
    press(&mut screen, Key::PageDown);
    assert_eq!(day(2026, 11, 20), cal.day());
    assert_eq!(day(2026, 11, 1), cal.month());
    press(&mut screen, Key::PageUp);
    press(&mut screen, Key::PageUp);
    assert_eq!(day(2026, 9, 20), cal.day());
    press(&mut screen, Key::Escape);
    assert!(screen.overlays().is_empty());
    assert!(host.logged().is_empty());
    press(&mut screen, Key::Down);
    press(&mut screen, Key::Right);
    press(&mut screen, Key::Enter);
    assert!(screen.overlays().is_empty());
    assert_eq!(Some(json!("2026-10-21")), host.read("task.due"));
    assert_eq!(input, screen.focused());
}

#[test]
fn a_moment_shows_its_time_on_a_24_hour_clock_and_a_picked_day_keeps_it() {
    let (mut screen, host) = make(json!("2026-10-20T14:05:00"), DUE, true);
    Shot::of(&mut screen, 800, 400, 1.0);
    assert!(part(&screen, "due").is_moment());
    assert_eq!("10/20/2026", text(&screen));
    assert_eq!("14:05", part(&screen, "due").time().unwrap().text());
    let (d, t) = (part(&screen, "due").input_bounds(Which::Day).unwrap(), part(&screen, "due").input_bounds(Which::Time).unwrap());
    assert!(t.x >= d.right());
    click_button(&mut screen, "due");
    let cal = calendar(&screen);
    click_day(&mut screen, &cal, day(2026, 10, 22));
    assert_eq!(Some(json!("2026-10-22T14:05:00")), host.read("task.due"));
    assert_eq!(2, host.logged().len());
}

#[test]
fn a_typed_time_changes_the_moment_and_a_time_that_is_no_time_is_marked() {
    let (mut screen, host) = make(json!("2026-10-20T14:05:00"), DUE, true);
    type_in(&mut screen, "due", Which::Time, "9:30");
    screen.focus(None);
    assert_eq!(Some(json!("2026-10-20T09:30:00")), host.read("task.due"));
    assert_eq!("09:30", part(&screen, "due").time().unwrap().text());
    assert_eq!(2, host.logged().len());
    type_in(&mut screen, "due", Which::Time, "25:99");
    press(&mut screen, Key::Enter);
    assert_eq!(Some("25:99".to_string()), part(&screen, "due").bad_time());
    assert_eq!(2, host.logged().len());
    type_in(&mut screen, "due", Which::Time, "09:30");
    screen.focus(None);
    assert_eq!(None, part(&screen, "due").bad_time());
    assert_eq!(2, host.logged().len());
}

#[test]
fn a_typed_day_keeps_a_moments_time_and_an_empty_moment_starts_at_midnight() {
    let (mut screen, host) = make(json!("2026-10-20T14:05:00"), DUE, true);
    type_in(&mut screen, "due", Which::Day, "10/23/2026");
    screen.focus(None);
    assert_eq!(Some(json!("2026-10-23T14:05:00")), host.read("task.due"));

    let (mut screen, host) = make(Value::Null, DUE, true);
    assert_eq!("", part(&screen, "due").time().unwrap().text());
    type_in(&mut screen, "due", Which::Time, "8:15");
    screen.focus(None);
    assert!(host.logged().is_empty());
    click_button(&mut screen, "due");
    let cal = calendar(&screen);
    assert_eq!(cal.today(), cal.day());
    let d = cal.month().add_days(21);
    click_day(&mut screen, &cal, d);
    assert_eq!(Some(json!(format!("{}T00:00:00", d.iso()))), host.read("task.due"));
}

#[test]
fn a_readonly_or_disabled_date_opens_nothing_and_takes_nothing() {
    let (mut screen, host) = make(json!("2026-10-20"), r#"<Date at="1,1" span="4" name="due" bind="task.due" readonly="true" />"#, false);
    click_button(&mut screen, "due");
    assert!(screen.overlays().is_empty());
    let input = part(&screen, "due").input(Which::Day);
    screen.focus(input);
    press(&mut screen, Key::Down);
    assert!(screen.overlays().is_empty());
    screen.text_typed("9");
    screen.focus(None);
    assert!(host.logged().is_empty());

    let (mut screen, host) = make(json!("2026-10-20"), DUE, false);
    host.set_state("due", |s| s.enabled = Some(false));
    click_button(&mut screen, "due");
    assert!(screen.overlays().is_empty());
}

#[test]
fn an_unbound_named_date_keeps_its_day_in_its_state() {
    let host = InputsHost::new();
    host.use_formats("en-US", None);
    let mut screen = window_of(r#"<Date at="1,1" span="4" name="when" onchange="whenChanged" />"#, host.clone());
    screen.resize(800.0, 400.0, 1.0);
    type_in(&mut screen, "when", Which::Day, "10/21/2026");
    screen.focus(None);
    assert_eq!(Some("2026-10-21".to_string()), host.state(Some("when")).unwrap().text);
    assert_eq!(vec!["when".to_string()], host.edits.borrow().clone());
    assert_eq!(vec!["run when change whenChanged"], host.logged());
    assert_eq!(Some(day(2026, 10, 21)), part(&screen, "when").value().map(|v| v.day));
}

/// An element that opens a calendar of its own, as another part (tags of days) does, and hears the day picked.
struct Opener {
    node: clarity_ui::element::Node,
    calendar: Option<Calendar>,
    picked: Rc<std::cell::Cell<Option<Day>>>,
}

impl clarity_ui::Element for Opener {
    clarity_ui::element_node!();

    fn on_overlay_closed(&mut self, _overlay: clarity_ui::element::ElementId, _cx: &mut clarity_ui::element::Cx) {
        self.picked.set(self.calendar.take().and_then(|c| c.take_picked()));
    }
}

#[test]
fn the_calendar_alone_takes_the_focus_marks_today_and_hands_the_picked_day() {
    let picked = Rc::new(std::cell::Cell::new(None));
    let opener = Opener { node: clarity_ui::element::Node::new(), calendar: None, picked: picked.clone() };
    let mut screen = Screen::new(Box::new(opener), clarity_ui::theme::ThemeLook::clarity(false));
    screen.resize(800.0, 400.0, 1.0);
    let root = screen.root().node().id();
    let formats = Formats::new(Some("en-US"), None);
    let cal = screen
        .act_root(|o: &mut Opener, cx| {
            let (_, cal) = DayCalendar::open(cx, Rect::new(10.0, 10.0, 100.0, 24.0), None, &formats, None, None, Some(root));
            o.calendar = Some(cal.clone());
            cal
        })
        .unwrap();
    cal.set_today(day(2026, 11, 2));
    assert_eq!(Some(screen.overlays()[0].id()), screen.focused());
    assert_eq!(None, cal.chosen());
    assert!(cal.cell_of(cal.today()).is_some());
    press(&mut screen, Key::Right);
    press(&mut screen, Key::Enter);
    assert_eq!(Some(day(2026, 11, 3)), picked.get());
    assert!(screen.overlays().is_empty());
}

#[test]
fn the_calendar_names_its_weekdays_and_months_in_the_persons_language() {
    let (mut screen, host) = make(json!("2026-10-20"), DUE, false);
    host.use_formats("nl-NL", None);
    Shot::of(&mut screen, 100, 40, 1.0);
    assert_eq!("20-10-2026", text(&screen));
    click_button(&mut screen, "due");
    let cal = calendar(&screen);
    assert_eq!("oktober 2026", cal.title());
    assert_eq!("ma", cal.weekdays()[0]);
}

#[test]
fn it_draws_closed_and_open_without_failing() {
    let (mut screen, _) = make(json!("2026-10-20T14:05:00"), DUE, true);
    let mut shot = Shot::of(&mut screen, 400, 320, 1.0);
    keep(&mut shot, "inputs-date-closed.png");
    click_button(&mut screen, "due");
    let mut shot = Shot::of(&mut screen, 400, 320, 1.0);
    keep(&mut shot, "inputs-date-open.png");
    assert_eq!(1, screen.overlays().len());
}

#[test]
fn a_required_day_cleared_is_no_day_and_the_form_says_it_is_missing_on_save() {
    // As Avalonia's DateInput: no input refuses an empty value, required or not.
    let host = InputsHost::new();
    host.use_formats("en-US", None);
    host.set("task.due", json!("2026-10-20"));
    host.facts("task.due", FieldFacts { label: "Due".into(), required: true, ..Default::default() });
    let mut screen = window_of(DUE, host.clone());
    screen.resize(800.0, 400.0, 1.0);
    type_in(&mut screen, "due", Which::Day, "");
    screen.focus(None);
    assert_eq!(None, part(&screen, "due").bad());
    assert_eq!(Some(Value::Null), host.read("task.due"));
    assert_eq!(vec!["write due task.due null", "run due change dueChanged"], host.logged());
}

/// A time typed on a 12-hour clock (9:30 PM) is read as the person means it; it then shows as hh:mm, as on every renderer.
#[test]
fn a_time_typed_with_am_or_pm_is_read() {
    let (mut screen, host) = make(json!("2026-10-20T14:05:00"), DUE, true);
    type_in(&mut screen, "due", Which::Time, "9:30 PM");
    screen.focus(None);
    assert_eq!(Some(json!("2026-10-20T21:30:00")), host.read("task.due"));
    assert_eq!("21:30", part(&screen, "due").time().unwrap().text());
    assert_eq!(Some((0, 30)), read_time("12:30 am"));
    assert_eq!(Some((21, 30)), read_time("9.30 p.m."));
    assert_eq!(None, read_time("13:30 PM"));
}

#[test]
fn a_time_without_a_day_waits_for_one() {
    let (mut screen, host) = make(Value::Null, DUE, true);
    type_in(&mut screen, "due", Which::Time, "8:15");
    press(&mut screen, Key::Enter);
    screen.focus(None);
    assert_eq!(Some(Value::Null), host.read("task.due"));
    assert_eq!(None, part(&screen, "due").bad_time());
    assert!(host.logged().is_empty());
    assert_eq!("8:15", part(&screen, "due").time().unwrap().text());
}

/// A form around a grid of parts: Enter in its one-line inputs takes what is typed and submits it.
struct TestForm {
    node: clarity_ui::element::Node,
    host: Rc<InputsHost>,
}

impl clarity_ui::Element for TestForm {
    clarity_ui::element_node!();

    fn arrange(&mut self, area: Rect, cx: &mut clarity_ui::element::Cx) {
        self.node.bounds = area;
        for child in self.node.children_mut() {
            child.arrange(area, cx);
        }
    }

    fn draw(&self, p: &clarity_ui::painter::Painter) {
        clarity_ui::element::draw_children(&self.node, p);
    }

    fn window_key(&mut self, key: KeyInput, cx: &mut clarity_ui::element::Cx) -> bool {
        if key.key != Key::Enter {
            return false;
        }
        let focused = cx.focused();
        if focused.and_then(|f| clarity_ui::element::find(self, f)).is_none_or(|e| e.multiline()) {
            return false;
        }
        clarity_ui::element::take_typed_from(self, focused, cx);
        self.host.log.borrow_mut().push("submit".to_string());
        true
    }
}

#[test]
fn enter_in_the_time_takes_it_and_then_the_form_submits() {
    let host = InputsHost::new();
    host.use_formats("en-US", None);
    host.set("task.due", json!("2026-10-20T14:05:00"));
    host.facts("task.due", FieldFacts { label: "Due".into(), format: Some("dateTime".into()), ..Default::default() });
    let spec = clarity_ui::markup::read(&format!("<Window name=\"Test\">{DUE}</Window>")).expect("the markup reads");
    let mut grid = clarity_ui::element::GridPanel::new(12);
    for p in spec.parts {
        let made = clarity_ui::parts::make(p, host.clone());
        let place = clarity_ui::parts::place_of(made.as_ref());
        grid.add(made, place);
    }
    let mut form = TestForm { node: clarity_ui::element::Node::new(), host: host.clone() };
    form.node.add(Box::new(grid));
    let mut screen = Screen::new(Box::new(form), clarity_ui::theme::ThemeLook::clarity(false));
    screen.resize(800.0, 400.0, 1.0);
    type_in(&mut screen, "due", Which::Time, "9:30");
    press(&mut screen, Key::Enter);
    assert_eq!(Some(json!("2026-10-20T09:30:00")), host.read("task.due"));
    let log = host.logged();
    assert_eq!("submit", log[log.len() - 1]);
    assert_eq!("run due change dueChanged", log[log.len() - 2]);
}

#[test]
fn clearing_the_day_empties_the_moment() {
    let (mut screen, host) = make(json!("2026-10-20T14:05:00"), DUE, true);
    type_in(&mut screen, "due", Which::Day, "");
    screen.focus(None);
    assert_eq!(Some(Value::Null), host.read("task.due"));
    assert_eq!(None, part(&screen, "due").value());
    assert_eq!("", text(&screen));
    assert_eq!("", part(&screen, "due").time().unwrap().text());
    assert_eq!("run due change dueChanged", host.logged().last().unwrap());
}

// -- The form cases of the day and the moment ----------------------------------

const MINE: [&str; 2] = ["day", "due"];

const FORM: &str = r#"
    <Date at="1,1" span="4" name="day" bind="day" onchange="changed" />
    <Date at="1,2" span="4" name="due" bind="due" onchange="changed" />
"#;

fn forms() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../conformance/forms.json");
    serde_json::from_str(&std::fs::read_to_string(path).expect("conformance/forms.json")).expect("it is JSON")
}

fn load(host: &InputsHost, record: &Value) {
    for field in MINE {
        host.values.borrow_mut().insert(field.to_string(), record[field].clone());
    }
}

/// Whether a case only concerns the day and the moment (its other values are left out).
fn concerns(case: &Value) -> bool {
    let steps = case["steps"].as_array().unwrap();
    let on = |s: &Value| MINE.contains(&s[1].as_str().unwrap_or(""));
    let touches = steps.iter().any(|s| matches!(s[0].as_str(), Some("pickDay" | "type")) && on(s));
    touches && steps.iter().all(|s| matches!(s[0].as_str(), Some("leave" | "enter" | "reload")) || on(s))
}

#[test]
fn every_form_case_of_a_day_or_a_moment_behaves_as_on_every_renderer() {
    let forms = forms();
    let mut run = 0;
    for case in forms["cases"].as_array().unwrap() {
        if !concerns(case) {
            continue;
        }
        run += 1;
        let name = case["name"].as_str().unwrap();
        let host = InputsHost::new();
        host.use_formats("en-US", None);
        host.facts("due", FieldFacts { label: "Due".into(), format: Some("dateTime".into()), ..Default::default() });
        load(&host, &forms["record"]);
        let mut screen = window_of(FORM, host.clone());
        screen.resize(800.0, 400.0, 1.0);
        Shot::of(&mut screen, 800, 400, 1.0);
        for step in case["steps"].as_array().unwrap() {
            let field = step[1].as_str().unwrap_or("");
            match step[0].as_str().unwrap() {
                "pickDay" => {
                    click_button(&mut screen, field);
                    let cal = part(&screen, field).calendar().expect("the calendar is open");
                    click_day(&mut screen, &cal, Day::parse(step[2].as_str().unwrap()).unwrap());
                }
                "type" => type_in(&mut screen, field, if field == "due" { Which::Time } else { Which::Day }, step[2].as_str().unwrap()),
                "leave" => screen.focus(None),
                "enter" => press(&mut screen, Key::Enter),
                "handler" => host.set(field, step[2].clone()),
                "reload" => {
                    load(&host, &forms["record"]);
                    host.change();
                }
                other => panic!("{name}: no step {other}"),
            }
        }
        let events: Vec<String> = host
            .logged()
            .iter()
            .filter_map(|l| l.strip_prefix("run ").map(|r| r.split(' ').take(2).collect::<Vec<_>>()))
            .map(|w| format!("{} {}", w[1], w[0]))
            .collect();
        let wanted: Vec<String> = case["expect"]["events"].as_array().unwrap().iter().map(|e| e.as_str().unwrap().to_string()).collect();
        assert_eq!(wanted, events, "{name}: events");
        for v in case["expect"]["values"].as_array().into_iter().flatten() {
            assert_eq!(Some(v[1].clone()), host.read(v[0].as_str().unwrap()), "{name}");
        }
        for m in case["expect"]["marked"].as_array().into_iter().flatten() {
            let field = m.as_str().unwrap();
            let marked = if field == "due" { part(&screen, field).bad_time() } else { part(&screen, field).bad() };
            assert!(marked.is_some(), "{name}: {field} is marked");
        }
    }
    assert!(run >= 3, "only {run} cases ran");
}

#[test]
fn every_form_input_of_a_day_or_a_moment_shows_its_value_and_what_a_handler_sets_is_no_change() {
    let forms = forms();
    let host = InputsHost::new();
    host.use_formats("en-US", None);
    host.facts("due", FieldFacts { label: "Due".into(), format: Some("dateTime".into()), ..Default::default() });
    load(&host, &forms["record"]);
    let mut screen = window_of(FORM, host.clone());
    screen.resize(800.0, 400.0, 1.0);
    Shot::of(&mut screen, 800, 400, 1.0);
    assert_eq!("14:05", part(&screen, "due").time().unwrap().text());
    assert_eq!("10/20/2026", part(&screen, "day").editor().text());
    host.set("day", json!("2026-11-01"));
    host.set("due", json!("2026-11-01T08:00:00"));
    Shot::of(&mut screen, 800, 400, 1.0);
    assert_eq!("11/01/2026", part(&screen, "day").editor().text());
    assert_eq!("08:00", part(&screen, "due").time().unwrap().text());
    assert!(host.logged().is_empty());
}
