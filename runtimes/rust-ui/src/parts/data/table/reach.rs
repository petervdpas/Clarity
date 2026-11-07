//! How the person reaches every cell of a table beside the pointer, and what
//! the table says, as C# TablePart.Reach.cs: the check or choice the keys
//! work on (Tab walks every kind of edited cell, Space ticks a check, a
//! choice's list opens with Enter, Space or Alt+Down and is worked with the
//! keys), a date's calendar and a color's palette, going back to the
//! designed widths, and what the page bar's buttons and a tree's toggles say
//! where the pointer rests.

use std::cell::RefCell;
use std::rc::Rc;

use serde_json::Value;

use super::{calendar_button, CellChoices, CellKind, OpenPalette, PageButton, TablePart, EDGE, SWATCH};
use crate::choices;
use crate::element::Cx;
use crate::geometry::Rect;
use crate::input::{Cursor, Key, KeyInput, Modifiers};
use crate::overlay::Overlay;
use crate::parts::data::rows;
use crate::parts::inputs::calendar::Keyed;
use crate::parts::inputs::color::PaletteGrid;

impl TablePart {
    /// The cell the keys work on now: the one typed into, or a check or choice reached with Tab or a press; none for none.
    pub fn focused_cell(&self) -> Option<(usize, String)> {
        match &self.editing {
            Some(e) => Some((e.place, e.field.clone())),
            None => self.focused_cell.clone(),
        }
    }

    /// The choice marked for Enter in the open choice list; none while none is open or none is marked.
    pub fn open_choices_highlighted(&self) -> Option<usize> {
        self.choices.as_ref().and_then(|c| c.marks.borrow().highlighted)
    }

    /// The overlay of a color cell's palette, while it shows.
    pub fn open_palette_overlay(&self) -> Option<crate::element::ElementId> {
        self.palette.as_ref().map(|p| p.overlay)
    }

    /// A check or choice becomes the cell the keys work on: the table has the focus and the cell is in sight.
    pub(super) fn focus_cell(&mut self, place: usize, field: &str, cx: &mut Cx) {
        self.focused_cell = Some((place, field.to_string()));
        self.reveal_cell(place, field);
        cx.focus(Some(self.base.node.id()));
        cx.invalidate();
    }

    /// Scrolls as little as it takes, down and across, to show the cell of `field` in the row of `place`.
    pub(super) fn reveal_cell(&mut self, place: usize, field: &str) {
        let Some(at) = self.shown_places().iter().position(|p| *p == place) else { return };
        let Some(c) = self.column_index(field) else { return };
        let height = self.row_height();
        self.place_rows();
        let (x, width) = self.lefts(0.0)[c];
        self.rows.scroller.scroll_into(at as f64 * height, height);
        self.rows.scroller.scroll_into_x(x, width);
    }

    /// A key for an open list or calendar, or for the check or choice the
    /// keys work on; true when it took it. Another key leaves the cell and
    /// works on the rows.
    pub(super) fn cell_keyed(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        if !self.base.enabled() {
            return false;
        }
        if let Some(open) = &self.choices {
            let count = open.marks.borrow().labels.len() as i64;
            let most = CellChoices::MOST_SHOWN as i64;
            let plain = key.modifiers == Modifiers::NONE;
            let by = match key.key {
                Key::Down if plain => Some(1),
                Key::Up if key.modifiers == Modifiers::ALT => None,
                Key::Up => Some(-1),
                Key::PageDown => Some(most),
                Key::PageUp => Some(-most),
                Key::Home => Some(-count),
                Key::End => Some(count),
                _ => None,
            };
            if let Some(by) = by {
                open.marks.borrow_mut().move_by(by);
                return true;
            }
            match key.key {
                Key::Up | Key::Escape => {
                    cx.close(open.overlay);
                    return true;
                }
                Key::Enter | Key::Space => {
                    let marked = open.marks.borrow().highlighted;
                    if marked.is_some() {
                        open.picked.set(marked);
                        cx.close(open.overlay);
                    }
                    return true;
                }
                Key::Tab => cx.close(open.overlay),
                _ => return true,
            }
        }
        if let Some((overlay, _, _, calendar)) = &self.calendar {
            match calendar.keyed(key) {
                Keyed::Not => {}
                Keyed::Moved => return true,
                Keyed::Picked(_) | Keyed::Closed => {
                    cx.close(*overlay);
                    return true;
                }
            }
        }
        let Some((place, field)) = self.focused_cell.clone() else { return false };
        let Some(column) = self.columns.iter().find(|c| c.field == field).cloned().filter(|_| place < self.rows.records.len()) else {
            self.focused_cell = None;
            return false;
        };
        if key.key == Key::Tab && (key.modifiers == Modifiers::NONE || key.modifiers == Modifiers::SHIFT) {
            // Past the last cell the table lets Tab move the focus on.
            return self.type_on(place, &field, key.shift(), cx);
        }
        let plain = key.modifiers == Modifiers::NONE;
        if column.kind == CellKind::Bool && key.key == Key::Space && plain {
            let on = rows::value_of(&self.rows.records[place], &field) == Value::Bool(true);
            self.edit(place, &field, Value::Bool(!on));
            return true;
        }
        if column.kind == CellKind::Choice && ((matches!(key.key, Key::Enter | Key::Space) && plain) || (key.key == Key::Down && key.modifiers == Modifiers::ALT)) {
            if let (Some(c), Some(cell)) = (self.column_index(&field), self.cell_box(place, &field)) {
                self.open_choices_of(place, c, cell, cx);
            }
            return true;
        }
        self.focused_cell = None;
        key.key == Key::Escape && plain
    }

