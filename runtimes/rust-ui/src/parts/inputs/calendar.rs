//! A calendar over the window: one month as a grid of days under its name,
//! with buttons for the month before and after. Its weekday and month names
//! are the person's language's. Today is marked, and so is the chosen day; a
//! click on a day picks it. Arrows move the marked day, Page Up and Page
//! Down a month, Enter picks it and Escape closes the calendar. A date part
//! opens it under its input; another part (tags of days, a table's date
//! cell) opens it the same way.
//!
//! The calendar is the overlay's element (`DayCalendar`) and a handle to
//! what it shows (`Calendar`), which the part that opened it keeps: the part
//! hands it its keys while it keeps the focus, and reads the picked day when
//! the overlay closes (`Element::on_overlay_closed` of its opener).

use std::cell::RefCell;
use std::rc::Rc;

use skia_safe::{Color, Paint, PaintCap, PaintStyle};

use crate::element::{Cx, Element, ElementId, Node};
use crate::geometry::Rect;
use crate::input::{Key, KeyInput, PointerInput};
use crate::locale;
use crate::overlay::Overlay;
use crate::painter::{Painter, TextAlign};
use crate::parts::inputs::formats::{days_in, Formats, Moment};

const PAD: f64 = 8.0;
const HEAD: f64 = 32.0;
const WEEK_ROW: f64 = 22.0;
const CELL_WIDTH: f64 = 32.0;
const CELL_HEIGHT: f64 = 28.0;

/// How wide a calendar is.
pub const WIDTH: f64 = 7.0 * CELL_WIDTH + 2.0 * PAD;

/// How high a calendar is.
pub const HEIGHT: f64 = PAD + HEAD + WEEK_ROW + 6.0 * CELL_HEIGHT + PAD;

/// A day of the calendar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Day {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

impl Day {
    /// The day of a year, month (1 to 12) and day; none for one that is no day.
    pub fn new(year: i32, month: u32, day: u32) -> Option<Day> {
        (day >= 1 && days_in(year, month)? >= day).then_some(Day { year, month, day })
    }

    /// The day of a stored date or moment (`2026-10-20`, `2026-10-20T14:05:00`); none for text that is none.
    pub fn parse(text: &str) -> Option<Day> {
        Moment::parse(text.trim()).map(|m| Day { year: m.year, month: m.month, day: m.day })
    }

    /// Today, where the program runs.
    pub fn today() -> Day {
        Day::parse(&clarity_runtime::today().to_string()).unwrap_or(Day { year: 2000, month: 1, day: 1 })
    }

    /// As stored: `2026-10-20`.
    pub fn iso(&self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }

    /// The days since 1970-01-01 (negative before).
    pub fn number(&self) -> i64 {
        let y = self.year as i64 - if self.month <= 2 { 1 } else { 0 };
        let era = y.div_euclid(400);
        let yoe = y - era * 400;
        let m = self.month as i64;
        let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + self.day as i64 - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146097 + doe - 719468
    }

    /// The day of a number of days since 1970-01-01.
    pub fn from_number(number: i64) -> Day {
        let z = number + 719468;
        let era = z.div_euclid(146097);
        let doe = z - era * 146097;
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
        let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
        let year = (yoe + era * 400 + if month <= 2 { 1 } else { 0 }) as i32;
        Day { year, month, day }
    }

    /// The day of the week: 0 for Sunday to 6 for Saturday.
    pub fn weekday(&self) -> u32 {
        (self.number() + 4).rem_euclid(7) as u32
    }

    pub fn add_days(&self, days: i64) -> Day {
        Day::from_number(self.number() + days)
    }

    /// The same day `months` months on; a day past the month's end is its last.
    pub fn add_months(&self, months: i32) -> Day {
        let all = self.year * 12 + self.month as i32 - 1 + months;
        let (year, month) = (all.div_euclid(12), all.rem_euclid(12) as u32 + 1);
        let last = days_in(year, month).unwrap_or(28);
        Day { year, month, day: self.day.min(last) }
    }

    /// The first day of its month.
    pub fn first_of_month(&self) -> Day {
        Day { day: 1, ..*self }
    }
}

/// The words a calendar shows in the person's language: the weekdays'
/// short names from the first day of the week, and the month's title.
#[derive(Debug, Clone)]
struct Words {
    formats: Formats,
    weekdays: [&'static str; 7],
    first: u32,
}

impl Words {
    fn of(formats: &Formats) -> Words {
        Words { formats: formats.clone(), weekdays: formats.weekdays(), first: formats.first_weekday() }
    }

