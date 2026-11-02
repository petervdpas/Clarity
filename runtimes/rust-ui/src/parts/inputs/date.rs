//! The date part, the same as on every stack (conformance/forms.json,
//! tables.json): a day, or a moment when its field holds a date and time.
//! The day shows in the person's format and typed text is read the same way
//! (or as 2026-10-20) when the input is left or Enter is pressed; a button at
//! its right opens a calendar under it to pick the day from. A moment has its
//! time beside the day, typed as hh:mm on a 24-hour clock (or 9:30 PM, shown
//! again as 21:30); picking or typing
//! a day keeps the time, and an empty moment starts at midnight. Only the
//! person's pick or typed change is a change; the same value again and what
//! code sets are not. Text that is no day or no time is marked and changes
//! nothing; a cleared day clears the value. Bound, it writes 2026-10-20 for
//! a day and 2026-10-20T14:05:00 for a moment; unbound and named, it keeps
//! the same text in its state.
//!
//! The part holds its inputs as elements of their own (the day's, a
//! moment's time, the calendar's button), which share what the part knows
//! (`Core`): each input takes the focus by itself, and leaving it takes what
//! was typed there.

use std::cell::{Ref, RefCell, RefMut};
use std::rc::Rc;

use serde_json::Value;
use skia_safe::{Paint, PaintCap, PaintStyle};

use crate::element::{Cx, Element, ElementId, Node};
use crate::geometry::Rect;
use crate::input::{Key, KeyInput, Modifiers, PointerInput};
use crate::locale;
use crate::markup::PartSpec;
use crate::painter::{Painter, TextAlign};
use crate::part_element;
use crate::parts::inputs::calendar::{Calendar, Day, DayCalendar, Keyed};
use crate::parts::inputs::formats::{format_of, formats_of, FieldFormat, FormatKind, Moment};
use crate::parts::text_editor::TextEditor;
use crate::parts::{Part, PartBase, PartHost};

const BUTTON_WIDTH: f64 = 26.0;
const TIME_WIDTH: f64 = 64.0;
const GAP: f64 = 4.0;

/// A value of the part: its day, and for a moment its time of day.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct At {
    pub day: Day,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
}

impl At {
    fn midnight(day: Day) -> At {
        At { day, hour: 0, minute: 0, second: 0 }
    }

    /// A stored value: 2026-10-20 or 2026-10-20T14:05:00; none for text that is no date.
    pub fn parse(text: &str) -> Option<At> {
        Moment::parse(text.trim()).map(|m| At { day: Day { year: m.year, month: m.month, day: m.day }, hour: m.hour, minute: m.minute, second: m.second })
    }

    /// As stored: the day alone, or the day with its time.
    pub fn stored(&self, moment: bool) -> String {
        if moment {
            format!("{}T{:02}:{:02}:{:02}", self.day.iso(), self.hour, self.minute, self.second)
        } else {
            self.day.iso()
        }
    }
}

/// A typed time of day: 21:30 (or 21.30), or on a 12-hour clock 9:30 PM
/// (am and pm, with or without dots, in any case); empty is midnight; none
/// when it is no time.
pub fn read_time(typed: &str) -> Option<(u32, u32)> {
    let text = typed.trim();
    if text.is_empty() {
        return Some((0, 0));
    }
    let at = text.find([':', '.'])?;
    let hours = &text[..at];
    let rest = &text[at + 1..];
    let minutes: String = rest.chars().take_while(char::is_ascii_digit).collect();
    let half = rest[minutes.len()..].trim().trim_end_matches('.').replace('.', "");
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    if !digits(hours) || hours.len() > 2 || minutes.len() != 2 {
        return None;
    }
    let (mut h, m): (u32, u32) = (hours.parse().ok()?, minutes.parse().ok()?);
    if !half.is_empty() {
        let pm = half.eq_ignore_ascii_case("pm");
        if !pm && !half.eq_ignore_ascii_case("am") {
            return None;
        }
        if !(1..=12).contains(&h) {
            return None;
        }
        h = h % 12 + if pm { 12 } else { 0 };
    }
    (h < 24 && m < 60).then_some((h, m))
}