    // -- A date's calendar, a color's palette ---------------------------------

    /// The button of an edited date cell that opens its calendar, while it is in sight.
    pub fn calendar_button_box(&self, place: usize, field: &str) -> Option<Rect> {
        self.columns.iter().find(|c| c.field == field && c.kind == CellKind::Date && c.edit)?;
        self.cell_box(place, field).map(calendar_button)
    }

    /// The swatch at the left of an edited color cell's input, with a little room around it: a press there opens the palette.
    pub(super) fn palette_button(&self, cell: Rect) -> Rect {
        Rect::new(cell.x + 3.0, cell.y + 3.0, self.look.padding * 2.5 + SWATCH, (cell.height - 6.0).max(0.0))
    }

    /// Opens the palette the color part has under an edited color cell: the
    /// color picked is its edit. The table keeps the focus.
    pub(super) fn open_palette(&mut self, place: usize, field: &str, cx: &mut Cx) {
        self.reveal_cell(place, field);
        if self.locked() {
            return;
        }
        let Some(cell) = self.cell_box(place, field) else { return };
        let chosen = rows::value_of(&self.rows.records[place], field).as_str().map(str::to_string);
        let picked = Rc::new(RefCell::new(None));
        let me = self.base.node.id();
        let grid = PaletteGrid::new(chosen, picked.clone(), Some(me));
        let (width, height) = (PaletteGrid::width(), PaletteGrid::height());
        let whole = cx.whole();
        let x = cell.x.min(whole.width - width).max(0.0);
        let below = cell.bottom() + 2.0;
        let y = if below + height > whole.height && cell.y - 2.0 - height >= 0.0 { cell.y - 2.0 - height } else { below };
        let mut overlay = Overlay::new(Box::new(grid), Rect::new(x, y, width, height));
        overlay.anchor = Some(me);
        overlay.opener = Some(me);
        let id = cx.open(overlay);
        self.palette = Some(OpenPalette { overlay: id, place, field: field.to_string(), picked });
        cx.focus(Some(me));
        cx.invalidate();
    }

    // -- The person's widths ----------------------------------------------------

    /// The column whose right edge a point of the header is on, a few pixels
    /// either side of the line, with its header; none elsewhere or when the person may not resize.
    pub(super) fn edge_column_at(&self, x: f64, y: f64) -> Option<(String, Rect)> {
        if !self.resizable() || !self.has_header() || !self.header().contains(x, y) {
            return None;
        }
        self.columns.iter().find_map(|c| self.header_box(&c.field).filter(|b| y >= b.y && y <= b.bottom() && (x - b.right()).abs() <= EDGE).map(|b| (c.field.clone(), b)))
    }

    /// Back to the designed widths, forgetting the person's.
    pub fn designed_widths(&mut self) {
        self.widths_read = true;
        self.person_widths.clear();
        if let Some(key) = self.remember_key() {
            choices::forget(&key);
        }
        self.place_rows();
    }

    /// Where the person's widths are kept: `columns.Main.grid` for the
    /// resizable table grid of the window Main; none for a table without a
    /// name or outside a named window. Window and table names are the
    /// project's own, an open set, so the key is made from them.
    fn remember_key(&self) -> Option<String> {
        if !self.resizable() {
            return None;
        }
        let name = self.base.name().filter(|n| !n.is_empty())?;
        let window = self.base.host.window_name().filter(|w| !w.is_empty())?;
        Some(format!("columns.{window}.{name}"))
    }