    fn title(&self, month: Day) -> String {
        self.formats.month_title(month.year, month.month)
    }
}

/// What a calendar shows, shared by its element and the part that opened it.
struct State {
    chosen: Option<Day>,
    day: Day,
    month: Day,
    today: Day,
    words: Words,
    picked: Option<Day>,
    hovered: Option<Day>,
    bounds: Rect,
}

/// What a key did to a calendar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keyed {
    /// It was no key of the calendar's.
    Not,
    /// It moved the marked day or turned the month.
    Moved,
    /// Enter or Space picked the marked day: the calendar closes, and its picked day is this one.
    Picked(Day),
    /// Escape: the calendar closes without a pick.
    Closed,
}

/// A handle to what a calendar shows, which the part that opened it keeps.
#[derive(Clone)]
pub struct Calendar(Rc<RefCell<State>>);

impl Calendar {
    /// A calendar on the month of `chosen` (today's for none), in the words of `formats`.
    pub fn new(chosen: Option<Day>, formats: &Formats) -> Calendar {
        let today = Day::today();
        let day = chosen.unwrap_or(today);
        Calendar(Rc::new(RefCell::new(State {
            chosen,
            day,
            month: day.first_of_month(),
            today,
            words: Words::of(formats),
            picked: None,
            hovered: None,
            bounds: Rect::default(),
        })))
    }

    /// The day it was opened with; none for an empty value.
    pub fn chosen(&self) -> Option<Day> {
        self.0.borrow().chosen
    }

    /// The day the keys move, which Enter picks.
    pub fn day(&self) -> Day {
        self.0.borrow().day
    }

    /// The first day of the month it shows.
    pub fn month(&self) -> Day {
        self.0.borrow().month
    }

    /// Today, which it marks.
    pub fn today(&self) -> Day {
        self.0.borrow().today
    }

    /// Makes `today` the day it marks as today; one opened on no day moves there too.
    pub fn set_today(&self, today: Day) {
        let mut s = self.0.borrow_mut();
        if s.chosen.is_none() && s.day == s.today {
            s.day = today;
            s.month = today.first_of_month();
        }
        s.today = today;
    }

    /// The month's name and year, in the person's language.
    pub fn title(&self) -> String {
        let s = self.0.borrow();
        s.words.title(s.month)
    }

    /// The weekdays' short names, from the first day of the week.
    pub fn weekdays(&self) -> Vec<String> {
        let s = self.0.borrow();
        s.words.weekdays.iter().map(|w| w.to_string()).collect()
    }

    /// The day picked, taken: the opener reads it once the calendar closed.
    pub fn take_picked(&self) -> Option<Day> {
        self.0.borrow_mut().picked.take()
    }

    /// Where the calendar is, on the screen.
    pub fn bounds(&self) -> Rect {
        self.0.borrow().bounds
    }

    /// The button of the month before.
    pub fn previous(&self) -> Rect {
        let b = self.bounds();
        Rect::new(b.x + PAD, b.y + PAD + 2.0, HEAD - 4.0, HEAD - 4.0)
    }

    /// The button of the month after.
    pub fn next(&self) -> Rect {
        let b = self.bounds();
        Rect::new(b.right() - PAD - (HEAD - 4.0), b.y + PAD + 2.0, HEAD - 4.0, HEAD - 4.0)
    }

    /// The first day of its grid: the start of the week the month starts in.
    fn grid_start(&self) -> Day {
        let s = self.0.borrow();
        let back = (s.month.weekday() + 7 - s.words.first) % 7;
        s.month.add_days(-(back as i64))
    }

    /// Where a day's cell is; none when its grid does not show it.
    pub fn cell_of(&self, day: Day) -> Option<Rect> {
        let at = day.number() - self.grid_start().number();
        if !(0..42).contains(&at) {
            return None;
        }
        let b = self.bounds();
        Some(Rect::new(b.x + PAD + (at % 7) as f64 * CELL_WIDTH, b.y + PAD + HEAD + WEEK_ROW + (at / 7) as f64 * CELL_HEIGHT, CELL_WIDTH, CELL_HEIGHT))
    }

    /// The day at a point; none outside its grid.
    fn day_at(&self, x: f64, y: f64) -> Option<Day> {
        let b = self.bounds();
        let col = ((x - b.x - PAD) / CELL_WIDTH).floor();
        let row = ((y - b.y - PAD - HEAD - WEEK_ROW) / CELL_HEIGHT).floor();
        if !(0.0..=6.0).contains(&col) || !(0.0..=5.0).contains(&row) {
            return None;
        }
        Some(self.grid_start().add_days(row as i64 * 7 + col as i64))
    }

