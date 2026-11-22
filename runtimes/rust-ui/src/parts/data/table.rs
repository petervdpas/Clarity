//! Clarity's table part (U1c), and a tree and a tree table, which are a
//! table in tree mode, as C# TablePart.cs. It owns how a table behaves, the
//! same on every stack (conformance/tables.json): which input a cell holds,
//! when a row counts as chosen and when as edited, and how text that is no
//! value is marked. Rows show in the table's order (`order`, a click on a
//! `sortable` header) but keep their place in the data, which every event
//! names (conformance/table-order.json); a page at a time with `page`
//! (conformance/table-pages.json); a tree's rows under their parents
//! (conformance/trees.json). The order, the pages and the tree are the
//! runtime's (`clarity_runtime::tables`, `trees`); only the rows in sight are
//! drawn, and they scroll under the header. A cell is typed into with
//! Clarity.UI's text editor, which the table holds over the cell; Tab types
//! on in the next cell. A choice cell opens its choices over the window.
//! How the keys reach every kind of cell, a date's calendar, a color's
//! palette and what the table says where the pointer rests: `reach`.

mod adding;
mod columns;
mod drawing;
mod editing;
mod order;
mod reach;
mod tree;

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::time::{Duration, Instant};

use clarity_runtime::reorder::{DropAt, Move};
use clarity_runtime::tables::{self, Paged, TableOrder};
use clarity_runtime::trees::{self, TreeShown};
use clarity_runtime::{NaiveDate, NaiveDateTime};
use serde_json::{Map, Value};
use skia_safe::{Color, Paint, PaintCap, PaintStyle};

use super::rows::{self, Measures, Rows, Scroller};
use crate::element::{Cx, Element, ElementId, Node};
use crate::element_node;
use crate::geometry::Rect;
use crate::input::{Key, KeyInput, Modifiers, PointerInput};
use crate::locale;
use crate::markup::PartSpec;
use crate::overlay::Overlay;
use crate::painter::{Painter, TextAlign};
use crate::part_element;
use crate::parts::inputs::calendar::{Calendar, Day, DayCalendar};
use crate::parts::inputs::formats::{format_of, formats_of, FieldFormat, FormatKind, Formats};
use crate::parts::text_editor::TextEditor;
use crate::parts::{Choice, Part, PartBase, PartHost};

/// What a cell holds: the kind of value of its column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellKind {
    Text,
    Number,
    Bool,
    Choice,
    Date,
    Moment,
    /// A color, "#rrggbb": a swatch of it beside its text, typed as text.
    Color,
    /// A folder's path, typed, beside a button that opens the folder picker.
    Folder,
}

/// The folder the picker chose for a cell, off the screen's thread: its row, its field and the folder.
struct FolderPicked(usize, String, Option<String>);

/// A column of a table: the field it shows, how, and whether its cells are edited.
#[derive(Debug, Clone, PartialEq)]
pub struct TableColumn {
    pub field: String,
    pub label: String,
    pub kind: CellKind,
    pub edit: bool,
    pub switch: bool,
    pub width: Option<f64>,
    pub choices: Vec<Choice>,
    /// A number column of whole numbers.
    pub integer: bool,
    /// How its numbers, days and moments show and are read, in the person's formats.
    pub format: Option<FieldFormat>,
    /// The handler a click on one of its cells runs (`link chapter: toChapter`): its cells are links.
    pub link: Option<String>,
}

/// The buttons of a table's page bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageButton {
    First,
    Previous,
    Next,
    Last,
}

impl PageButton {
    pub const ALL: [PageButton; 4] = [PageButton::First, PageButton::Previous, PageButton::Next, PageButton::Last];
}

/// What a press went down on.
#[derive(Debug, Clone, PartialEq)]
enum Press {
    Row(usize),
    Header(String),
    /// A header edge: its column, where the press went down, the column's width then, and the widths as kept then.
    Edge(String, f64, f64, String),
    Pager(PageButton),
    /// Expand all (true) or Collapse all (false) above a tree's rows.
    Fold(bool),
    Bar,
    Editor,
    /// Its Add button (`add`), and a row's trash button (`remove`).
    Add,
    Remove(usize),
}

/// How wide the column of the rows' trash buttons is, and how big the icons of its own buttons.
const REMOVE_WIDTH: f64 = 36.0;
const OWN_ICON: f64 = 16.0;

/// A cell being typed into: its row's place, its field, and the text editor over it.
struct Editing {
    place: usize,
    field: String,
    editor: TextEditor,
}

/// The choices of a choice cell while they show: the overlay's id, the cell,
/// what was picked, and the choice the keys mark.
struct OpenChoices {
    overlay: ElementId,
    place: usize,
    field: String,
    picked: Rc<Cell<Option<usize>>>,
    marks: Rc<RefCell<ChoiceMarks>>,
}

/// The palette of a color cell while it shows: the overlay's id, the cell, and the color picked.
struct OpenPalette {
    overlay: ElementId,
    place: usize,
    field: String,
    picked: Rc<RefCell<Option<String>>>,
}

/// How far from a header's right edge a press takes the edge.
const EDGE: f64 = 5.0;
/// The side of a color cell's swatch.
const SWATCH: f64 = 12.0;
/// How far in a tree's rows are under their parent.
const INDENT: f64 = 16.0;

