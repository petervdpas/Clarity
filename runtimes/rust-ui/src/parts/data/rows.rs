//! What the parts that show the records of a list share (a list, a table, a
//! tree and a tree table), as C# DataRows.cs: the rows read from the
//! window's data, the row the person chose, their own border unless a
//! container frames them (DESIGN 7.9), and moving rows by hand (DESIGN 5.30,
//! conformance/reorder.json): a drag of a row onto another and Alt with an
//! arrow key, written back as one edit after which `reorder` runs. The rows
//! scroll in their own place (`Scroller`), with the wheel and a bar; evenly
//! spaced rows are placed, hit and kept in sight here once for all.

use clarity_runtime::reorder::{self, DropAt, Move, MoveKey, Row};
use serde_json::Value;

use crate::geometry::Rect;
use crate::input::{Key, KeyInput, Modifiers, PointerInput};
use crate::locale;
use crate::painter::Painter;
use crate::parts::inputs::formats::{FieldFormat, FormatKind, Formats};
use crate::parts::{Choice, PartBase};
use crate::theme::LookValues;

// -- Fields of records --------------------------------------------------------

/// The value of `field` of a record: its key as written, else in any case
/// (`finishedBy` for `finished_by` too); none when it has none.
pub fn field<'a>(row: &'a Value, field: &str) -> Option<&'a Value> {
    let record = row.as_object()?;
    if let Some(value) = record.get(field) {
        return Some(value);
    }
    let plain = |s: &str| s.replace('_', "").to_lowercase();
    let wanted = plain(field);
    record.iter().find(|(k, _)| plain(k) == wanted).map(|(_, v)| v)
}

/// The value of `field` of a record, `null` when it has none.
pub fn value_of(row: &Value, name: &str) -> Value {
    field(row, name).cloned().unwrap_or(Value::Null)
}

/// The path of a field of the record at `place` of the list at `list`, as a part writes it: `tasks[0].done`.
pub fn path(list: &str, place: usize, field: &str) -> String {
    let mut path = String::with_capacity(list.len() + field.len() + 8);
    path.push_str(list);
    path.push('[');
    path.push_str(&place.to_string());
    path.push_str("].");
    path.push_str(field);
    path
}

/// The path of a field of every record of a list, whose choices a column offers: `tasks.priority`.
pub fn column_path(list: &str, field: &str) -> String {
    let mut path = String::with_capacity(list.len() + field.len() + 1);
    path.push_str(list);
    path.push('.');
    path.push_str(field);
    path
}

/// Whether a choice's value is the value a field holds: `1` and `1.0` are one
/// number, a number and its text are one, a missing value is null.
pub fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Null, _) | (_, Value::Null) => false,
        (Value::Number(x), Value::Number(y)) => x == y || x.as_f64() == y.as_f64(),
        (Value::Number(x), Value::String(s)) | (Value::String(s), Value::Number(x)) => {
            s.trim().parse::<f64>().ok().is_some_and(|n| Some(n) == x.as_f64())
        }
        _ => a == b,
    }
}

/// A value as the person reads it: text as it is, yes and no in their
/// language, a list as its items, numbers as they are written.
pub fn shown_text(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        Value::Bool(true) => locale::translate("yes", "Yes"),
        Value::Bool(false) => locale::translate("no", "No"),
        Value::Number(n) => n.to_string(),
        Value::Array(items) => items.iter().map(shown_text).collect::<Vec<_>>().join(", "),
        Value::Object(_) => value.to_string(),
    }
}

/// A value of a field as the person reads it: a choice by its label, else as `shown_text` writes it.
pub fn shown_of(value: &Value, choices: &[Choice]) -> String {
    if choices.is_empty() {
        return shown_text(value);
    }
    match choices.iter().find(|c| same(&c.value, value)) {
        Some(choice) => choice.label.clone(),
        None if value.is_null() => String::new(),
        None => shown_text(value),
    }
}

/// A value as the person reads it in their `formats` (C# ShownText.Of): a
/// number with the person's thousands and decimal mark, as `format` says or
/// else as an integer or a decimal; anything else as `shown_text` writes it.
pub fn shown_formatted(value: &Value, format: Option<&FieldFormat>, formats: &Formats) -> String {
    match value {
        Value::Number(n) => {
            let kind = if n.is_i64() || n.is_u64() { FormatKind::Integer } else { FormatKind::Decimal };
            let fallback = FieldFormat::new(kind);
            let format = format.filter(|f| !matches!(f.kind, FormatKind::Date | FormatKind::DateTime)).unwrap_or(&fallback);
            formats.show(format, value)
        }
        Value::Array(items) => items.iter().map(|i| shown_formatted(i, None, formats)).collect::<Vec<_>>().join(", "),
        _ => shown_text(value),
    }
}