    /// Moves the marked day by `days`; the month follows it.
    pub fn move_by(&self, days: i64) {
        let mut s = self.0.borrow_mut();
        s.day = s.day.add_days(days);
        s.month = s.day.first_of_month();
    }

    /// Turns `months` months on; the marked day goes along.
    pub fn turn(&self, months: i32) {
        let mut s = self.0.borrow_mut();
        s.day = s.day.add_months(months);
        s.month = s.day.first_of_month();
    }

    /// Picks `day`: its opener reads it once the calendar closed.
    pub fn pick(&self, day: Day) {
        self.0.borrow_mut().picked = Some(day);
    }

    /// A key, from the calendar or from the part that keeps the focus while it is open.
    pub fn keyed(&self, key: KeyInput) -> Keyed {
        if !key.modifiers.is_empty() {
            return Keyed::Not;
        }
        let (day, month) = (self.day(), self.month());
        match key.key {
            Key::Left => self.move_by(-1),
            Key::Right => self.move_by(1),
            Key::Up => self.move_by(-7),
            Key::Down => self.move_by(7),
            Key::PageUp => self.turn(-1),
            Key::PageDown => self.turn(1),
            Key::Home => self.move_by(month.number() - day.number()),
            Key::End => self.move_by(month.add_months(1).number() - 1 - day.number()),
            Key::Enter | Key::Space => {
                self.pick(day);
                return Keyed::Picked(day);
            }
            Key::Escape => return Keyed::Closed,
            _ => return Keyed::Not,
        }
        Keyed::Moved
    }
}

/// Where a calendar lies: below `under`, else above it when there is no room below and there is above.
pub fn below(whole: Rect, under: Rect) -> Rect {
    let x = under.x.min(whole.width - WIDTH).max(0.0);
    let below = under.bottom() + 2.0;
    if below + HEIGHT > whole.height && under.y - 2.0 - HEIGHT >= 0.0 {
        return Rect::new(x, under.y - 2.0 - HEIGHT, WIDTH, HEIGHT);
    }
    Rect::new(x, below, WIDTH, HEIGHT)
}

/// What a press on the calendar began on.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Press {
    Previous,
    Next,
    On(Day),
}

/// The calendar as an element over the window.
pub struct DayCalendar {
    node: Node,
    calendar: Calendar,
    holder: Option<ElementId>,
    pressed_on: Option<Press>,
}

/// What the person's month buttons do, in their language.
pub fn previous_name() -> String {
    locale::translate("previousMonth", "Previous month")
}

pub fn next_name() -> String {
    locale::translate("nextMonth", "Next month")
}

impl DayCalendar {
    /// The element of `calendar`; `holder` keeps the focus while it is open (a date part), else it takes the focus itself.
    pub fn new(calendar: Calendar, holder: Option<ElementId>) -> DayCalendar {
        DayCalendar { node: Node::new(), calendar, holder, pressed_on: None }
    }

    pub fn calendar(&self) -> &Calendar {
        &self.calendar
    }

    /// Opens a calendar under `under` on the month of `day` (today's for
    /// none), in the words of `formats`. A picked day closes it, and its
    /// `opener` hears it close and takes the day (`Calendar::take_picked`).
    /// Without a `holder` it takes the focus and its keys itself; with one,
    /// the holder keeps the focus and hands it the keys (`Calendar::keyed`).
    /// A press on `anchor` is no press outside. Gives the overlay's id and the calendar.
    #[allow(clippy::too_many_arguments)]
    pub fn open(cx: &mut Cx, under: Rect, day: Option<Day>, formats: &Formats, holder: Option<ElementId>, anchor: Option<ElementId>, opener: Option<ElementId>) -> (ElementId, Calendar) {
        let calendar = Calendar::new(day, formats);
        let place = below(cx.whole(), under);
        calendar.0.borrow_mut().bounds = place;
        let element = DayCalendar::new(calendar.clone(), holder);
        let own = element.node.id();
        let mut overlay = Overlay::new(Box::new(element), place);
        overlay.anchor = anchor;
        overlay.opener = opener;
        // A press outside the calendar closes it and reaches what it was on.
        overlay.through = true;
        let id = cx.open(overlay);
        cx.focus(Some(holder.unwrap_or(own)));
        (id, calendar)
    }

    fn chevron(p: &Painter, area: Rect, way: f32, color: Color) {
        let mut pen = Paint::default();
        pen.set_color(color);
        pen.set_anti_alias(true);
        pen.set_style(PaintStyle::Stroke);
        pen.set_stroke_width(1.6);
        pen.set_stroke_cap(PaintCap::Round);
        let cx = (area.x + area.width / 2.0) as f32;
        let cy = (area.y + area.height / 2.0) as f32;
        // The point of the arrow lies on the side it turns to.
        p.canvas.draw_line((cx - 2.0 * way, cy - 4.0), (cx + 2.0 * way, cy), &pen);
        p.canvas.draw_line((cx + 2.0 * way, cy), (cx - 2.0 * way, cy + 4.0), &pen);
    }
}