pub struct TablePart {
    base: PartBase,
    rows: Rows,
    columns: Vec<TableColumn>,
    order: Option<TableOrder>,
    order_read: bool,
    page: usize,
    opened: Vec<Value>,
    /// How wide the words of Expand all and Collapse all are, as last laid out.
    fold_text: (f64, f64),
    person_widths: HashMap<String, f64>,
    marked: HashMap<(usize, String), String>,
    marked_for: Vec<Value>,
    editing: Option<Editing>,
    press: Option<Press>,
    choices: Option<OpenChoices>,
    palette: Option<OpenPalette>,
    /// The check or choice the keys work on, reached with Tab or a press; none for none.
    focused_cell: Option<(usize, String)>,
    /// Whether the widths the person left before were read.
    widths_read: bool,
    /// The calendar of a date or moment cell while it is open: its overlay, the cell, and the calendar.
    calendar: Option<(ElementId, usize, String, Calendar)>,
    look: Measures,
    /// How wide each header's label is, and the page bar's text, as last measured.
    widths: HashMap<String, f64>,
    pager_text: f64,
    /// How wide its Add button's caption is, as last measured.
    add_text: f64,
    /// Set when the person said yes to removing the chosen row (`remove confirm`): its `on remove` runs on the next wake.
    remove_said: Rc<Cell<bool>>,
    formats: Formats,
}

pub fn make(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    let mut part = TablePart {
        base: PartBase::new(spec, host),
        rows: Rows::default(),
        columns: Vec::new(),
        order: None,
        order_read: false,
        page: 1,
        opened: Vec::new(),
        fold_text: (0.0, 0.0),
        person_widths: HashMap::new(),
        marked: HashMap::new(),
        marked_for: Vec::new(),
        editing: None,
        press: None,
        choices: None,
        palette: None,
        focused_cell: None,
        widths_read: false,
        calendar: None,
        look: Measures::default(),
        widths: HashMap::new(),
        pager_text: 60.0,
        add_text: 32.0,
        remove_said: Rc::new(Cell::new(false)),
        formats: Formats::default(),
    };
    part.sync();
    Box::new(part)
}

/// A value as a moment: `2026-10-20T14:05:00`.
fn moment_of(value: &Value) -> Option<NaiveDateTime> {
    let text = value.as_str()?.trim();
    NaiveDateTime::parse_from_str(text, "%Y-%m-%dT%H:%M:%S")
        .or_else(|_| NaiveDateTime::parse_from_str(text, "%Y-%m-%dT%H:%M:%S%.f"))
        .or_else(|_| NaiveDateTime::parse_from_str(text, "%Y-%m-%dT%H:%M"))
        .or_else(|_| NaiveDateTime::parse_from_str(text, "%Y-%m-%d %H:%M:%S"))
        .ok()
}

/// A value as a day: `2026-10-20`.
fn day_of(value: &Value) -> Option<NaiveDate> {
    let text = value.as_str()?.trim();
    NaiveDate::parse_from_str(text, "%Y-%m-%d").ok().or_else(|| moment_of(value).map(|m| m.date()))
}

/// A moment as the data holds it.
fn moment_text(moment: NaiveDateTime) -> String {
    moment.format("%Y-%m-%dT%H:%M:%S").to_string()
}

/// A time typed as `h:mm` or `hh:mm`, on a 24-hour clock: hours and minutes.
fn time_typed(typed: &str) -> Option<(u32, u32)> {
    let (h, m) = typed.trim().split_once(':')?;
    if h.is_empty() || h.len() > 2 || m.len() != 2 || !h.chars().chain(m.chars()).all(|c| c.is_ascii_digit()) {
        return None;
    }
    let (h, m) = (h.parse::<u32>().ok()?, m.parse::<u32>().ok()?);
    (h < 24 && m < 60).then_some((h, m))
}

/// The color a cell holds, read as the color input reads it ("#rrggbb", three digits, any case), or none.
fn color_of(value: &Value) -> Option<Color> {
    crate::parts::inputs::color::color_of(value.as_str()?)
}

/// The kind of cell a type's name asks for, and whether a number is whole.
fn kind_of_type(name: &str) -> Option<(CellKind, bool)> {
    Some(match name.to_ascii_lowercase().as_str() {
        "bool" | "boolean" => (CellKind::Bool, false),
        "int" | "integer" | "i64" | "i32" | "long" | "short" => (CellKind::Number, true),
        "decimal" | "money" | "percent" | "float" | "f64" | "f32" | "double" | "number" => (CellKind::Number, false),
        "date" | "day" | "naivedate" | "dateonly" => (CellKind::Date, false),
        "moment" | "datetime" | "naivedatetime" | "timestamp" => (CellKind::Moment, false),
        "text" | "string" => (CellKind::Text, false),
        _ => return None,
    })
}

/// The kind of cell a value of the data asks for, when nothing names its type.
fn kind_of_value(value: &Value) -> Option<(CellKind, bool)> {
    match value {
        Value::Bool(_) => Some((CellKind::Bool, false)),
        Value::Number(n) => Some((CellKind::Number, n.is_i64() || n.is_u64())),
        Value::String(s) => {
            let t = s.trim();
            if t.len() >= 16 && moment_of(value).is_some() {
                Some((CellKind::Moment, false))
            } else if t.len() == 10 && NaiveDate::parse_from_str(t, "%Y-%m-%d").is_ok() {
                Some((CellKind::Date, false))
            } else {
                None
            }
        }
        _ => None,
    }
}

impl TablePart {

    /// Whether it draws its own border.
    pub fn framed(&self) -> bool {
        rows::framed(&self.base)
    }

    /// The order the rows show in now; none for the data's order.
    pub fn order(&self) -> Option<&TableOrder> {
        self.order.as_ref()
    }

    /// The records it shows, in the order of the data.
    pub fn records(&self) -> &[Value] {
        &self.rows.records
    }

    /// The place of the chosen record in the data; none for none.
    pub fn chosen(&self) -> Option<usize> {
        self.rows.chosen(&self.base)
    }

    /// Chooses a row as code does: no `select`, and it shows (its tree opens, its page turns, it scrolls into sight).
    pub fn set_chosen(&mut self, place: Option<usize>) {
        self.sync();
        self.rows.set_chosen(&self.base, place);
        if let Some(place) = self.rows.chosen(&self.base) {
            self.show_place(place);
        }
    }

