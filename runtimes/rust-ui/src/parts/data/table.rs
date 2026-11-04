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

mod reach;

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
}

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
    Bar,
    Editor,
}

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
    /// Whether its rows are a tree, under their parents.
    pub fn is_tree(&self) -> bool {
        matches!(self.base.spec.kind.as_str(), "tree" | "treeTable")
    }

    fn has_header(&self) -> bool {
        self.base.spec.kind != "tree"
    }

    fn sortable(&self) -> bool {
        self.base.spec.flag("sortable") && !self.is_tree()
    }

    fn resizable(&self) -> bool {
        self.base.spec.flag("resizable")
    }

    fn page_size(&self) -> Option<usize> {
        self.base.spec.number("page").filter(|s| *s >= 1.0 && !self.is_tree()).map(|s| s as usize)
    }

    /// Whether its cells take no edits: read-only, or not enabled.
    fn locked(&self) -> bool {
        self.base.read_only() || !self.base.enabled()
    }

    /// Whether it draws its own border.
    pub fn framed(&self) -> bool {
        rows::framed(&self.base)
    }

    pub fn columns(&self) -> &[TableColumn] {
        &self.columns
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

    /// The text editor of the cell typed into now; none while no cell is.
    pub fn cell_editor(&self) -> Option<&TextEditor> {
        self.editing.as_ref().map(|e| &e.editor)
    }

    /// The overlay of a choice cell's choices, while they show.
    pub fn open_choices(&self) -> Option<ElementId> {
        self.choices.as_ref().map(|c| c.overlay)
    }

    /// The calendar of a date or moment cell, while it is open.
    pub fn open_calendar(&self) -> Option<&Calendar> {
        self.calendar.as_ref().map(|c| &c.3)
    }

    /// Opens the calendar under a date or moment cell: the day picked is its edit (`pick_day`).
    fn open_calendar_of(&mut self, place: usize, column: usize, cell: Rect, cx: &mut Cx) {
        if self.locked() {
            return;
        }
        let field = self.columns[column].field.clone();
        self.reveal_cell(place, &field);
        let cell = self.cell_box(place, &field).unwrap_or(cell);
        let day = rows::value_of(&self.rows.records[place], &field).as_str().and_then(Day::parse);
        let me = self.base.node.id();
        let (id, calendar) = DayCalendar::open(cx, cell, day, &self.formats, None, Some(me), Some(me));
        self.calendar = Some((id, place, field, calendar));
    }

    // -- Columns ----------------------------------------------------------------

    /// The fields it shows: a tree the fields of `show`, else its record's first text; a table its `columns`.
    fn fields(&self) -> Vec<(String, Option<f64>)> {
        let widths = self.base.spec.json("widths").and_then(Value::as_object);
        let names = if self.base.spec.kind == "tree" {
            let show = self.base.spec.names("show");
            if show.is_empty() { vec![self.first_text()] } else { show }
        } else {
            self.base.spec.names("columns")
        };
        let mut out = Vec::new();
        for written in names {
            // `title 240`: a field and its designed width.
            let mut words = written.split_whitespace();
            let Some(field) = words.next() else { continue };
            let width = words.next().and_then(|w| w.parse::<f64>().ok()).or_else(|| widths.and_then(|w| w.get(field)).and_then(Value::as_f64));
            out.push((field.to_string(), width));
        }
        out
    }

    fn first_text(&self) -> String {
        self.rows
            .records
            .first()
            .and_then(Value::as_object)
            .and_then(|r| r.iter().find(|(_, v)| v.is_string()).map(|(k, _)| k.clone()))
            .unwrap_or_else(|| "name".to_string())
    }

    fn column_of(&self, field: &str, width: Option<f64>) -> TableColumn {
        let base = &self.base;
        let edit = base.spec.names("edit").iter().any(|f| f == field);
        let choices = Rows::choices_of(base, field);
        let path = base.bound().map(|b| rows::column_path(b, field));
        let facts = path.as_deref().and_then(|p| base.host.field(p));
        let typed = path.as_deref().and_then(|p| base.host.type_of(p)).and_then(|t| kind_of_type(&t));
        let formatted = facts.as_ref().and_then(|f| f.format.as_deref()).and_then(|f| match f {
            "date" => Some((CellKind::Date, false)),
            "datetime" | "moment" => Some((CellKind::Moment, false)),
            "integer" => Some((CellKind::Number, true)),
            "decimal" | "money" | "percent" => Some((CellKind::Number, false)),
            _ => None,
        });
        let seen = || self.rows.records.iter().map(|r| rows::value_of(r, field)).find(|v| !v.is_null()).and_then(|v| kind_of_value(&v));
        let (kind, integer) = if facts.as_ref().is_some_and(|f| f.color) {
            (CellKind::Color, false)
        } else if !choices.is_empty() {
            (CellKind::Choice, false)
        } else {
            typed.or(formatted).or_else(seen).unwrap_or((CellKind::Text, false))
        };
        // Its header: the field's label as the application gives it, else its name.
        let label = match facts.as_ref().map(|f| f.label.as_str()).filter(|l| !l.is_empty()) {
            Some(label) => base.host.text(Some(label)),
            None => base.host.text(Some(field)),
        };
        let switch = base.spec.names("switches").iter().any(|f| f == field);
        let format = match kind {
            CellKind::Number | CellKind::Date | CellKind::Moment => path
                .as_deref()
                .and_then(|p| format_of(base.host.as_ref(), p))
                .filter(|f| match kind {
                    CellKind::Number => !matches!(f.kind, FormatKind::Date | FormatKind::DateTime),
                    CellKind::Date => f.kind == FormatKind::Date,
                    _ => f.kind == FormatKind::DateTime,
                })
                .or_else(|| {
                    Some(FieldFormat::new(match kind {
                        CellKind::Date => FormatKind::Date,
                        CellKind::Moment => FormatKind::DateTime,
                        _ if integer => FormatKind::Integer,
                        _ => FormatKind::Decimal,
                    }))
                }),
            _ => None,
        };
        let integer = integer || format.as_ref().is_some_and(|f| f.kind == FormatKind::Integer);
        TableColumn { field: field.to_string(), label, kind, edit, switch, width, choices, integer, format }
    }

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

    fn read_columns(&mut self) {
        self.formats = formats_of(self.base.host.as_ref());
        self.columns = self.fields().into_iter().map(|(f, w)| self.column_of(&f, w)).collect();
        if !self.order_read {
            self.order_read = true;
            if let Some(o) = self.base.spec.json("order").and_then(Value::as_object) {
                if let Some(field) = o.get("field").and_then(Value::as_str) {
                    self.order = Some(TableOrder { field: field.to_string(), desc: o.get("desc").and_then(Value::as_bool).unwrap_or(false) });
                }
            }
        }
        if self.marked_for != self.rows.records {
            if self.marked_for.len() != self.rows.records.len() {
                self.marked.clear();
                self.editing = None;
            }
            self.marked_for = self.rows.records.clone();
        }
        if let Some(paged) = self.paged() {
            self.page = paged.page;
        }
    }

    // -- What shows ---------------------------------------------------------------

    fn tree_rows(&self) -> Vec<clarity_runtime::reorder::Row> {
        let parent = self.parent_field();
        self.rows
            .records
            .iter()
            .map(|r| clarity_runtime::reorder::Row { id: rows::value_of(r, "id"), parent: if self.is_tree() { rows::value_of(r, &parent) } else { Value::Null } })
            .collect()
    }

    /// The field of a record that holds its parent's id, in a tree: as the application's facts say, else `parent`.
    pub fn parent_field(&self) -> String {
        let facts = |f: &str| self.base.bound().and_then(|b| self.base.host.field(&rows::column_path(b, f))).and_then(|f| f.parent);
        facts("id").or_else(|| facts("parent")).unwrap_or_else(|| "parent".to_string())
    }

    fn tree_shown(&self) -> Vec<TreeShown> {
        trees::shown_tree(&self.tree_rows(), &self.opened)
    }

    /// The places of the rows in the order they show: by the table's order, a tree under their parents; all of them, before a page is cut out.
    fn ordered(&self) -> Vec<usize> {
        if self.is_tree() {
            return self.tree_shown().iter().map(|s| s.0).collect();
        }
        let Some(order) = &self.order else { return (0..self.rows.records.len()).collect() };
        let records: Vec<Map<String, Value>> = self.rows.records.iter().map(|r| r.as_object().cloned().unwrap_or_default()).collect();
        let columns: Vec<tables::TableColumn> = self
            .columns
            .iter()
            .map(|c| tables::TableColumn {
                field: c.field.clone(),
                kind: match c.kind {
                    CellKind::Choice => "choice",
                    CellKind::Number => "number",
                    CellKind::Bool => "bool",
                    _ => "text",
                }
                .to_string(),
                choices: c.choices.iter().map(|ch| vec![ch.value.clone(), Value::from(ch.label.clone())]).collect(),
            })
            .collect();
        tables::shown_order(&records, &columns, Some(order))
    }

    /// The page shown; none without `page`.
    pub fn paged(&self) -> Option<Paged> {
        self.page_size().map(|size| tables::page_of(self.rows.records.len(), size, self.page as i64))
    }

    /// The places of the rows shown, top to bottom: the page shown of the ordered rows.
    pub fn shown_places(&self) -> Vec<usize> {
        let ordered = self.ordered();
        match self.paged() {
            None => ordered,
            Some(paged) => {
                let from = paged.from.saturating_sub(1);
                ordered[from.min(ordered.len())..paged.to.min(ordered.len())].to_vec()
            }
        }
    }

    /// Where a row of a tree is: how deep, and whether rows are under it.
    pub fn tree_place(&self, place: usize) -> Option<TreeShown> {
        self.tree_shown().into_iter().find(|s| s.0 == place)
    }

    /// Whether a row of a tree shows the rows under it.
    pub fn is_open(&self, place: usize) -> bool {
        self.rows.records.get(place).is_some_and(|r| {
            let id = rows::value_of(r, "id");
            self.opened.iter().any(|o| rows::same(o, &id))
        })
    }

    /// Shows or hides the rows under a row of a tree.
    pub fn toggle(&mut self, place: usize) {
        let Some(record) = self.rows.records.get(place) else { return };
        let id = rows::value_of(record, "id");
        match self.opened.iter().position(|o| rows::same(o, &id)) {
            Some(at) => {
                self.opened.remove(at);
            }
            None => self.opened.push(id),
        }
        self.place_rows();
    }

    /// Clarity's own word on a tree row's toggle: what a press does.
    pub fn toggle_word(&self, place: usize) -> String {
        if self.is_open(place) { locale::translate("collapseRow", "Collapse") } else { locale::translate("expandRow", "Expand") }
    }

    /// Clarity's own word on a button of the page bar.
    pub fn page_word(button: PageButton) -> String {
        match button {
            PageButton::First => locale::translate("firstPage", "First page"),
            PageButton::Previous => locale::translate("previousPage", "Previous page"),
            PageButton::Next => locale::translate("nextPage", "Next page"),
            PageButton::Last => locale::translate("lastPage", "Last page"),
        }
    }

    /// Whether a button of the page bar works now: first and previous past the first page, next and last before the last.
    pub fn page_button_works(&self, button: PageButton) -> bool {
        match self.paged() {
            Some(p) if self.base.enabled() => match button {
                PageButton::First | PageButton::Previous => p.page > 1,
                _ => p.page < p.pages,
            },
            _ => false,
        }
    }

    /// Shows page `number`, kept within the pages there are.
    pub fn turn_to(&mut self, number: usize) {
        self.page = match self.paged() {
            Some(p) => number.clamp(1, p.pages),
            None => 1,
        };
        self.rows.scroller.scroll_to(0.0);
        self.place_rows();
    }

    /// Sorts by the column of `field`, or the other way when it already is; the first page shows.
    pub fn sort_by(&mut self, field: &str) {
        if !self.sortable() {
            return;
        }
        self.order = Some(tables::next_order(self.order.as_ref(), field));
        self.page = 1;
        self.rows.scroller.scroll_to(0.0);
        self.place_rows();
    }

    /// A row chosen shows: in a tree the rows above it open, in pages its page, and it scrolls into sight.
    fn show_place(&mut self, place: usize) {
        if self.is_tree() {
            for id in trees::above(&self.tree_rows(), place) {
                if !self.opened.iter().any(|o| rows::same(o, &id)) {
                    self.opened.push(id);
                }
            }
        }
        if let Some(size) = self.page_size() {
            if let Some(at) = self.ordered().iter().position(|p| *p == place) {
                let wanted = tables::page_with(size, at);
                if wanted != self.page {
                    self.page = wanted;
                    self.rows.scroller.scroll_to(0.0);
                }
            }
        }
        self.place_rows();
        if let Some(row) = self.shown_places().iter().position(|p| *p == place) {
            let height = self.row_height();
            self.rows.scroller.scroll_into(row as f64 * height, height);
        }
    }

    // -- Layout -------------------------------------------------------------------

    fn row_height(&self) -> f64 {
        self.look.text + 18.0
    }

    fn inner(&self) -> Rect {
        let b = self.base.node.bounds;
        if self.framed() { b.inset(self.look.border) } else { b }
    }

    fn header(&self) -> Rect {
        Rect { height: if self.has_header() { self.row_height() } else { 0.0 }, ..self.inner() }
    }

    /// The header's columns as they lie: scrolled sideways with the rows.
    fn header_row(&self) -> Rect {
        let header = self.header();
        Rect { x: header.x - self.rows.scroller.scrolled_x(), width: header.width.max(self.rows.scroller.content_width), ..header }
    }

    fn pager(&self) -> Rect {
        let inner = self.inner();
        let height = if self.page_size().is_some() { self.row_height() } else { 0.0 };
        Rect::new(inner.x, inner.bottom() - height, inner.width, height)
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
        let total: f64 = self.lefts(0.0).iter().map(|l| l.1).sum();
        let width = if total > body.width + 0.5 { total } else { body.width };
        self.rows.scroller.place_wide(body, content, width);
    }

    fn width_of(&self, column: &TableColumn) -> f64 {
        self.person_widths.get(&column.field).copied().or(column.width).unwrap_or(0.0)
    }

    /// A check's column is as wide as its header.
    fn check_width(&self, column: &TableColumn) -> f64 {
        let label = self.widths.get(&column.field).copied().unwrap_or(0.0);
        56f64.max(label.ceil() + 4.0 * self.look.padding + 16.0)
    }

    /// Each column's left edge and width from `left`: a designed width as it
    /// is, a check its own, the others sharing what is left of the place the
    /// rows show in (at least a little each, so together they may be wider
    /// and scroll sideways).
    fn lefts(&self, left: f64) -> Vec<(f64, f64)> {
        let room = self.body().width;
        const LEAST: f64 = 48.0;
        let fixed: f64 = self
            .columns
            .iter()
            .map(|c| {
                if self.width_of(c) > 0.0 {
                    self.width_of(c)
                } else if c.kind == CellKind::Bool {
                    self.check_width(c)
                } else {
                    0.0
                }
            })
            .sum();
        let sharing = self.columns.iter().filter(|c| self.width_of(c) <= 0.0 && c.kind != CellKind::Bool).count();
        let share = if sharing == 0 { 0.0 } else { LEAST.max((room - fixed) / sharing as f64) };
        let mut x = left;
        let mut lefts = Vec::with_capacity(self.columns.len());
        for column in &self.columns {
            let width = if self.width_of(column) > 0.0 {
                self.width_of(column)
            } else if column.kind == CellKind::Bool {
                self.check_width(column)
            } else {
                share
            };
            lefts.push((x, width));
            x += width;
        }
        lefts
    }

    fn column_index(&self, field: &str) -> Option<usize> {
        self.columns.iter().position(|c| c.field == field)
    }

    /// The box of the row of the record at `place` on the screen; none while it is not in sight.
    pub fn row_box(&self, place: usize) -> Option<Rect> {
        let at = self.shown_places().iter().position(|p| *p == place)?;
        let height = self.row_height();
        self.rows.scroller.row_in_sight(at, height, height)
    }

    /// The box of the cell of `field` in the row of `place`, while it is in sight.
    pub fn cell_box(&self, place: usize, field: &str) -> Option<Rect> {
        let c = self.column_index(field)?;
        let row = self.row_box(place)?;
        let (x, width) = self.lefts(row.x)[c];
        Some(Rect::new(x, row.y, width, row.height))
    }

    /// The swatch of the color in a cell of a color column, while it is in sight; none for a cell without a color.
    pub fn swatch_box(&self, place: usize, field: &str) -> Option<Rect> {
        color_of(&rows::value_of(self.rows.records.get(place)?, field))?;
        self.swatch_place(place, field)
    }

    /// Where the swatch of a cell of a color column lies, with a color or without, while it is in sight.
    fn swatch_place(&self, place: usize, field: &str) -> Option<Rect> {
        let column = self.columns.iter().find(|c| c.field == field && c.kind == CellKind::Color)?;
        self.rows.records.get(place)?;
        let cell = self.cell_box(place, field)?;
        let pad = self.look.padding;
        let left = if column.edit { cell.x + 3.0 + pad * 1.5 } else { cell.x + pad * 2.0 };
        Some(Rect::new(left, cell.y + (cell.height - SWATCH) / 2.0, SWATCH, SWATCH))
    }

    /// The box of the header of `field`; none without a header.
    pub fn header_box(&self, field: &str) -> Option<Rect> {
        if !self.has_header() {
            return None;
        }
        let header = self.header();
        let c = self.column_index(field)?;
        let (x, width) = self.lefts(self.header_row().x)[c];
        Some(Rect::new(x, header.y, width, header.height))
    }

    /// The box of a tree row's toggle, when rows are under it.
    pub fn toggle_box(&self, place: usize) -> Option<Rect> {
        if !self.is_tree() {
            return None;
        }
        let first = self.columns.first()?;
        let cell = self.cell_box(place, &first.field)?;
        let at = self.tree_place(place).filter(|t| t.2)?;
        Some(Rect::new(cell.x + self.look.padding + at.1 as f64 * INDENT, cell.y, 20.0, cell.height))
    }

    /// The box of a button of the page bar; none without pages.
    pub fn pager_box(&self, button: PageButton) -> Option<Rect> {
        self.paged()?;
        let bar = self.pager();
        const BUTTON: f64 = 26.0;
        let text = self.pager_text.ceil() + 16.0;
        let right = bar.right() - self.look.padding;
        let left = match button {
            PageButton::Last => right - BUTTON,
            PageButton::Next => right - 2.0 * BUTTON,
            PageButton::Previous => right - 3.0 * BUTTON - text,
            PageButton::First => right - 4.0 * BUTTON - text,
        };
        Some(Rect::new(left, bar.y, BUTTON, bar.height))
    }

    fn pager_text_box(&self) -> Option<Rect> {
        let previous = self.pager_box(PageButton::Previous)?;
        let next = self.pager_box(PageButton::Next)?;
        Some(Rect::new(previous.right(), previous.y, next.x - previous.right(), previous.height))
    }

    fn row_at(&self, y: f64) -> Option<(usize, Rect)> {
        let shown = self.shown_places();
        let height = self.row_height();
        let at = self.rows.scroller.index_at(y, height, shown.len())?;
        self.rows.scroller.row_in_sight(at, height, height).map(|b| (shown[at], b))
    }

    fn cell_at(&self, x: f64, y: f64) -> Option<(usize, usize, Rect)> {
        let (place, row) = self.row_at(y)?;
        let lefts = self.lefts(row.x);
        lefts.iter().enumerate().find(|(_, (lx, w))| x >= *lx && x < lx + w).map(|(c, (lx, w))| (place, c, Rect::new(*lx, row.y, *w, row.height)))
    }

    // -- What a cell shows and takes ----------------------------------------------

    /// The text a cell shows: what is typed into it, else its value as its column shows it.
    pub fn shown(&self, place: usize, field: &str) -> String {
        if let Some(e) = self.editing.as_ref().filter(|e| e.place == place && e.field == field) {
            return e.editor.text();
        }
        if let Some(typed) = self.marked.get(&(place, field.to_string())) {
            return typed.clone();
        }
        let (Some(column), Some(record)) = (self.columns.iter().find(|c| c.field == field), self.rows.records.get(place)) else {
            return String::new();
        };
        self.shown_value(column, &rows::value_of(record, field))
    }

    fn shown_value(&self, column: &TableColumn, value: &Value) -> String {
        if column.kind == CellKind::Choice {
            return rows::shown_of(value, &column.choices);
        }
        if !column.edit {
            return match &column.format {
                Some(format) if !value.is_null() && !value.is_boolean() => self.formats.show(format, value),
                _ => rows::shown_text(value),
            };
        }
        match column.kind {
            CellKind::Number => match (&column.format, value) {
                (_, Value::Null) => String::new(),
                (Some(format), _) => self.formats.edit_text(format, value),
                (None, other) => rows::shown_text(other),
            },
            // An edited moment shows its day beside its time; the time is what is typed.
            CellKind::Moment => moment_of(value).map(|m| m.format("%H:%M").to_string()).unwrap_or_default(),
            // An edited day is typed as the person writes a day.
            CellKind::Date => match day_of(value) {
                Some(d) => self.formats.show(&FieldFormat::new(FormatKind::Date), &Value::from(d.format("%Y-%m-%d").to_string())),
                None => rows::shown_text(value),
            },
            _ => rows::shown_text(value),
        }
    }

    /// Whether a cell holds typed text that is no value (it shows as typed and the value stays).
    pub fn marked(&self, place: usize, field: &str) -> bool {
        self.marked.contains_key(&(place, field.to_string()))
    }

    /// The person changed a cell: only a value that differs is an edit,
    /// written as the person's edit of the record's field; its row is chosen
    /// first (`select`), then `change` runs.
    fn edit(&mut self, place: usize, field: &str, value: Value) {
        if self.locked() {
            return;
        }
        let Some(bound) = self.base.bound().map(str::to_string) else { return };
        let Some(record) = self.rows.records.get(place) else { return };
        if rows::same(&rows::value_of(record, field), &value) {
            return;
        }
        self.base.host.write(self.base.name().unwrap_or(""), &rows::path(&bound, place, field), value);
        self.sync();
        if self.rows.choose(&self.base, place) {
            self.show_place(place);
        }
        self.base.fire("change");
    }

    /// A day picked for a date or moment cell (from a calendar): a moment keeps its time, an empty one starts at midnight.
    pub fn pick_day(&mut self, place: usize, field: &str, day: NaiveDate) {
        self.sync();
        let Some(column) = self.columns.iter().find(|c| c.field == field && c.edit).cloned() else { return };
        let Some(record) = self.rows.records.get(place) else { return };
        match column.kind {
            CellKind::Moment => {
                let moment = match moment_of(&rows::value_of(record, field)) {
                    Some(m) => day.and_time(m.time()),
                    None => match day.and_hms_opt(0, 0, 0) {
                        Some(midnight) => midnight,
                        None => return,
                    },
                };
                self.edit(place, field, Value::from(moment_text(moment)));
            }
            CellKind::Date => self.edit(place, field, Value::from(day.format("%Y-%m-%d").to_string())),
            _ => {}
        }
    }

    /// Starts typing into a text, number or moment cell: the text editor over
    /// it has all of its text selected, so what is typed first replaces it.
    fn begin_typing(&mut self, place: usize, column: usize, cx: &mut Cx) {
        self.take(cx);
        if self.locked() {
            return;
        }
        let column = self.columns[column].clone();
        let mut editor = TextEditor::new(false);
        editor.set_text(&self.shown(place, &column.field));
        editor.align = match column.kind {
            CellKind::Number => TextAlign::Right,
            CellKind::Moment => TextAlign::Center,
            _ => TextAlign::Left,
        };
        editor.takes_focus = false;
        editor.select_all();
        editor.focus_moved(true, cx);
        self.focused_cell = None;
        self.reveal_cell(place, &column.field);
        self.editing = Some(Editing { place, field: column.field, editor });
        self.place_editor(cx);
        cx.focus(Some(self.base.node.id()));
        cx.invalidate();
    }

    /// Tab in a cell: takes what was typed and goes on to the next edited
    /// cell of the shown rows (the one before with Shift), of every kind: a
    /// typed one is typed into, a check or a choice is the cell the keys work
    /// on. False past the last, so the focus leaves the table as Tab moves it.
    fn type_on(&mut self, place: usize, field: &str, back: bool, cx: &mut Cx) -> bool {
        let edited: Vec<usize> = (0..self.columns.len()).filter(|c| self.columns[*c].edit).collect();
        let cells: Vec<(usize, usize)> = self.shown_places().into_iter().flat_map(|p| edited.iter().map(move |c| (p, *c))).collect();
        let at = cells.iter().position(|(p, c)| *p == place && self.columns[*c].field == field);
        self.take(cx);
        self.focused_cell = None;
        let next = match at {
            None => return false,
            Some(at) if back => match at.checked_sub(1) {
                Some(n) => n,
                None => return false,
            },
            Some(at) => at + 1,
        };
        if next >= cells.len() || self.locked() {
            return false;
        }
        let (to, column) = cells[next];
        if matches!(self.columns[column].kind, CellKind::Bool | CellKind::Choice) {
            let field = self.columns[column].field.clone();
            self.focus_cell(to, &field, cx);
        } else {
            self.begin_typing(to, column, cx);
        }
        true
    }

    /// Places the cell's editor over its input (a moment's time at its right); out of sight it has no place.
    fn place_editor(&mut self, cx: &Cx) {
        let Some((place, field)) = self.editing.as_ref().map(|e| (e.place, e.field.clone())) else { return };
        let mut area = Rect::default();
        if let (Some(cell), Some(column)) = (self.cell_box(place, &field), self.columns.iter().find(|c| c.field == field)) {
            let cell = Rect { height: cell.height - self.look.border, ..cell };
            area = editor_box(column, cell);
        }
        let marked = self.marked(place, &field);
        if let Some(e) = self.editing.as_mut() {
            e.editor.marked = marked;
            e.editor.place(area, cx);
        }
    }

    /// Ends typing into the cell without taking the text.
    fn stop(&mut self, cx: &mut Cx) -> Option<Editing> {
        let e = self.editing.take()?;
        cx.invalidate();
        Some(e)
    }

    /// Takes what was typed into the cell being typed into: a value is an edit, text that is no value is marked.
    fn take(&mut self, cx: &mut Cx) {
        let Some(e) = self.stop(cx) else { return };
        let Some(column) = self.columns.iter().find(|c| c.field == e.field).cloned() else { return };
        let Some(record) = self.rows.records.get(e.place) else { return };
        let now = rows::value_of(record, &e.field);
        let typed = e.editor.text();
        let key = (e.place, e.field.clone());
        let value = match column.kind {
            CellKind::Number => {
                let trimmed = typed.trim();
                if trimmed.is_empty() {
                    Value::Null
                } else {
                    let format = column.format.clone().unwrap_or_else(|| FieldFormat::new(if column.integer { FormatKind::Integer } else { FormatKind::Decimal }));
                    // A decimal is kept as exact text, as the runtime holds it; one held as a JSON number stays one.
                    let read = self.formats.read(&format, trimmed).map(|v| match (&v, now.is_number()) {
                        (Value::String(text), true) => text.parse::<f64>().map(Value::from).unwrap_or(v),
                        _ => v,
                    });
                    match read {
                        Some(value) => value,
                        None => {
                            self.marked.insert(key, typed);
                            return;
                        }
                    }
                }
            }
            CellKind::Date => {
                // A day as the person writes it; cleared, it is none.
                if typed.trim().is_empty() {
                    Value::Null
                } else {
                    match self.formats.read_date(&typed).and_then(|(y, m, d)| NaiveDate::from_ymd_opt(y, m, d)) {
                        Some(day) => Value::from(day.format("%Y-%m-%d").to_string()),
                        None => {
                            self.marked.insert(key, typed);
                            return;
                        }
                    }
                }
            }
            CellKind::Moment => {
                let Some((h, m)) = time_typed(&typed) else {
                    if !typed.trim().is_empty() {
                        self.marked.insert(key, typed);
                    }
                    return;
                };
                // Without a day, a time waits for one.
                let Some(day) = moment_of(&now) else { return };
                let Some(moment) = day.date().and_hms_opt(h, m, 0) else { return };
                Value::from(moment_text(moment))
            }
            _ => {
                let held = if now.is_null() { Value::from("") } else { now.clone() };
                if rows::same(&held, &Value::from(typed.clone())) {
                    self.marked.remove(&key);
                    return;
                }
                Value::from(typed)
            }
        };
        self.marked.remove(&key);
        self.edit(e.place, &e.field, value);
    }

    /// Opens the choices of a choice cell under it.
    fn open_choices_of(&mut self, place: usize, column: usize, cell: Rect, cx: &mut Cx) {
        if self.locked() {
            return;
        }
        let column = self.columns[column].clone();
        self.reveal_cell(place, &column.field);
        let cell = self.cell_box(place, &column.field).unwrap_or(cell);
        let height = self.row_height() - 4.0;
        let value = rows::value_of(&self.rows.records[place], &column.field);
        let picked = Rc::new(Cell::new(None));
        let me = self.base.node.id();
        let list = CellChoices::new(column.choices.clone(), value, height, picked.clone(), Some(me));
        let marks = list.marks.clone();
        let mut below = Rect::new(cell.x, cell.bottom(), cell.width.max(120.0), CellChoices::height_for(column.choices.len(), height));
        if below.bottom() > cx.whole().height && cell.y - below.height >= 0.0 {
            below.y = cell.y - below.height;
        }
        let mut overlay = Overlay::new(Box::new(list), below);
        overlay.opener = Some(self.base.node.id());
        let id = cx.open(overlay);
        self.choices = Some(OpenChoices { overlay: id, place, field: column.field, picked, marks });
        cx.focus(Some(me));
    }

    /// Where an edited cell's input takes a press: a switch or a check with a little room around it, else the input inside the cell's margin.
    fn input_of(column: &TableColumn, cell: Rect) -> Rect {
        if column.kind != CellKind::Bool {
            return cell.inset(3.0);
        }
        let (width, height) = if column.switch { (32.0, 18.0) } else { (16.0, 16.0) };
        const ROOM: f64 = 4.0;
        Rect::new(cell.x + (cell.width - width) / 2.0 - ROOM, cell.y + (cell.height - height) / 2.0 - ROOM, width + 2.0 * ROOM, height + 2.0 * ROOM)
    }

    fn choose(&mut self, place: usize, cx: &mut Cx) {
        if self.rows.choose(&self.base, place) {
            self.show_place(place);
        }
        cx.invalidate();
    }

    fn apply(&mut self, mv: Move, cx: &mut Cx) {
        let parent = self.is_tree().then(|| self.parent_field());
        if let Some(now) = self.rows.apply(&self.base, mv, parent.as_deref()) {
            self.sync();
            self.show_place(now);
        }
        cx.invalidate();
    }

    // -- Drawing --------------------------------------------------------------------

    fn draw_header(&self, p: &Painter, header: Rect) {
        let look = p.look;
        let shown = look.part("header", None);
        p.fill(header, shown.background.unwrap_or_else(|| look.color("raised")), 0.0);
        p.fill(Rect { y: header.bottom() - look.border(), height: look.border(), ..header }, shown.border.unwrap_or_else(|| look.color("border")), 0.0);
        let lefts = self.lefts(self.header_row().x);
        let pad = look.padding() * 2.0;
        let size = shown.size.map(|s| s as f64).unwrap_or_else(|| look.size("text"));
        let font = shown.font.clone().unwrap_or_else(|| "text".to_string());
        for (c, column) in self.columns.iter().enumerate() {
            let cell = Rect::new(lefts[c].0, header.y, lefts[c].1, header.height);
            let color = shown.text.unwrap_or_else(|| look.color("muted"));
            let text = Rect { x: cell.x + pad, width: (cell.width - 2.0 * pad - 12.0).max(0.0), ..cell };
            p.text(&column.label, text, color, size, &font, shown.bold(), TextAlign::Left);
            if let Some(o) = self.order.as_ref().filter(|o| o.field == column.field) {
                let width = p.fonts.measure(&column.label, &font, size, shown.bold());
                let x = (text.x + width + 6.0).min(cell.right() - 14.0);
                chevron(p, x + 4.0, cell.y + cell.height / 2.0, if o.desc { "down" } else { "up" }, color);
            }
            if c > 0 {
                p.fill(Rect::new(cell.x, cell.y + 6.0, look.border(), cell.height - 12.0), look.color("border"), 0.0);
            }
        }
    }

    fn draw_rows(&self, p: &Painter) {
        let shown = self.shown_places();
        let height = self.row_height();
        let hover = match self.rows.pointer {
            Some((_, y)) if self.base.node.hovered && !self.rows.dragging() => self.row_at(y).map(|r| r.0),
            _ => None,
        };
        let chosen = self.rows.chosen(&self.base);
        let (first, end) = self.rows.scroller.in_sight(height, shown.len());
        for (at, place) in shown.iter().enumerate().take(end).skip(first) {
            let row = self.rows.scroller.row_on_screen(at, height, height);
            let state = if Some(*place) == chosen {
                Some("selected")
            } else if Some(*place) == hover {
                Some("hover")
            } else {
                None
            };
            self.draw_row(p, *place, row, state);
            if let Some((target, where_)) = self.rows.drop {
                if target == *place {
                    rows::draw_drop(p, row, where_);
                }
            }
        }
    }

    fn draw_row(&self, p: &Painter, place: usize, row: Rect, state: Option<&str>) {
        let look = p.look;
        let text = rows::draw_row_back(p, row, state).text.unwrap_or_else(|| look.color("text"));
        let lefts = self.lefts(row.x);
        for (c, column) in self.columns.iter().enumerate() {
            let cell = Rect::new(lefts[c].0, row.y, lefts[c].1, row.height - look.border());
            {
                let _clip = p.clip(cell);
                self.draw_cell(p, place, column, cell, text, c == 0);
            }
            // The check or choice the keys work on shows the focus ring.
            if self.base.node.focused && self.focused_cell.as_ref().is_some_and(|(fp, ff)| *fp == place && *ff == column.field) {
                p.focus_ring(Self::input_of(column, cell), look.radius("input"));
            }
        }
    }

    fn draw_cell(&self, p: &Painter, place: usize, column: &TableColumn, mut cell: Rect, text: Color, first: bool) {
        let look = p.look;
        let pad = look.padding() * 2.0;
        let size = look.size("text");
        if first && self.is_tree() {
            if let Some(at) = self.tree_place(place) {
                let indent = look.padding() + at.1 as f64 * INDENT;
                if at.2 {
                    chevron(p, cell.x + indent + 10.0, cell.y + cell.height / 2.0, if self.is_open(place) { "down" } else { "right" }, look.color("muted"));
                }
                let shift = indent + 20.0 - pad + look.padding();
                cell = Rect { x: cell.x + shift, width: (cell.width - shift).max(0.0), ..cell };
            }
        }
        let value = rows::value_of(&self.rows.records[place], &column.field);
        let inert = self.locked();
        if !column.edit {
            let align = if column.kind == CellKind::Number { TextAlign::Right } else { TextAlign::Left };
            let lead = self.draw_swatch(p, place, column);
            let area = Rect { x: cell.x + pad + lead, width: (cell.width - 2.0 * pad - lead).max(0.0), ..cell };
            p.text(&self.shown(place, &column.field), area, text, size, "text", false, align);
            return;
        }
        match column.kind {
            CellKind::Bool if column.switch => draw_switch(p, cell, value == Value::Bool(true), inert),
            CellKind::Bool => draw_check(p, cell, value == Value::Bool(true), inert),
            CellKind::Moment => {
                let day = match moment_of(&value) {
                    Some(m) => self.formats.show(&FieldFormat::new(FormatKind::Date), &Value::from(m.format("%Y-%m-%d").to_string())),
                    None => String::new(),
                };
                let time = time_box(cell);
                self.draw_input(p, Rect { width: time.x - cell.x, ..cell }, &day, place, &column.field, false, TextAlign::Left, 0.0);
                self.draw_input(p, time, &self.shown(place, &column.field), place, &column.field, true, TextAlign::Center, 0.0);
            }
            CellKind::Date => {
                // A typed day beside the button that opens its calendar.
                self.draw_input(p, cell, "", place, &column.field, false, TextAlign::Left, 0.0);
                self.draw_input(p, cell, &self.shown(place, &column.field), place, &column.field, true, TextAlign::Left, 0.0);
                let button = calendar_button(cell);
                chevron(p, button.x + button.width / 2.0, button.y + button.height / 2.0, "down", look.color("muted"));
            }
            CellKind::Choice => {
                self.draw_input(p, cell, &self.shown(place, &column.field), place, &column.field, false, TextAlign::Left, 0.0);
                chevron(p, cell.right() - 14.0, cell.y + cell.height / 2.0, "down", look.color("muted"));
            }
            CellKind::Color => {
                let editing = self.editing.as_ref().is_some_and(|e| e.place == place && e.field == column.field);
                let lead = if editing { 0.0 } else { SWATCH + look.padding() * 1.5 };
                self.draw_input(p, cell, &self.shown(place, &column.field), place, &column.field, true, TextAlign::Left, lead);
                if !editing {
                    self.draw_swatch(p, place, column);
                }
            }
            _ => {
                let align = if column.kind == CellKind::Number { TextAlign::Right } else { TextAlign::Left };
                self.draw_input(p, cell, &self.shown(place, &column.field), place, &column.field, true, align, 0.0);
            }
        }
    }

    /// Draws the swatch of a color cell, and gives the room its text keeps beside it.
    fn draw_swatch(&self, p: &Painter, place: usize, column: &TableColumn) -> f64 {
        if column.kind != CellKind::Color {
            return 0.0;
        }
        let look = p.look;
        let radius = look.radius("input").min(2.0);
        let color = color_of(&rows::value_of(&self.rows.records[place], &column.field));
        let (Some(swatch), Some(color)) = (self.swatch_box(place, &column.field), color) else {
            // An edited cell without a color shows an empty swatch, crossed, which opens the palette all the same.
            let Some(empty) = self.swatch_place(place, &column.field).filter(|_| column.edit) else { return 0.0 };
            p.stroke(empty, look.color("strong_border"), look.border(), radius);
            line(p, empty.x + 2.0, empty.bottom() - 2.0, empty.right() - 2.0, empty.y + 2.0, look.color("muted"));
            return SWATCH + look.padding() * 1.5;
        };
        p.fill(swatch, color, radius);
        p.stroke(swatch, look.color("strong_border"), look.border(), radius);
        SWATCH + look.padding() * 1.5
    }

    /// An input in a cell, in the theme's input look, marked when it holds
    /// text that is no value; the cell typed into shows its text editor there.
    #[allow(clippy::too_many_arguments)]
    fn draw_input(&self, p: &Painter, cell: Rect, text: &str, place: usize, field: &str, typed: bool, align: TextAlign, lead: f64) {
        let look = p.look;
        let area = cell.inset(3.0);
        if typed {
            if let Some(e) = self.editing.as_ref().filter(|e| e.place == place && e.field == field) {
                e.editor.paint(p);
                return;
            }
        }
        let input = look.part("input", None);
        let radius = input.radius.map(|r| r as f64).unwrap_or_else(|| look.radius("input"));
        p.fill(area, input.background.unwrap_or_else(|| look.color("input")), radius);
        let border = if typed && self.marked(place, field) { look.color("error") } else { input.border.unwrap_or_else(|| look.color("strong_border")) };
        p.stroke(area, border, look.border(), radius);
        let inside = Rect {
            x: area.x + look.padding() * 1.5 + lead,
            width: (area.width - look.padding() * 3.0 - lead - if align == TextAlign::Left { 12.0 } else { 0.0 }).max(0.0),
            ..area
        };
        p.text(text, inside, input.text.unwrap_or_else(|| look.color("text")), look.size("text"), "text", false, align);
    }

    fn draw_pager(&self, p: &Painter, paged: Paged) {
        let look = p.look;
        let bar = self.pager();
        p.fill(Rect { height: look.border(), ..bar }, look.color("border"), 0.0);
        for button in PageButton::ALL {
            let Some(b) = self.pager_box(button) else { continue };
            let color = if self.page_button_works(button) { look.color("text") } else { look.color("muted").with_a(110) };
            let (x, y) = (b.x + b.width / 2.0, b.y + b.height / 2.0);
            match button {
                PageButton::First => {
                    chevron(p, x + 2.0, y, "left", color);
                    line(p, x - 4.0, y - 5.0, x - 4.0, y + 5.0, color);
                }
                PageButton::Previous => chevron(p, x, y, "left", color),
                PageButton::Next => chevron(p, x, y, "right", color),
                PageButton::Last => {
                    chevron(p, x - 2.0, y, "right", color);
                    line(p, x + 4.0, y - 5.0, x + 4.0, y + 5.0, color);
                }
            }
        }
        if let Some(text) = self.pager_text_box() {
            p.text(&paged.text(), text, look.color("text"), look.size("text"), "text", false, TextAlign::Center);
        }
    }
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

    fn focusable(&self) -> bool {
        self.base.enabled()
    }

    /// While a cell has the keys, Tab goes on to the next cell, and from the last out of the table.
    fn wants_tab(&self) -> bool {
        self.editing.is_some() || self.focused_cell.is_some() || self.choices.is_some()
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
        self.place_rows();
        self.place_editor(cx);
    }

    fn draw(&self, p: &Painter) {
        let look = p.look;
        let b = self.base.node.bounds;
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
        if self.pager().contains(x, y) {
            self.press = PageButton::ALL.into_iter().find(|b| self.pager_box(*b).is_some_and(|r| r.contains(x, y))).map(Press::Pager);
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
                CellKind::Text | CellKind::Number | CellKind::Color | CellKind::Date => {
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