impl Element for DayCalendar {
    crate::element_node!();

    fn focusable(&self) -> bool {
        self.holder.is_none()
    }

    fn arrange(&mut self, area: Rect, _cx: &mut Cx) {
        self.node.bounds = area;
        self.calendar.0.borrow_mut().bounds = area;
    }

    fn on_key(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        match self.calendar.keyed(key) {
            Keyed::Not => false,
            Keyed::Moved => {
                cx.invalidate();
                true
            }
            Keyed::Picked(_) | Keyed::Closed => {
                cx.close(self.node.id());
                true
            }
        }
    }

    fn on_pointer_pressed(&mut self, at: PointerInput, cx: &mut Cx) {
        let (x, y) = (at.screen_x, at.screen_y);
        self.pressed_on = if self.calendar.previous().contains(x, y) {
            Some(Press::Previous)
        } else if self.calendar.next().contains(x, y) {
            Some(Press::Next)
        } else {
            self.calendar.day_at(x, y).map(Press::On)
        };
        if self.holder.is_some() {
            cx.focus(self.holder);
        }
    }

    fn on_click(&mut self, cx: &mut Cx) {
        match self.pressed_on.take() {
            Some(Press::Previous) => self.calendar.turn(-1),
            Some(Press::Next) => self.calendar.turn(1),
            Some(Press::On(day)) => {
                self.calendar.pick(day);
                cx.close(self.node.id());
            }
            None => {}
        }
        cx.invalidate();
    }

    fn on_pointer_moved(&mut self, at: PointerInput, cx: &mut Cx) {
        let day = self.calendar.day_at(at.screen_x, at.screen_y);
        let mut s = self.calendar.0.borrow_mut();
        if s.hovered != day {
            s.hovered = day;
            cx.invalidate();
        }
    }

    fn tooltip_at(&self, x: f64, y: f64) -> Option<String> {
        if self.calendar.previous().contains(x, y) {
            return Some(previous_name());
        }
        if self.calendar.next().contains(x, y) {
            return Some(next_name());
        }
        None
    }

    fn draw(&self, p: &Painter) {
        let look = p.look;
        let plain = look.part("list", None);
        let radius = look.radius("input");
        let text = plain.text.unwrap_or_else(|| look.color("text"));
        let muted = look.color("muted");
        let (size, small) = (look.size("text"), look.size("small"));
        let b = self.node.bounds;
        p.fill(b, plain.background.unwrap_or_else(|| look.color("panel")), radius);

        let (previous, next) = (self.calendar.previous(), self.calendar.next());
        let title = Rect::new(previous.right(), b.y + PAD, next.x - previous.right(), HEAD);
        p.text(&self.calendar.title(), title, text, size, "text", true, TextAlign::Center);
        Self::chevron(p, previous, -1.0, text);
        Self::chevron(p, next, 1.0, text);

        for (i, name) in self.calendar.weekdays().iter().enumerate() {
            p.text(name, Rect::new(b.x + PAD + i as f64 * CELL_WIDTH, b.y + PAD + HEAD, CELL_WIDTH, WEEK_ROW), muted, small, "text", false, TextAlign::Center);
        }

        let start = self.calendar.grid_start();
        let s = self.calendar.0.borrow();
        for i in 0..42 {
            let day = start.add_days(i);
            let Some(cell) = self.calendar.cell_of(day).map(|c| c.inset(1.0)) else { continue };
            let mut color = if day.month == s.month.month { text } else { muted };
            if Some(day) == s.chosen {
                p.fill(cell, look.color("accent"), radius);
                color = look.color("on_accent");
            } else if day == s.day {
                p.fill(cell, look.color("selection"), radius);
            } else if Some(day) == s.hovered {
                p.fill(cell, look.color("raised"), radius);
            }
            if day == s.day && Some(day) == s.chosen {
                p.stroke(cell, look.color("focus"), 2.0, radius);
            }
            if day == s.today {
                p.stroke(cell.inset(1.0), look.color(if Some(day) == s.chosen { "on_accent" } else { "accent" }), 1.0, radius);
            }
            p.text(&day.day.to_string(), cell, color, small, "text", day == s.today, TextAlign::Center);
        }
        p.stroke(b, plain.border.unwrap_or_else(|| look.color("border")), look.border(), radius);
    }
}