    /// Where a dragged row lands now.
    pub fn drop(&self) -> Option<(usize, DropAt)> {
        self.rows.drop
    }

    /// Where its rows scroll.
    pub fn scroller(&self) -> &Scroller {
        &self.rows.scroller
    }

    // -- Columns ----------------------------------------------------------------

    /// Reads the records again when the window's values changed; what was typed into other records is gone.
    fn sync(&mut self) {
        self.read_remembered_widths();
        if self.rows.read(&self.base) {
            self.read_columns();
        }
        if let Some(place) = self.rows.chosen_by_code(&self.base) {
            self.show_place(place);
        }
    }

    // -- What shows ---------------------------------------------------------------

    // -- Layout -------------------------------------------------------------------

    /// A row fits its text, and is never higher than a row of the window's grid (the project's `row height`).
    fn row_height(&self) -> f64 {
        (self.look.text + 18.0).min(crate::grid::row_height())
    }

    fn inner(&self) -> Rect {
        let b = self.framed_box();
        if self.framed() { b.inset(self.look.border) } else { b }
    }

    /// What its frame, header and rows fill: all of it below the Add strip.
    fn framed_box(&self) -> Rect {
        let b = self.base.node.bounds;
        let strip = self.add_strip().height;
        Rect::new(b.x, b.y + strip, b.width, (b.height - strip).max(0.0))
    }

    fn body(&self) -> Rect {
        let header = self.header();
        let pager = self.pager();
        Rect::new(header.x, header.bottom(), header.width, (pager.y - header.bottom()).max(0.0))
    }

    /// Places the rows' scroll place: as high as the rows shown, as wide as the columns when they are wider.
    fn place_rows(&mut self) {
        let content = self.shown_places().len() as f64 * self.row_height();
        let body = self.body();
        let total: f64 = self.lefts(0.0).iter().map(|l| l.1).sum::<f64>() + if self.removes() { REMOVE_WIDTH } else { 0.0 };
        let width = if total > body.width + 0.5 { total } else { body.width };
        self.rows.scroller.place_wide(body, content, width);
    }

    // -- What a cell shows and takes ----------------------------------------------

    // -- Drawing --------------------------------------------------------------------

}

/// How wide the button of a date cell is that opens its calendar.
const CALENDAR_BUTTON: f64 = 24.0;

/// The button at the right of a date cell's input, which opens its calendar.
fn calendar_button(cell: Rect) -> Rect {
    Rect::new(cell.right() - 3.0 - CALENDAR_BUTTON, cell.y + 3.0, CALENDAR_BUTTON, (cell.height - 6.0).max(0.0))
}

/// Where a cell's editor lies in its cell: a moment's time at its right, a date beside its calendar button, else the input.
fn editor_box(column: &TableColumn, cell: Rect) -> Rect {
    match column.kind {
        CellKind::Moment => time_box(cell).inset(3.0),
        CellKind::Date => {
            let input = cell.inset(3.0);
            Rect { width: (cell.width - 6.0 - CALENDAR_BUTTON).max(0.0), ..input }
        }
        CellKind::Folder if crate::files::system().picks_folders() => {
            let input = cell.inset(3.0);
            Rect { width: (cell.width - 6.0 - CALENDAR_BUTTON).max(0.0), ..input }
        }
        _ => cell.inset(3.0),
    }
}

/// Where a moment cell's time is typed: at its right.
fn time_box(cell: Rect) -> Rect {
    let width = 72f64.min(cell.width / 2.0);
    Rect::new(cell.right() - width, cell.y, width, cell.height)
}

fn line(p: &Painter, x0: f64, y0: f64, x1: f64, y1: f64, color: Color) {
    let mut paint = Paint::default();
    paint.set_color(color);
    paint.set_anti_alias(true);
    paint.set_stroke_width(2.0);
    paint.set_style(PaintStyle::Stroke);
    paint.set_stroke_cap(PaintCap::Round);
    p.canvas.draw_line((x0 as f32, y0 as f32), (x1 as f32, y1 as f32), &paint);
}

/// A small chevron pointing up, down, left or right, around a point.
fn chevron(p: &Painter, x: f64, y: f64, way: &str, color: Color) {
    const S: f64 = 3.5;
    match way {
        "down" => {
            line(p, x - S, y - S / 2.0, x, y + S / 2.0, color);
            line(p, x, y + S / 2.0, x + S, y - S / 2.0, color);
        }
        "up" => {
            line(p, x - S, y + S / 2.0, x, y - S / 2.0, color);
            line(p, x, y - S / 2.0, x + S, y + S / 2.0, color);
        }
        "left" => {
            line(p, x + S / 2.0, y - S, x - S / 2.0, y, color);
            line(p, x - S / 2.0, y, x + S / 2.0, y + S, color);
        }
        _ => {
            line(p, x - S / 2.0, y - S, x + S / 2.0, y, color);
            line(p, x + S / 2.0, y, x - S / 2.0, y + S, color);
        }
    }
}

fn draw_check(p: &Painter, cell: Rect, on: bool, inert: bool) {
    let look = p.look;
    let side = 16.0;
    let b = Rect::new(cell.x + (cell.width - side) / 2.0, cell.y + (cell.height - side) / 2.0, side, side);
    let radius = look.radius("input").min(4.0);
    p.fill(b, if on { look.color("accent") } else { look.color("input") }, radius);
    p.stroke(b, if on { look.color("accent") } else { look.color("strong_border") }, look.border(), radius);
    if on {
        let color = look.color("on_accent");
        line(p, b.x + 4.0, b.y + 8.5, b.x + 7.0, b.y + 11.5, color);
        line(p, b.x + 7.0, b.y + 11.5, b.x + 12.5, b.y + 5.0, color);
    }
    if inert {
        p.fill(b, look.color("panel").with_a(110), radius);
    }
}