// -- Scrolling --------------------------------------------------------------

/// The rows' own place, which scrolls: the wheel turns it, and a bar at its
/// right shows how much is in view and moves it (as C# ScrollView).
#[derive(Debug, Clone, Default)]
pub struct Scroller {
    /// Where the rows show on the screen.
    pub bounds: Rect,
    /// How high all the rows are together.
    pub content: f64,
    scroll_y: f64,
    /// Where on the thumb the pointer holds it; none while it holds no thumb.
    grab: Option<f64>,
    /// Whether a press went down on the bar.
    holding: bool,
}

impl Scroller {
    /// How wide the bar is.
    pub const BAR: f64 = 8.0;
    /// How long the thumb is at least.
    const LEAST_THUMB: f64 = 24.0;

    /// How far the rows are scrolled, from the top.
    pub fn scrolled(&self) -> f64 {
        self.scroll_y
    }

    /// How far the rows scroll at most.
    pub fn max(&self) -> f64 {
        (self.content - self.bounds.height).max(0.0)
    }

    /// Takes its place for rows of `content` height; what is scrolled stays within it.
    pub fn place(&mut self, bounds: Rect, content: f64) {
        self.bounds = bounds;
        self.content = content.max(0.0);
        self.scroll_y = self.scroll_y.clamp(0.0, self.max());
    }

    /// Scrolls to `y`, within what it scrolls; true when it moved.
    pub fn scroll_to(&mut self, y: f64) -> bool {
        let was = self.scroll_y;
        self.scroll_y = y.clamp(0.0, self.max());
        self.scroll_y != was
    }

    /// The wheel turned: true when it scrolled.
    pub fn wheel(&mut self, dy: f64) -> bool {
        self.scroll_to(self.scroll_y + dy)
    }

    /// Scrolls as little as it takes to show the place from `top` down `height`, in the content.
    pub fn scroll_into(&mut self, top: f64, height: f64) {
        let mut y = self.scroll_y;
        if top < y {
            y = top;
        } else if top + height > y + self.bounds.height {
            y = top + height - self.bounds.height;
        }
        self.scroll_to(y);
    }

    /// The top of the content on the screen.
    pub fn content_top(&self) -> f64 {
        self.bounds.y - self.scroll_y
    }

    /// Its bar, while the rows scroll.
    pub fn bar(&self) -> Option<Rect> {
        (self.max() > 0.0).then(|| Rect::new(self.bounds.right() - Self::BAR, self.bounds.y, Self::BAR, self.bounds.height))
    }

    fn thumb_length(&self) -> f64 {
        let track = self.bounds.height;
        if self.content <= 0.0 { track } else { track.min(Self::LEAST_THUMB.max(track * track / self.content)) }
    }

    fn thumb_at(&self) -> f64 {
        let max = self.max();
        if max <= 0.0 { 0.0 } else { (self.bounds.height - self.thumb_length()) * self.scroll_y / max }
    }

    /// A press at a point: on the bar it takes the thumb, or moves a place's
    /// height toward the press; true when it was on the bar.
    pub fn press(&mut self, x: f64, y: f64) -> bool {
        let Some(bar) = self.bar().filter(|b| b.contains(x, y)) else { return false };
        self.holding = true;
        let along = y - bar.y;
        let at = self.thumb_at();
        if along >= at && along < at + self.thumb_length() {
            self.grab = Some(along - at);
        } else {
            self.grab = None;
            let page = self.bounds.height;
            self.scroll_to(self.scroll_y + if along < at { -page } else { page });
        }
        true
    }

    /// The pointer moved while it holds the bar: the thumb follows; true when it holds it.
    pub fn drag(&mut self, y: f64) -> bool {
        if !self.holding {
            return false;
        }
        if let Some(grab) = self.grab {
            let travel = self.bounds.height - self.thumb_length();
            let at = y - self.bounds.y - grab;
            self.scroll_to(if travel <= 0.0 { 0.0 } else { at / travel * self.max() });
        }
        true
    }