/// Which of its inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Which {
    Day,
    Time,
}

/// What the part knows, shared by the part and its inputs.
struct Core {
    host: Rc<dyn PartHost>,
    spec: PartSpec,
    /// The part's own id: it opens the calendar and hears it close.
    part: ElementId,
    /// The day's input, which keeps the focus while the calendar is open.
    day_input: ElementId,
    day: TextEditor,
    time: Option<TextEditor>,
    own: String,
    shown_day: Option<String>,
    shown_time: Option<String>,
    bad: Option<String>,
    bad_time: Option<String>,
    moment: bool,
    open: Option<ElementId>,
    calendar: Option<Calendar>,
}

impl Core {
    fn bound(&self) -> Option<&str> {
        self.spec.text("bind").filter(|b| !b.is_empty())
    }

    fn state(&self) -> Option<clarity_runtime::ui::PartState> {
        self.host.state(self.spec.name())
    }

    fn enabled(&self) -> bool {
        self.state().and_then(|s| s.enabled).unwrap_or(true)
    }

    fn read_only(&self) -> bool {
        self.spec.flag("readonly") || self.state().and_then(|s| s.readonly).unwrap_or(false)
    }

    fn usable(&self) -> bool {
        self.enabled() && !self.read_only()
    }

    fn fire(&self, event: &str) {
        for call in self.spec.on(event) {
            self.host.run(self.spec.name(), call);
        }
    }

    /// The value; none when it is empty or no date.
    fn value(&self) -> Option<At> {
        match self.bound() {
            Some(path) => match self.host.read(path) {
                Some(Value::String(s)) => At::parse(&s),
                _ => None,
            },
            None => At::parse(&self.state().and_then(|s| s.text).unwrap_or_else(|| self.own.clone())),
        }
    }

    /// How the day shows: the field's named format (iso, long) when it has one.
    fn day_format(&self) -> FieldFormat {
        let named = self.bound().and_then(|p| format_of(self.host.as_ref(), p)).and_then(|f| f.format);
        FieldFormat { format: named, ..FieldFormat::new(FormatKind::Date) }
    }

    fn editor(&self, which: Which) -> Option<&TextEditor> {
        match which {
            Which::Day => Some(&self.day),
            Which::Time => self.time.as_ref(),
        }
    }

    fn editor_mut(&mut self, which: Which) -> Option<&mut TextEditor> {
        match which {
            Which::Day => Some(&mut self.day),
            Which::Time => self.time.as_mut(),
        }
    }

    /// The inputs follow what code set: enabled, read-only, marked.
    fn refresh(&mut self) {
        let (enabled, read_only) = (self.enabled(), self.read_only());
        let error = self.state().and_then(|s| s.error).is_some_and(|e| !e.show().is_empty());
        let (bad, bad_time) = (self.bad.is_some(), self.bad_time.is_some());
        self.day.enabled = enabled;
        self.day.read_only = read_only;
        self.day.marked = bad || error;
        if let Some(time) = self.time.as_mut() {
            time.enabled = enabled;
            time.read_only = read_only;
            time.marked = bad_time || error;
        }
    }

    /// A value set from elsewhere shows as it is, and is no longer marked.
    fn sync(&mut self) {
        let value = self.value();
        let formats = formats_of(self.host.as_ref());
        let day = value.map(|v| formats.show(&self.day_format(), &Value::String(v.day.iso()))).unwrap_or_default();
        if self.shown_day.as_deref() != Some(day.as_str()) {
            self.bad = None;
            self.day.set_text(&day);
            self.shown_day = Some(day);
        }
        if let Some(time) = self.time.as_mut() {
            let clock = value.map(|v| format!("{:02}:{:02}", v.hour, v.minute)).unwrap_or_default();
            if self.shown_time.as_deref() != Some(clock.as_str()) {
                self.bad_time = None;
                time.set_text(&clock);
                self.shown_time = Some(clock);
            }
        }
    }