fn draw_switch(p: &Painter, cell: Rect, on: bool, inert: bool) {
    let look = p.look;
    let track = Rect::new(cell.x + (cell.width - 32.0) / 2.0, cell.y + (cell.height - 18.0) / 2.0, 32.0, 18.0);
    p.fill(track, if on { look.color("accent") } else { look.color("strong_border") }, 9.0);
    let knob = Rect::new(if on { track.right() - 16.0 } else { track.x + 2.0 }, track.y + 2.0, 14.0, 14.0);
    p.fill(knob, if on { look.color("on_accent") } else { look.color("input") }, 7.0);
    if inert {
        p.fill(track, look.color("panel").with_a(110), 9.0);
    }
}

impl Element for TablePart {
    part_element!();

    /// A folder the picker chose for a cell is that cell's edit.
    fn on_later(&mut self, done: crate::later::Done, cx: &mut Cx) {
        if let Ok(picked) = done.downcast::<FolderPicked>() {
            let FolderPicked(place, field, folder) = *picked;
            if let Some(folder) = folder.filter(|f| !f.is_empty()) {
                self.edit(place, &field, Value::String(folder));
            }
        }
        cx.invalidate();
    }

    fn on_wake(&mut self, _cx: &mut Cx) {
        if self.remove_said.replace(false) {
            self.base.fire("remove");
        }
    }

    fn focusable(&self) -> bool {
        self.base.enabled()
    }

    /// While a cell has the keys, Tab goes on to the next cell, and from the last out of the table.
    fn wants_tab(&self) -> bool {
        self.editing.is_some() || self.focused_cell.is_some() || self.choices.is_some()
    }

    /// A row of another part let go on it: the row under the point is chosen and the handler for that part runs.
    fn take_drop(&mut self, from: &str, _x: f64, y: f64, cx: &mut Cx) -> bool {
        let Some(handler) = rows::drop_handler(&self.base, from) else { return false };
        if let Some((place, _)) = self.row_at(y) {
            self.choose(place, cx);
        }
        self.base.host.run(self.base.name(), &crate::markup::HandlerCall::new("drop", &handler, Vec::new()));
        true
    }

    /// The page bar's buttons and a tree row's toggle say what a press does, in Clarity's words.
    fn tooltip_at(&self, x: f64, y: f64) -> Option<String> {
        self.tip_at(x, y)
    }

    /// A header edge resizes; a page button that works and a toggle are pressed.
    fn cursor_at(&self, x: f64, y: f64) -> Option<crate::input::Cursor> {
        self.pointer_shape(x, y)
    }

    fn typing(&self) -> bool {
        self.editing.is_some()
    }