    /// The press ended: true when it was on the bar.
    pub fn release(&mut self) -> bool {
        let was = self.holding;
        self.holding = false;
        self.grab = None;
        was
    }

    /// Draws its thumb, while the rows scroll.
    pub fn draw(&self, p: &Painter, hovered: bool) {
        let Some(bar) = self.bar() else { return };
        let thumb = Rect::new(bar.x + 2.0, bar.y + self.thumb_at(), bar.width - 4.0, self.thumb_length());
        let color = if self.holding || hovered { "muted" } else { "strong_border" };
        p.fill(thumb, p.look.color(color), thumb.width / 2.0);
    }

    // Rows that are each `pitch` apart, the first at the top of the content.

    /// The box on the screen of the row at `index` in the content, in sight or not.
    pub fn row_on_screen(&self, index: usize, pitch: f64, height: f64) -> Rect {
        Rect::new(self.bounds.x, self.content_top() + index as f64 * pitch, self.bounds.width, height)
    }

    /// The box on the screen of the row at `index`; none while it is scrolled out of sight.
    pub fn row_in_sight(&self, index: usize, pitch: f64, height: f64) -> Option<Rect> {
        let row = self.row_on_screen(index, pitch, height);
        (row.bottom() > self.bounds.y && row.y < self.bounds.bottom()).then_some(row)
    }

    /// The index of the row at `y` on the screen, of `count`; none outside the rows' place.
    pub fn index_at(&self, y: f64, pitch: f64, count: usize) -> Option<usize> {
        if y < self.bounds.y || y >= self.bounds.bottom() || pitch <= 0.0 {
            return None;
        }
        let at = ((y - self.content_top()) / pitch).floor();
        (at >= 0.0 && (at as usize) < count).then_some(at as usize)
    }

    /// The indexes of the rows in sight, from the first up to but not with the end.
    pub fn in_sight(&self, pitch: f64, count: usize) -> (usize, usize) {
        if pitch <= 0.0 {
            return (0, 0);
        }
        let first = ((self.scroll_y / pitch).floor() as usize).min(count);
        let end = (((self.scroll_y + self.bounds.height) / pitch).ceil() as usize).min(count);
        (first, end.max(first))
    }
}

// -- Rows, the chosen one and moving them -----------------------------------

/// How far the pointer goes before a press is a drag.
const DRAG_START: f64 = 4.0;

/// What a list, table and tree keep of their rows.
#[derive(Default)]
pub struct Rows {
    /// The records shown, in the order of the data.
    pub records: Vec<Value>,
    read_at: Option<u64>,
    /// The chosen row's place, kept by a part without a name.
    own: Option<usize>,
    /// The chosen row as the part last saw it.
    seen: Option<usize>,
    pressed_row: Option<(usize, f64, f64)>,
    dragging: bool,
    /// Where a dragged row lands now: on the row at a place, before, after or into it.
    pub drop: Option<(usize, DropAt)>,
    pub scroller: Scroller,
    /// Where the pointer was last over the part, on the screen.
    pub pointer: Option<(f64, f64)>,
}

/// Where a drop at `y` from the top of a row of `height` lands: before or
/// after it, or in a tree into it (its middle half).
pub fn drop_where(y: f64, height: f64, tree: bool) -> DropAt {
    let part = y / height.max(1.0);
    if !tree {
        return if part < 0.5 { DropAt::Before } else { DropAt::After };
    }
    if part < 0.25 {
        DropAt::Before
    } else if part > 0.75 {
        DropAt::After
    } else {
        DropAt::Into
    }
}

/// The row a key moves the choice to, of `count` rows with the row at `at`
/// chosen: Up and Down one row (the first when none is chosen), Home and End
/// the first and last, Page Up and Page Down `page` rows; none for another key.
pub fn moved_to(key: Key, at: Option<usize>, count: usize, page: usize) -> Option<usize> {
    if count == 0 {
        return None;
    }
    let at = at.map(|a| a as i64);
    let next = match key {
        Key::Down => at.map_or(0, |a| a + 1),
        Key::Up => at.map_or(0, |a| a - 1),
        Key::Home => 0,
        Key::End => count as i64 - 1,
        Key::PageDown => at.unwrap_or(-1) + page as i64,
        Key::PageUp => at.unwrap_or(0) - page as i64,
        _ => return None,
    };
    Some(next.clamp(0, count as i64 - 1) as usize)
}