    /// Takes the typed day: a change when it is a day that differs; cleared, the value is emptied.
    fn take(&mut self) {
        self.sync();
        let typed = self.day.text();
        if self.bad.is_none() && self.shown_day.as_deref() == Some(typed.as_str()) {
            return;
        }
        // As Avalonia's DateInput: no input refuses an empty value, required or not; the form says it is missing on save.
        if typed.trim().is_empty() {
            self.bad = None;
            self.offer(None);
            return;
        }
        let read = formats_of(self.host.as_ref()).read_date(&typed).and_then(|(y, m, d)| Day::new(y, m, d));
        let Some(day) = read else {
            self.bad = Some(typed);
            return;
        };
        self.bad = None;
        self.offer(Some(self.keeping_time(day)));
    }

    /// A moment's typed time; without a day it waits for one.
    fn take_time(&mut self) {
        let Some(typed) = self.time.as_ref().map(|t| t.text()) else { return };
        self.sync();
        if self.bad_time.is_none() && self.shown_time.as_deref() == Some(typed.as_str()) {
            return;
        }
        let Some((hour, minute)) = read_time(&typed) else {
            self.bad_time = Some(typed);
            return;
        };
        self.bad_time = None;
        if let Some(value) = self.value() {
            self.offer(Some(At { day: value.day, hour, minute, second: 0 }));
        }
    }

    /// `day`, at the time a moment has.
    fn keeping_time(&self, day: Day) -> At {
        match (self.moment, self.value()) {
            (true, Some(v)) => At { day, ..v },
            _ => At::midnight(day),
        }
    }

    /// The person picked `day` from the calendar: a moment keeps its time.
    fn pick(&mut self, day: Day) {
        if !self.usable() {
            return;
        }
        self.bad = None;
        self.offer(Some(self.keeping_time(day)));
    }

    /// Makes `next` the value when it differs; else the inputs show the value again.
    fn offer(&mut self, next: Option<At>) {
        let next = if self.moment { next } else { next.map(|n| At::midnight(n.day)) };
        let now = if self.moment { self.value() } else { self.value().map(|v| At::midnight(v.day)) };
        if next == now {
            self.shown_day = None;
            self.shown_time = None;
            self.sync();
            return;
        }
        let stored = next.map(|n| n.stored(self.moment));
        let name = self.spec.name().map(str::to_string);
        if let Some(path) = self.bound().map(str::to_string) {
            self.host.write(name.as_deref().unwrap_or(""), &path, stored.map(Value::String).unwrap_or(Value::Null));
        } else if let Some(name) = name {
            let text = stored.unwrap_or_default();
            crate::parts::change_state(&*self.host, &name, |s| s.text = Some(text.clone()));
            self.host.edited(&name);
        } else {
            self.own = stored.unwrap_or_default();
        }
        self.sync();
        self.fire("change");
    }

    /// Opens the calendar under the day's input, on the value's month; the day's input keeps the focus.
    fn open_calendar(&mut self, cx: &mut Cx) {
        if !self.usable() || self.open.is_some() {
            return;
        }
        let under = self.day.node().bounds;
        let formats = formats_of(self.host.as_ref());
        let day = self.value().map(|v| v.day);
        let (id, calendar) = DayCalendar::open(cx, under, day, &formats, Some(self.day_input), Some(self.part), Some(self.part));
        self.open = Some(id);
        self.calendar = Some(calendar);
    }

    fn close(&mut self, cx: &mut Cx) {
        if let Some(open) = self.open {
            cx.close(open);
        }
    }

    fn toggle(&mut self, cx: &mut Cx) {
        if self.open.is_some() {
            self.close(cx);
        } else {
            self.open_calendar(cx);
        }
    }