    fn take_typed(&mut self, cx: &mut Cx) -> bool {
        if self.editing.is_none() {
            return false;
        }
        self.take(cx);
        true
    }

    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        // A yes to removing a row runs `on remove` once the screen is laid out, as an event.
        if self.remove_said.get() {
            cx.wake(self.base.node.id(), cx.now());
        }
        self.look = Measures::of(cx.look());
        self.base.node.bounds = area;
        self.sync();
        let header = cx.look().part("header", None);
        let size = header.size.map(|s| s as f64).unwrap_or_else(|| cx.look().size("text"));
        let font = header.font.clone().unwrap_or_else(|| "text".to_string());
        let widths: HashMap<String, f64> = self.columns.iter().map(|c| (c.field.clone(), cx.fonts().measure(&c.label, &font, size, header.bold()))).collect();
        self.widths = widths;
        if let Some(paged) = self.paged() {
            self.pager_text = cx.fonts().measure(&paged.text(), "text", cx.look().size("text"), false);
        }
        if self.folds() {
            let size = cx.look().size("text");
            self.fold_text = (cx.fonts().measure(&locale::translate("expandAll", "Expand all"), "text", size, false), cx.fonts().measure(&locale::translate("collapseAll", "Collapse all"), "text", size, false));
        }
        if self.adds() {
            self.add_text = cx.fonts().measure(&add_word(), "text", cx.look().size("text"), false);
        }
        self.place_rows();
        self.place_editor(cx);
    }

    fn draw(&self, p: &Painter) {
        let look = p.look;
        if let Some(button) = self.add_button() {
            self.draw_add(p, button);
        }
        let b = self.framed_box();
        let radius = look.radius("panel");
        let plain = look.part("list", None);
        let framed = self.framed();
        p.fill(b, plain.background.unwrap_or_else(|| look.color("input")), if framed { radius } else { 0.0 });
        {
            let _clip = p.clip(self.inner());
            if self.has_header() {
                self.draw_header(p, self.header());
            }
            {
                let _rows = p.clip(self.rows.scroller.bounds);
                self.draw_rows(p);
                self.rows.scroller.draw(p, self.base.node.hovered);
            }
            if let Some(paged) = self.paged() {
                self.draw_pager(p, paged);
            }
            if self.folds() {
                self.draw_fold_bar(p);
            }
        }
        if framed {
            p.stroke(b, plain.border.unwrap_or_else(|| look.color("strong_border")), look.border(), radius);
        }
        if self.base.node.focused && self.editing.is_none() {
            p.stroke(b, look.color("focus"), 2.0, if framed { radius } else { 0.0 });
        }
    }

    fn on_pointer_pressed(&mut self, at: PointerInput, cx: &mut Cx) {
        self.sync();
        self.press = None;
        self.focused_cell = None;
        if !self.base.enabled() {
            return;
        }
        let (x, y) = (at.screen_x, at.screen_y);
        if self.add_button().is_some_and(|b| b.contains(x, y)) {
            self.take(cx);
            if !self.locked() {
                self.press = Some(Press::Add);
            }
            return;
        }
        if let Some((place, _)) = self.row_at(y).filter(|_| self.rows.scroller.bounds.contains(x, y)) {
            if self.remove_box(place).is_some_and(|b| b.contains(x, y)) {
                self.take(cx);
                if !self.locked() {
                    self.press = Some(Press::Remove(place));
                }
                return;
            }
        }
        if self.pager().contains(x, y) {
            self.press = PageButton::ALL.into_iter().find(|b| self.pager_box(*b).is_some_and(|r| r.contains(x, y))).map(Press::Pager);
            return;
        }
        if self.fold_bar().contains(x, y) {
            self.press = [true, false].into_iter().find(|e| self.fold_box(*e).is_some_and(|r| r.contains(x, y))).map(Press::Fold);
            return;
        }
        if self.has_header() && self.header().contains(x, y) {
            self.take(cx);
            // The line between two headers is grabbed from either side of it, as a grid's grip straddles it.
            if let Some((field, b)) = self.edge_column_at(x, y) {
                if at.clicks == 2 {
                    self.designed_widths();
                    self.place_editor(cx);
                    cx.invalidate();
                    return;
                }
                self.press = Some(Press::Edge(field, x, b.width, self.widths_written()));
                return;
            }
            for column in self.columns.clone() {
                if self.header_box(&column.field).is_some_and(|b| b.contains(x, y)) {
                    self.press = Some(Press::Header(column.field.clone()));
                }
            }
            return;
        }
        if self.rows.scroller.press(x, y) {
            self.press = Some(Press::Bar);
            self.place_editor(cx);
            cx.invalidate();
            return;
        }
        // A press in the cell typed into goes to its editor.
        if let Some(e) = self.editing.as_mut() {
            if e.editor.node().bounds.contains(x, y) {
                e.editor.pressed(at, cx);
                self.press = Some(Press::Editor);
                return;
            }
        }
        let Some((place, c, cell)) = self.cell_at(x, y) else {
            self.take(cx);
            return;
        };
        let column = self.columns[c].clone();
        if self.editing.as_ref().is_some_and(|e| e.place != place || e.field != column.field) {
            self.take(cx);
        }
        if self.is_tree() && self.toggle_box(place).is_some_and(|t| t.contains(x, y)) {
            self.toggle(place);
            cx.invalidate();
            return;
        }
        // A press on an edited cell's input chooses no row: a taken edit chooses it.
        // A press beside the input, in the cell's margin, chooses the row.
        if column.edit && !self.locked() && Self::input_of(&column, cell).contains(x, y) {
            match column.kind {
                CellKind::Bool => {
                    // The check is the cell the keys work on now: Space ticks it again.
                    self.focused_cell = Some((place, column.field.clone()));
                    let on = rows::value_of(&self.rows.records[place], &column.field) == Value::Bool(true);
                    self.edit(place, &column.field, Value::Bool(!on));
                    cx.invalidate();
                    return;
                }
                CellKind::Choice => {
                    self.focused_cell = Some((place, column.field.clone()));
                    self.open_choices_of(place, c, cell, cx);
                    return;
                }
                CellKind::Color if self.palette_button(cell).contains(x, y) => {
                    self.open_palette(place, &column.field, cx);
                    return;
                }
                CellKind::Date if calendar_button(cell).contains(x, y) => {
                    self.open_calendar_of(place, c, cell, cx);
                    return;
                }
                CellKind::Folder if crate::files::system().picks_folders() && calendar_button(cell).contains(x, y) => {
                    self.focused_cell = Some((place, column.field.clone()));
                    self.choose_folder(place, &column.field, cx);
                    return;
                }
                CellKind::Text | CellKind::Number | CellKind::Color | CellKind::Date | CellKind::Folder => {
                    if self.editing.is_none() {
                        self.begin_typing(place, c, cx);
                    }
                    return;
                }
                CellKind::Moment if x >= time_box(cell).x => {
                    if self.editing.is_none() {
                        self.begin_typing(place, c, cx);
                    }
                    return;
                }
                CellKind::Moment => {
                    self.open_calendar_of(place, c, cell, cx);
                    return;
                }
            }
        }
        // A click on a link chooses the row and runs the link's handler.
        if let Some((_, handler)) = self.link_at(x, y).filter(|_| !self.locked() && at.clicks == 1) {
            self.choose(place, cx);
            self.base.host.run(self.base.name(), &crate::parts::HandlerCall::new("click", &handler, Vec::new()));
            cx.invalidate();
            return;
        }
        self.press = Some(Press::Row(place));
        self.rows.press_row(&self.base, place, at);
        if at.clicks == 2 {
            self.choose(place, cx);
            self.base.fire("open");
            self.press = None;
        }
    }

    fn on_pointer_moved(&mut self, at: PointerInput, cx: &mut Cx) {
        self.rows.pointer = Some((at.screen_x, at.screen_y));
        match self.press.clone() {
            Some(Press::Edge(field, x, width, _)) if self.base.node.pressed => {
                self.person_widths.insert(field.clone(), (width + at.screen_x - x).clamp(32.0, 2000.0));
                // The other columns keep the widths they show.
                for other in self.columns.clone() {
                    if !self.person_widths.contains_key(&other.field) {
                        if let Some(b) = self.header_box(&other.field) {
                            self.person_widths.insert(other.field.clone(), b.width);
                        }
                    }
                }
                self.place_editor(cx);
            }
            Some(Press::Row(_)) if self.base.node.pressed => {
                let over = self.row_at(at.screen_y);
                let tree = self.is_tree();
                self.rows.drag_to(at, over, tree);
            }
            Some(Press::Bar) => {
                self.rows.scroller.drag(at.screen_x, at.screen_y);
                self.place_editor(cx);
            }
            Some(Press::Editor) => {
                if let Some(e) = self.editing.as_mut() {
                    e.editor.dragged(at, cx);
                }
            }
            _ => {}
        }
        cx.invalidate();
    }

    fn on_pointer_released(&mut self, at: PointerInput, cx: &mut Cx) {
        let was = self.press.take();
        cx.invalidate();
        // A row let go outside the table goes to the part there: the row is chosen, the drop asked for.
        if let (Some(Press::Row(place)), Some(name)) = (&was, self.base.name().map(str::to_string)) {
            if !self.base.node.bounds.contains(at.screen_x, at.screen_y) {
                let place = *place;
                self.rows.cancel();
                self.choose(place, cx);
                cx.drop_at(&name, at.screen_x, at.screen_y);
                return;
            }
        }
        match was {
            Some(Press::Row(place)) => {
                let (dragged, mv) = self.rows.drop_row();
                if dragged {
                    if let Some(mv) = mv {
                        self.apply(mv, cx);
                    }
                } else if self.row_at(at.screen_y).is_some_and(|(over, _)| over == place) {
                    self.choose(place, cx);
                }
            }
            Some(Press::Edge(_, _, _, before)) => self.keep_widths(&before),
            Some(Press::Header(field)) => {
                if self.header_box(&field).is_some_and(|b| b.contains(at.screen_x, at.screen_y)) {
                    self.sort_by(&field);
                    self.place_editor(cx);
                }
            }
            Some(Press::Fold(expand)) => {
                if self.fold_box(expand).is_some_and(|b| b.contains(at.screen_x, at.screen_y)) {
                    self.fold_all(expand);
                    cx.invalidate();
                }
            }
            Some(Press::Pager(button)) => {
                if self.pager_box(button).is_some_and(|b| b.contains(at.screen_x, at.screen_y)) && self.page_button_works(button) {
                    let Some(paged) = self.paged() else { return };
                    self.turn_to(match button {
                        PageButton::First => 1,
                        PageButton::Previous => paged.page - 1,
                        PageButton::Next => paged.page + 1,
                        PageButton::Last => paged.pages,
                    });
                }
            }
            Some(Press::Bar) => {
                self.rows.scroller.release();
            }
            // Its Add button runs `on add`; a row's trash button picks the row and runs `on remove`.
            Some(Press::Add) => {
                if self.add_button().is_some_and(|b| b.contains(at.screen_x, at.screen_y)) {
                    self.base.fire("add");
                }
            }
            Some(Press::Remove(place)) => {
                if self.remove_box(place).is_some_and(|b| b.contains(at.screen_x, at.screen_y)) {
                    self.choose(place, cx);
                    self.remove_chosen(cx);
                }
            }
            Some(Press::Editor) => {
                if let Some(e) = self.editing.as_mut() {
                    e.editor.released();
                }
            }
            _ => {}
        }
    }

    /// The pointer was taken away mid press: a row drag or a header edge drag ends where it is, and the later release does nothing.
    fn on_pointer_cancelled(&mut self, cx: &mut Cx) {
        self.press = None;
        self.rows.cancel();
        cx.invalidate();
    }

    /// The wheel anywhere over the part (its header too) scrolls the rows,
    /// sideways too when it turns that way or Shift is held.
    fn on_wheel(&mut self, dx: f64, dy: f64, held: Modifiers, cx: &mut Cx) -> bool {
        let moved = self.rows.scroller.wheel_both(dx, dy, held);
        if moved {
            self.place_editor(cx);
            cx.invalidate();
        }
        moved
    }

    fn on_focus_changed(&mut self, focused: bool, cx: &mut Cx) {
        if !focused {
            self.take(cx);
            self.focused_cell = None;
        }
        cx.invalidate();
    }

    fn on_key(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        self.sync();
        if let Some((place, field)) = self.editing.as_ref().map(|e| (e.place, e.field.clone())) {
            let plain = key.modifiers == Modifiers::NONE;
            let kind = self.columns.iter().find(|c| c.field == field).map(|c| c.kind);
            match key.key {
                Key::Tab if plain || key.modifiers == Modifiers::SHIFT => {
                    // Past the last cell, the focus leaves the table as Tab moves it.
                    return self.type_on(place, &field, key.shift(), cx);
                }
                // Alt+Down opens a date's calendar, a color's palette.
                Key::Down if key.modifiers == Modifiers::ALT && matches!(kind, Some(CellKind::Date | CellKind::Color)) => {
                    self.take(cx);
                    if kind == Some(CellKind::Date) {
                        if let (Some(c), Some(cell)) = (self.column_index(&field), self.cell_box(place, &field)) {
                            self.open_calendar_of(place, c, cell, cx);
                        }
                    } else {
                        self.open_palette(place, &field, cx);
                    }
                    cx.invalidate();
                    return true;
                }
                Key::Escape => {
                    self.stop(cx);
                    return true;
                }
                Key::Enter if plain => {
                    self.take(cx);
                    return true;
                }
                _ => {
                    let took = self.editing.as_mut().is_some_and(|e| e.editor.key(key, cx));
                    cx.invalidate();
                    return took;
                }
            }
        }
        if self.cell_keyed(key, cx) {
            cx.invalidate();
            return true;
        }
        let (was_move, mv) = self.rows.key_move(&self.base, key);
        if was_move {
            if let Some(mv) = mv {
                self.apply(mv, cx);
            }
            return true;
        }
        if !self.base.enabled() || !rows::plain(key) {
            return false;
        }
        // Insert runs the Add button, an empty table's too.
        if key.key == Key::Insert && self.adds() && !self.locked() {
            self.base.fire("add");
            return true;
        }
        let shown = self.shown_places();
        if shown.is_empty() {
            return false;
        }
        let at = self.rows.chosen(&self.base).and_then(|c| shown.iter().position(|p| *p == c));
        let page = ((self.body().height / self.row_height()) as usize).max(1);
        if self.is_tree() && matches!(key.key, Key::Right | Key::Left) {
            if let Some(at) = at {
                let place = shown[at];
                let open = self.is_open(place);
                if self.tree_place(place).is_some_and(|t| t.2) && open == (key.key == Key::Left) {
                    self.toggle(place);
                    cx.invalidate();
                }
                return true;
            }
        }
        if key.key == Key::Enter && at.is_some() {
            self.base.fire("open");
            return true;
        }
        // Delete runs the chosen row's trash button; Insert the Add button.
        if key.key == Key::Delete && at.is_some() && self.removes() && !self.locked() {
            self.remove_chosen(cx);
            return true;
        }
        let Some(to) = rows::moved_to(key.key, at, shown.len(), page) else { return false };
        self.choose(shown[to], cx);
        true
    }

    fn on_text(&mut self, text: &str, cx: &mut Cx) {
        // Letters typed while a choice cell's list is open go to the choice that starts with them.
        if let Some(open) = &self.choices {
            open.marks.borrow_mut().seek(text, cx.now());
            cx.invalidate();
            return;
        }
        if let Some(e) = self.editing.as_mut() {
            e.editor.typed(text, cx);
            cx.invalidate();
        }
    }

    fn on_overlay_closed(&mut self, overlay: ElementId, cx: &mut Cx) {
        if let Some((_, place, field, calendar)) = self.calendar.take_if(|c| c.0 == overlay) {
            if let Some(day) = calendar.take_picked().and_then(|d| NaiveDate::from_ymd_opt(d.year, d.month, d.day)) {
                self.marked.remove(&(place, field.clone()));
                self.pick_day(place, &field, day);
            }
            cx.invalidate();
            return;
        }
        if let Some(open) = self.palette.take_if(|p| p.overlay == overlay) {
            let color = open.picked.borrow_mut().take();
            if let Some(color) = color {
                self.marked.remove(&(open.place, open.field.clone()));
                self.edit(open.place, &open.field, Value::from(color));
            }
            cx.invalidate();
            return;
        }
        let Some(open) = self.choices.take_if(|c| c.overlay == overlay) else { return };
        let Some(at) = open.picked.get() else { return };
        let Some(column) = self.columns.iter().find(|c| c.field == open.field) else { return };
        let Some(choice) = column.choices.get(at) else { return };
        let value = choice.value.clone();
        self.edit(open.place, &open.field, value);
        cx.invalidate();
    }
}