/// The move Alt and an arrow key ask for, or none.
pub fn move_key(key: KeyInput) -> Option<MoveKey> {
    if !key.alt() || key.control() || key.meta() {
        return None;
    }
    match key.key {
        Key::Up => Some(MoveKey::Up),
        Key::Down => Some(MoveKey::Down),
        Key::Left => Some(MoveKey::Left),
        Key::Right => Some(MoveKey::Right),
        _ => None,
    }
}

impl Rows {
    /// Reads the records again when the window's values changed (or never
    /// were read); a chosen place past the rows is chosen no more. True when it read them.
    pub fn read(&mut self, base: &PartBase) -> bool {
        let changes = base.host.changes();
        if self.read_at == Some(changes) {
            return false;
        }
        self.records = base.bound().map(|b| base.host.rows(b)).unwrap_or_default();
        let held = match base.name() {
            Some(name) => base.host.selected(name),
            None => self.own,
        };
        if held.is_some_and(|c| c >= self.records.len()) {
            self.set_chosen(base, None);
        }
        self.read_at = Some(base.host.changes());
        true
    }

    /// Reads the records again now, whatever changed.
    pub fn read_now(&mut self, base: &PartBase) {
        self.read_at = None;
        self.read(base);
    }

    /// The place of the chosen record in the data; none for none. A named
    /// part's is the window's (`PartHost::selected`), which handlers read and
    /// code sets; an unnamed one keeps its own.
    pub fn chosen(&self, base: &PartBase) -> Option<usize> {
        let held = match base.name() {
            Some(name) => base.host.selected(name),
            None => self.own,
        };
        held.filter(|c| *c < self.records.len())
    }

    /// Chooses a row (or none) as code does: no `select`.
    pub fn set_chosen(&mut self, base: &PartBase, place: Option<usize>) {
        let place = place.filter(|p| *p < self.records.len());
        match base.name() {
            Some(name) => base.host.select(name, place),
            None => self.own = place,
        }
        self.seen = place;
    }

    /// The chosen row when code chose another one since the part last
    /// looked: the part then shows it (opens its tree, turns to its page, scrolls to it).
    pub fn chosen_by_code(&mut self, base: &PartBase) -> Option<usize> {
        let now = self.chosen(base);
        if now == self.seen {
            return None;
        }
        self.seen = now;
        now
    }

    /// The person chose the record at `place`: `select` runs when it is
    /// another one. True when the choice changed.
    pub fn choose(&mut self, base: &PartBase, place: usize) -> bool {
        if place >= self.records.len() || self.chosen(base) == Some(place) {
            return false;
        }
        self.set_chosen(base, Some(place));
        base.fire("select");
        true
    }

    /// The field the rows are numbered in when the person moves them (`reorder by position`); none, and they stay put.
    pub fn reorder_by(base: &PartBase) -> Option<String> {
        base.spec.text("reorder").filter(|r| !r.is_empty()).map(str::to_string)
    }

    /// Whether the person moves rows now.
    pub fn moves(base: &PartBase) -> bool {
        Self::reorder_by(base).is_some() && base.bound().is_some() && base.enabled() && !base.read_only()
    }

    /// Whether a press became a drag.
    pub fn dragging(&self) -> bool {
        self.dragging
    }

    /// A press on the row at `place`, which may become a drag.
    pub fn press_row(&mut self, base: &PartBase, place: usize, at: PointerInput) {
        self.pressed_row = Self::moves(base).then_some((place, at.screen_x, at.screen_y));
        self.dragging = false;
        self.drop = None;
    }

    /// The pointer moved while pressed: a drag past a few pixels, marking
    /// where the row lands; `over` is the row under the pointer with its box.
    pub fn drag_to(&mut self, at: PointerInput, over: Option<(usize, Rect)>, tree: bool) -> bool {
        let Some((from, x, y)) = self.pressed_row else { return false };
        if !self.dragging && (at.screen_x - x).abs() + (at.screen_y - y).abs() < DRAG_START {
            return false;
        }
        self.dragging = true;
        self.drop = over.filter(|(place, _)| *place != from).map(|(place, row)| (place, drop_where(at.screen_y - row.y, row.height, tree)));
        true
    }

    /// The press ended: a drag gives the move it asks for, or none; the bool says whether it was a drag.
    pub fn drop_row(&mut self) -> (bool, Option<Move>) {
        let was = self.dragging;
        let row = self.pressed_row.take();
        let drop = self.drop.take();
        self.dragging = false;
        match (was, row, drop) {
            (true, Some((row, _, _)), Some((target, at))) => (true, Some(Move::Drag { row, at, target })),
            _ => (was, None),
        }
    }