    /// The calendar closed: the day picked in it, if any, is taken.
    fn closed(&mut self, overlay: ElementId) {
        if self.open != Some(overlay) {
            return;
        }
        self.open = None;
        if let Some(day) = self.calendar.take().and_then(|c| c.take_picked()) {
            self.pick(day);
        }
    }

    /// Keys in the day's input: while the calendar is open they work it; Down opens it.
    fn day_key(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        if let Some(calendar) = self.calendar.clone() {
            match calendar.keyed(key) {
                Keyed::Not => {}
                Keyed::Moved => {
                    cx.invalidate();
                    return true;
                }
                Keyed::Picked(_) | Keyed::Closed => {
                    self.close(cx);
                    return true;
                }
            }
        }
        if key.key == Key::Down && key.modifiers.is_empty() && self.usable() {
            self.open_calendar(cx);
            return true;
        }
        false
    }
}

/// One of a date part's inputs: the day's or a moment's time.
pub struct DateInput {
    node: Node,
    core: Rc<RefCell<Core>>,
    which: Which,
}

impl DateInput {
    fn ready(&mut self) {
        let mut core = self.core.borrow_mut();
        core.refresh();
        let node = &self.node;
        if let Some(editor) = core.editor_mut(self.which) {
            editor.mirror(node);
        }
    }

    fn take(&self) {
        let mut core = self.core.borrow_mut();
        match self.which {
            Which::Day => core.take(),
            Which::Time => core.take_time(),
        }
    }
}

impl Element for DateInput {
    crate::element_node!();

    fn focusable(&self) -> bool {
        self.core.borrow().enabled()
    }

    fn typing(&self) -> bool {
        true
    }

    fn take_typed(&mut self, _cx: &mut Cx) -> bool {
        self.take();
        true
    }

    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.node.bounds = area;
        self.ready();
        if let Some(editor) = self.core.borrow_mut().editor_mut(self.which) {
            editor.place(area, cx);
        }
    }

    fn draw(&self, p: &Painter) {
        if let Some(editor) = self.core.borrow().editor(self.which) {
            editor.paint(p);
        }
    }

    fn on_pointer_pressed(&mut self, at: PointerInput, cx: &mut Cx) {
        self.ready();
        if let Some(editor) = self.core.borrow_mut().editor_mut(self.which) {
            editor.pressed(at, cx);
        }
    }

    fn on_pointer_moved(&mut self, at: PointerInput, cx: &mut Cx) {
        if let Some(editor) = self.core.borrow_mut().editor_mut(self.which) {
            editor.dragged(at, cx);
        }
    }

    fn on_pointer_released(&mut self, _at: PointerInput, _cx: &mut Cx) {
        if let Some(editor) = self.core.borrow_mut().editor_mut(self.which) {
            editor.released();
        }
    }

    fn on_wheel(&mut self, _dx: f64, dy: f64, _held: Modifiers, cx: &mut Cx) -> bool {
        self.core.borrow_mut().editor_mut(self.which).is_some_and(|e| e.wheel(dy, cx))
    }

    fn on_focus_changed(&mut self, focused: bool, cx: &mut Cx) {
        if let Some(editor) = self.core.borrow_mut().editor_mut(self.which) {
            editor.focus_moved(focused, cx);
        }
        if focused {
            self.core.borrow().fire("enter");
            return;
        }
        // A press in its own calendar takes the focus for a moment and gives it back.
        if cx.focused().is_some() {
            self.core.borrow_mut().close(cx);
        }
        self.take();
        self.core.borrow().fire("leave");
    }

    fn on_key(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        self.ready();
        if self.which == Which::Day && self.core.borrow_mut().day_key(key, cx) {
            return true;
        }
        // Enter takes the text; the form it is in, or the window's default button, does the rest.
        if key.key == Key::Enter && key.modifiers.is_empty() {
            self.take();
            cx.invalidate();
            return false;
        }
        self.core.borrow_mut().editor_mut(self.which).is_some_and(|e| e.key(key, cx))
    }

    fn on_text(&mut self, text: &str, cx: &mut Cx) {
        self.ready();
        if let Some(editor) = self.core.borrow_mut().editor_mut(self.which) {
            editor.typed(text, cx);
        }
    }
}