impl Part for TablePart {
    fn base(&self) -> &PartBase {
        &self.base
    }

    fn grows(&self) -> bool {
        true
    }
}

/// The choice of a choice cell's list the keys mark, shared by the list
/// and its table (which keeps the focus and works the list with its keys).
#[derive(Debug, Default)]
pub struct ChoiceMarks {
    labels: Vec<String>,
    /// The choice marked for Enter: its value's at first, then where the keys or the pointer go.
    pub highlighted: Option<usize>,
    /// Whether the marked choice still has to be scrolled into view.
    reveal: bool,
    sought: String,
    sought_at: Option<Instant>,
}

/// How long typed letters add up to one search.
const SEEK_PAUSE: Duration = Duration::from_secs(1);

impl ChoiceMarks {
    /// Marks the choice `by` on (back when below 0), within the choices.
    pub fn move_by(&mut self, by: i64) {
        let count = self.labels.len() as i64;
        if count == 0 {
            return;
        }
        let at = match self.highlighted {
            None if by > 0 => by - 1,
            None => count + by,
            Some(h) => h as i64 + by,
        };
        self.move_to(at);
    }

    fn move_to(&mut self, at: i64) {
        let count = self.labels.len() as i64;
        self.highlighted = Some(at.clamp(0, count - 1) as usize);
        self.reveal = true;
    }