    /// Alt and an arrow key move the chosen row: the move, or none (the bool says whether the key was one of those).
    pub fn key_move(&self, base: &PartBase, key: KeyInput) -> (bool, Option<Move>) {
        if !Self::moves(base) {
            return (false, None);
        }
        let Some(k) = move_key(key) else { return (false, None) };
        (true, self.chosen(base).map(|row| Move::Key { row, key: k }))
    }

    /// A move of a row: each record gets its number (and in a tree its
    /// parent in `parent`), the list its new order, written as one edit; the
    /// moved row is chosen and `reorder` runs. A move that changes nothing
    /// does nothing. The moved row's place after, when it moved.
    pub fn apply(&mut self, base: &PartBase, mv: Move, parent: Option<&str>) -> Option<usize> {
        let by = Self::reorder_by(base)?;
        let bound = base.bound()?.to_string();
        let given: Vec<Row> = self
            .records
            .iter()
            .map(|r| Row { id: value_of(r, "id"), parent: parent.map(|p| value_of(r, p)).unwrap_or(Value::Null) })
            .collect();
        let moved = reorder::reorder(&given, &mv, parent.is_some())?;
        let mut order = Vec::with_capacity(moved.order.len());
        for (i, place) in moved.order.iter().enumerate() {
            let mut record = self.records[*place].clone();
            if let Some(fields) = record.as_object_mut() {
                fields.insert(by.clone(), Value::from(moved.numbers[i]));
                if let Some(p) = parent {
                    fields.insert(p.to_string(), moved.parents[i].clone());
                }
            }
            order.push(record);
        }
        let now = moved.order.iter().position(|p| *p == mv.row())?;
        base.host.write(base.name().unwrap_or(""), &bound, Value::Array(order));
        self.read_now(base);
        self.set_chosen(base, Some(now));
        base.fire("reorder");
        Some(now)
    }

    /// The choices a field of the records offers: the host's for the column's path.
    pub fn choices_of(base: &PartBase, field: &str) -> Vec<Choice> {
        match base.bound() {
            Some(bound) => base.host.choices(&column_path(bound, field)),
            None => Vec::new(),
        }
    }
}

/// Whether a data part draws its own border: with `frame`, or when no container frames it (DESIGN 7.9).
pub fn framed(base: &PartBase) -> bool {
    base.spec.flag("frame") || !base.spec.flag("inside")
}

/// The line where a dragged row lands, or the row it goes under in a tree.
pub fn draw_drop(p: &Painter, row: Rect, at: DropAt) {
    let accent = p.look.color("accent");
    match at {
        DropAt::Before => p.fill(Rect { height: 2.0, ..row }, accent, 0.0),
        DropAt::After => p.fill(Rect { y: row.bottom() - 2.0, height: 2.0, ..row }, accent, 0.0),
        DropAt::Into => p.fill(row, accent.with_a(64), 0.0),
    }
}

/// A row's background in the list look: filled while it is chosen or under
/// the pointer, a line along its bottom; gives the look its text is drawn in.
pub fn draw_row_back(p: &Painter, row: Rect, state: Option<&str>) -> LookValues {
    let look = p.look;
    let shown = look.part("list", state);
    if let Some(state) = state {
        p.fill(row, shown.background.unwrap_or_else(|| look.color(if state == "selected" { "selection" } else { "accent_soft" })), 0.0);
    }
    p.fill(Rect { y: row.bottom() - look.border(), height: look.border(), ..row }, look.color("border"), 0.0);
    shown
}

/// Whether no keys are held.
pub fn plain(key: KeyInput) -> bool {
    key.modifiers == Modifiers::NONE
}

/// The sizes of the look a part was laid out in, which it places its rows
/// by between events too (where a test or the pointer asks for a row).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Measures {
    pub padding: f64,
    pub border: f64,
    pub text: f64,
    pub small: f64,
}

impl Measures {
    pub fn of(look: &crate::theme::ThemeLook) -> Measures {
        Measures { padding: look.padding(), border: look.border(), text: look.size("text"), small: look.size("small") }
    }
}

impl Default for Measures {
    fn default() -> Self {
        Measures::of(&crate::theme::ThemeLook::clarity(false))
    }
}