/// The small button at a date input's right that opens its calendar: a calendar sheet.
pub struct CalendarButton {
    node: Node,
    core: Rc<RefCell<Core>>,
}

impl CalendarButton {
    /// What it does, in the person's language.
    pub fn name() -> String {
        locale::translate("openCalendar", "Calendar")
    }
}

impl Element for CalendarButton {
    crate::element_node!();

    fn on_click(&mut self, cx: &mut Cx) {
        let mut core = self.core.borrow_mut();
        if core.usable() {
            core.toggle(cx);
        }
    }

    fn tooltip_at(&self, _x: f64, _y: f64) -> Option<String> {
        Some(Self::name())
    }

    fn draw(&self, p: &Painter) {
        let enabled = self.core.borrow().usable();
        draw_calendar_sheet(p, self.node.bounds, enabled, self.node.hovered, self.node.pressed);
    }
}

/// The button that opens a calendar, drawn: a calendar sheet, its ground raised while the pointer is on it.
pub fn draw_calendar_sheet(p: &Painter, b: Rect, enabled: bool, hovered: bool, pressed: bool) {
    let look = p.look;
    if enabled && (hovered || pressed) {
        p.fill(b, look.color(if pressed { "accent_soft" } else { "raised" }), look.radius("input"));
    }
    let mut pen = Paint::default();
    pen.set_color(look.color(if enabled { "text" } else { "muted" }));
    pen.set_anti_alias(true);
    pen.set_style(PaintStyle::Stroke);
    pen.set_stroke_width(1.4);
    pen.set_stroke_cap(PaintCap::Round);
    let cx = (b.x + b.width / 2.0) as f32;
    let cy = (b.y + b.height / 2.0) as f32;
    let canvas = p.canvas;
    canvas.draw_round_rect(skia_safe::Rect::new(cx - 6.0, cy - 5.0, cx + 6.0, cy + 6.0), 1.5, 1.5, &pen);
    canvas.draw_line((cx - 6.0, cy - 1.0), (cx + 6.0, cy - 1.0), &pen);
    canvas.draw_line((cx - 3.0, cy - 7.0), (cx - 3.0, cy - 4.0), &pen);
    canvas.draw_line((cx + 3.0, cy - 7.0), (cx + 3.0, cy - 4.0), &pen);
}

pub struct DatePart {
    base: PartBase,
    core: Rc<RefCell<Core>>,
}

pub fn make(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    Box::new(DatePart::new(spec, host))
}

/// Where its children are, by the order they were added.
const DAY: usize = 0;
const BUTTON: usize = 1;
const TIME: usize = 2;

impl DatePart {
    pub fn new(spec: PartSpec, host: Rc<dyn PartHost>) -> DatePart {
        let mut base = PartBase::new(spec, host);
        let moment = base.bound().and_then(|p| format_of(base.host.as_ref(), p)).is_some_and(|f| f.kind == FormatKind::DateTime);
        let mut day = TextEditor::new(false);
        day.placeholder = base.spec.text("placeholder").map(|p| base.host.text(Some(p)));
        day.right_inset = BUTTON_WIDTH + 2.0;
        let time = moment.then(|| {
            let mut time = TextEditor::new(false);
            time.placeholder = Some("hh:mm".to_string());
            time.align = TextAlign::Center;
            time
        });
        let day_node = Node::new();
        let core = Rc::new(RefCell::new(Core {
            host: base.host.clone(),
            spec: base.spec.clone(),
            part: base.node.id(),
            day_input: day_node.id(),
            day,
            time,
            own: String::new(),
            shown_day: None,
            shown_time: None,
            bad: None,
            bad_time: None,
            moment,
            open: None,
            calendar: None,
        }));
        base.node.add(Box::new(DateInput { node: day_node, core: core.clone(), which: Which::Day }));
        base.node.add(Box::new(CalendarButton { node: Node::new(), core: core.clone() }));
        if moment {
            base.node.add(Box::new(DateInput { node: Node::new(), core: core.clone(), which: Which::Time }));
        }
        core.borrow_mut().sync();
        DatePart { base, core }
    }