    /// Letters typed while it is open: they add up, while typed close
    /// together, to the start of a choice's label it goes to; one letter
    /// typed again and again goes through the choices that start with it.
    pub fn seek(&mut self, text: &str, now: Instant) {
        if text.is_empty() || self.labels.is_empty() {
            return;
        }
        let fresh = self.sought_at.is_none_or(|at| now.saturating_duration_since(at) > SEEK_PAUSE);
        if fresh {
            self.sought = text.to_string();
        } else {
            self.sought.push_str(text);
        }
        self.sought_at = Some(now);
        let lower = self.sought.to_lowercase();
        let mut letters = lower.chars();
        let first = letters.next();
        let seek = if letters.all(|c| Some(c) == first) { first.map(String::from).unwrap_or_default() } else { lower };
        let count = self.labels.len() as i64;
        let start = if seek.chars().count() == 1 { self.highlighted.map_or(0, |h| h as i64 + 1) } else { self.highlighted.map_or(0, |h| h as i64) };
        for i in 0..count {
            let at = (start + i).rem_euclid(count);
            if self.labels[at as usize].to_lowercase().starts_with(&seek) {
                self.move_to(at);
                return;
            }
        }
    }
}

/// The choices of a choice cell, as a list over the window: a press on one
/// picks it. Its table works it with the keys: the arrows, Home and End move
/// the marked choice, Enter picks it, and the first letters of a choice go to it.
pub struct CellChoices {
    node: Node,
    choices: Vec<Choice>,
    value: Value,
    item_height: f64,
    scroller: Scroller,
    picked: Rc<Cell<Option<usize>>>,
    marks: Rc<RefCell<ChoiceMarks>>,
    /// The part it belongs to, which keeps the focus while the person picks.
    holder: Option<ElementId>,
    pressed_at: Option<usize>,
    pointer_y: Option<f64>,
    revealed: bool,
}

impl CellChoices {
    /// How many choices show at once; more scroll.
    pub const MOST_SHOWN: usize = 8;