    /// The widths the person left before, read once.
    pub(super) fn read_remembered_widths(&mut self) {
        if self.widths_read {
            return;
        }
        self.widths_read = true;
        let Some(text) = self.remember_key().and_then(|k| choices::kept(&k)).filter(|t| !t.is_empty()) else { return };
        // A damaged value: the designed widths.
        let Ok(widths) = serde_json::from_str::<serde_json::Map<String, Value>>(&text) else { return };
        for (field, width) in widths {
            if let Some(width) = width.as_f64().filter(|w| *w > 0.0) {
                self.person_widths.insert(field, width.clamp(32.0, 2000.0));
            }
        }
    }

    /// Each column's width as it shows, by field, as they are kept.
    pub(super) fn widths_written(&self) -> String {
        let lefts = self.lefts(0.0);
        let widths: serde_json::Map<String, Value> = self.columns.iter().zip(lefts).map(|(c, (_, w))| (c.field.clone(), Value::from(w.round()))).collect();
        Value::Object(widths).to_string()
    }

    /// The person let go of a header edge: the widths they left are kept, when they changed.
    pub(super) fn keep_widths(&self, before: &str) {
        let now = self.widths_written();
        if now != before {
            if let Some(key) = self.remember_key() {
                choices::keep_value(&key, &now);
            }
        }
    }

    // -- What it says -------------------------------------------------------------

    /// The button of the page bar at a point; none elsewhere.
    pub(super) fn pager_button_at(&self, x: f64, y: f64) -> Option<PageButton> {
        self.paged()?;
        PageButton::ALL.into_iter().find(|b| self.pager_box(*b).is_some_and(|r| r.contains(x, y)))
    }

    /// The place of the tree row whose toggle is at a point; none elsewhere.
    pub(super) fn toggle_at(&self, x: f64, y: f64) -> Option<usize> {
        if !self.is_tree() || !self.rows.scroller.bounds.contains(x, y) {
            return None;
        }
        let (place, _) = self.row_at(y)?;
        self.toggle_box(place).filter(|t| t.contains(x, y)).map(|_| place)
    }

    /// Its own button at a point: the Add button (false) or a row's trash button (true).
    fn own_button_at(&self, x: f64, y: f64) -> Option<bool> {
        if self.add_button().is_some_and(|b| b.contains(x, y)) {
            return Some(false);
        }
        let (place, _) = self.row_at(y).filter(|_| self.rows.scroller.bounds.contains(x, y))?;
        self.remove_box(place).filter(|b| b.contains(x, y)).map(|_| true)
    }

    /// The pointer's shape at a point: a header edge resizes; a page button that works and a toggle are pressed.
    pub fn pointer_shape(&self, x: f64, y: f64) -> Option<Cursor> {
        if matches!(self.press, Some(super::Press::Edge(..))) {
            return Some(Cursor::ResizeLeftRight);
        }
        if !self.base.enabled() {
            return None;
        }
        if let Some(button) = self.pager_button_at(x, y) {
            return self.page_button_works(button).then_some(Cursor::Hand);
        }
        if self.own_button_at(x, y).is_some() {
            return (!self.locked()).then_some(Cursor::Hand);
        }
        if self.edge_column_at(x, y).is_some() {
            return Some(Cursor::ResizeLeftRight);
        }
        if let Some(toggle) = self.toggle_at(x, y).map(|_| Cursor::Hand) {
            return Some(toggle);
        }
        if self.link_at(x, y).is_some() && !self.locked() {
            return Some(Cursor::Hand);
        }
        // A row that opens something shows it with the hand.
        (crate::parts::data::rows::rows_open(&self.base) && self.row_at(y).is_some()).then_some(Cursor::Hand)
    }

    /// What the page bar's buttons and a tree row's toggle say at a point, in Clarity's words.
    pub fn tip_at(&self, x: f64, y: f64) -> Option<String> {
        if let Some(button) = self.pager_button_at(x, y) {
            return Some(Self::page_word(button));
        }
        if let Some(remove) = self.own_button_at(x, y) {
            return Some(if remove { super::remove_word() } else { super::add_word() });
        }
        if let Some(word) = self.toggle_at(x, y).map(|place| self.toggle_word(place)) {
            return Some(word);
        }
        (crate::parts::data::rows::rows_open(&self.base) && self.row_at(y).is_some()).then(crate::parts::data::rows::open_word)
    }
}