    /// Whether it holds a moment: its field is a date and time.
    pub fn is_moment(&self) -> bool {
        self.core.borrow().moment
    }

    /// Where the day is typed.
    pub fn editor(&self) -> Ref<'_, TextEditor> {
        Ref::map(self.core.borrow(), |c| &c.day)
    }

    pub fn editor_mut(&self) -> RefMut<'_, TextEditor> {
        RefMut::map(self.core.borrow_mut(), |c| &mut c.day)
    }

    /// Where a moment's time is typed; none for a day.
    pub fn time(&self) -> Option<Ref<'_, TextEditor>> {
        Ref::filter_map(self.core.borrow(), |c| c.time.as_ref()).ok()
    }

    pub fn time_mut(&self) -> Option<RefMut<'_, TextEditor>> {
        RefMut::filter_map(self.core.borrow_mut(), |c| c.time.as_mut()).ok()
    }

    /// The id of an input, to give it the focus.
    pub fn input(&self, which: Which) -> Option<ElementId> {
        let index = if which == Which::Day { DAY } else { TIME };
        self.base.node.children().get(index).map(|c| c.node().id())
    }

    /// Where an input is, on the screen.
    pub fn input_bounds(&self, which: Which) -> Option<Rect> {
        let index = if which == Which::Day { DAY } else { TIME };
        self.base.node.children().get(index).map(|c| c.node().bounds)
    }

    /// Where the button that opens the calendar is.
    pub fn button(&self) -> Rect {
        self.base.node.children()[BUTTON].node().bounds
    }

    /// What was typed as the day while it is no day; none when the input shows its value.
    pub fn bad(&self) -> Option<String> {
        self.core.borrow().bad.clone()
    }

    /// What was typed as the time while it is no time.
    pub fn bad_time(&self) -> Option<String> {
        self.core.borrow().bad_time.clone()
    }

    /// Its value; none when it is empty.
    pub fn value(&self) -> Option<At> {
        self.core.borrow().value()
    }

    /// Its calendar, while it is open.
    pub fn calendar(&self) -> Option<Calendar> {
        self.core.borrow().calendar.clone()
    }

    /// Takes the typed day; see the module's words.
    pub fn take(&self) {
        self.core.borrow_mut().take();
    }

    /// Takes a moment's typed time.
    pub fn take_time(&self) {
        self.core.borrow_mut().take_time();
    }

    /// The person picked `day` from the calendar.
    pub fn pick(&self, day: Day) {
        self.core.borrow_mut().pick(day);
    }
}

impl Element for DatePart {
    part_element!();

    fn take_typed(&mut self, _cx: &mut Cx) -> bool {
        let mut core = self.core.borrow_mut();
        core.take();
        core.take_time();
        true
    }

    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.base.node.bounds = area;
        self.core.borrow_mut().sync();
        let moment = self.is_moment();
        let day = if moment { Rect { width: (area.width - TIME_WIDTH - GAP).max(0.0), ..area } } else { area };
        let children = self.base.node.children_mut();
        children[DAY].arrange(day, cx);
        children[BUTTON].arrange(Rect::new(day.right() - 2.0 - BUTTON_WIDTH, day.y + 2.0, BUTTON_WIDTH, (day.height - 4.0).max(0.0)), cx);
        if let Some(time) = children.get_mut(TIME) {
            time.arrange(Rect::new(day.right() + GAP, area.y, TIME_WIDTH.min(area.width), area.height), cx);
        }
    }

    fn on_overlay_closed(&mut self, overlay: ElementId, cx: &mut Cx) {
        self.core.borrow_mut().closed(overlay);
        cx.invalidate();
    }
}

impl Part for DatePart {
    fn base(&self) -> &PartBase {
        &self.base
    }
}