    fn new(choices: Vec<Choice>, value: Value, item_height: f64, picked: Rc<Cell<Option<usize>>>, holder: Option<ElementId>) -> CellChoices {
        let marked = choices.iter().position(|c| rows::same(&c.value, &value));
        let marks = ChoiceMarks { labels: choices.iter().map(|c| c.label.clone()).collect(), highlighted: marked, ..ChoiceMarks::default() };
        CellChoices {
            node: Node::new(),
            choices,
            value,
            item_height,
            scroller: Scroller::default(),
            picked,
            marks: Rc::new(RefCell::new(marks)),
            holder,
            pressed_at: None,
            pointer_y: None,
            revealed: false,
        }
    }

    /// How high it is for `count` choices of `item_height`: at most `MOST_SHOWN`, with its border.
    pub fn height_for(count: usize, item_height: f64) -> f64 {
        count.clamp(1, Self::MOST_SHOWN) as f64 * item_height + 2.0
    }

    pub fn choices(&self) -> &[Choice] {
        &self.choices
    }

    /// Where its choices scroll.
    pub fn scroller(&self) -> &Scroller {
        &self.scroller
    }

    /// The box of the choice at `index` on the screen, in sight or not.
    pub fn item_box(&self, index: usize) -> Rect {
        self.scroller.row_on_screen(index, self.item_height, self.item_height)
    }

    /// Where its value is among its choices; none when it is none of them.
    pub fn marked(&self) -> Option<usize> {
        self.choices.iter().position(|c| rows::same(&c.value, &self.value))
    }

    /// The choice marked for Enter: its value's at first, then where the keys or the pointer go; none for none.
    pub fn highlighted(&self) -> Option<usize> {
        self.marks.borrow().highlighted
    }

    fn at(&self, y: f64) -> Option<usize> {
        self.scroller.index_at(y, self.item_height, self.choices.len())
    }
}

impl Element for CellChoices {
    element_node!();

    fn arrange(&mut self, area: Rect, _cx: &mut Cx) {
        self.node.bounds = area;
        self.scroller.place(area.inset(1.0), self.choices.len() as f64 * self.item_height);
        // It opens with its value's choice in view, as a dropdown's list does.
        if !self.revealed && area.height > 0.0 {
            self.revealed = true;
            if let Some(marked) = self.marked() {
                self.scroller.scroll_into(marked as f64 * self.item_height, self.item_height);
            }
        }
        // The choice the keys went to comes into view.
        let mut marks = self.marks.borrow_mut();
        if marks.reveal && area.height > 0.0 {
            marks.reveal = false;
            if let Some(at) = marks.highlighted {
                self.scroller.scroll_into(at as f64 * self.item_height, self.item_height);
            }
        }
    }

    fn draw(&self, p: &Painter) {
        let look = p.look;
        let menu = look.part("menu", None);
        let radius = menu.radius.map(|r| r as f64).unwrap_or_else(|| look.radius("input"));
        p.fill(self.node.bounds, menu.background.unwrap_or_else(|| look.color("panel")), radius);
        {
            let _clip = p.clip(self.scroller.bounds);
            let hover = self.highlighted().or_else(|| if self.node.hovered { self.pointer_y.and_then(|y| self.at(y)) } else { None });
            let (first, end) = self.scroller.in_sight(self.item_height, self.choices.len());
            for i in first..end {
                let item = self.item_box(i);
                let hovered = (Some(i) == hover).then(|| look.part("menu", Some("hover")));
                if rows::same(&self.choices[i].value, &self.value) {
                    p.fill(item, look.color("selection"), 0.0);
                } else if let Some(h) = &hovered {
                    p.fill(item, h.background.unwrap_or_else(|| look.color("accent_soft")), 0.0);
                }
                let color = hovered.and_then(|h| h.text).or(menu.text).unwrap_or_else(|| look.color("text"));
                let text = Rect { x: item.x + look.padding() * 2.0, width: (item.width - look.padding() * 4.0).max(0.0), ..item };
                p.text(&self.choices[i].label, text, color, look.size("text"), "text", false, TextAlign::Left);
            }
            self.scroller.draw(p, self.node.hovered);
        }
        p.stroke(self.node.bounds, menu.border.unwrap_or_else(|| look.color("border")), look.border(), radius);
    }

    fn on_pointer_pressed(&mut self, at: PointerInput, cx: &mut Cx) {
        if self.holder.is_some() {
            cx.focus(self.holder);
        }
        if self.scroller.press(at.screen_x, at.screen_y) {
            self.pressed_at = None;
            cx.invalidate();
            return;
        }
        self.pressed_at = self.at(at.screen_y);
    }

    fn on_pointer_moved(&mut self, at: PointerInput, cx: &mut Cx) {
        self.pointer_y = Some(at.screen_y);
        if let Some(over) = self.at(at.screen_y) {
            self.marks.borrow_mut().highlighted = Some(over);
        }
        self.scroller.drag(at.screen_x, at.screen_y);
        cx.invalidate();
    }

    fn on_pointer_released(&mut self, _at: PointerInput, _cx: &mut Cx) {
        self.scroller.release();
    }

    fn on_click(&mut self, cx: &mut Cx) {
        if let Some(at) = self.pressed_at.take() {
            self.picked.set(Some(at));
            cx.close(self.node.id());
        }
    }

    fn on_wheel(&mut self, dx: f64, dy: f64, held: Modifiers, cx: &mut Cx) -> bool {
        let moved = self.scroller.wheel_both(dx, dy, held);
        if moved {
            cx.invalidate();
        }
        moved
    }
}

/// What a table's Add button says, and what its rows' trash buttons say where the pointer rests, in the person's language.
pub fn add_word() -> String {
    locale::translate("addRow", "Add")
}

pub fn remove_word() -> String {
    locale::translate("removeRow", "Remove")
}
