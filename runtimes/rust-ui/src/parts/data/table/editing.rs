//! Editing a table in place (conformance/tables.json): the cell editor, typing
//! on with Tab, choices, a date's calendar, a folder, a color's palette, and
//! taking what was typed.

use super::*;

impl TablePart {
    /// Whether its cells take no edits: read-only, or not enabled.
    pub(super) fn locked(&self) -> bool {
        self.base.read_only() || !self.base.enabled()
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
    pub(super) fn open_calendar_of(&mut self, place: usize, column: usize, cell: Rect, cx: &mut Cx) {
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

    /// The person changed a cell: only a value that differs is an edit,
    /// written as the person's edit of the record's field; its row is chosen
    /// first (`select`), then `change` runs.
    pub(super) fn edit(&mut self, place: usize, field: &str, value: Value) {
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
    pub(super) fn begin_typing(&mut self, place: usize, column: usize, cx: &mut Cx) {
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
    pub(super) fn type_on(&mut self, place: usize, field: &str, back: bool, cx: &mut Cx) -> bool {
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
    pub(super) fn place_editor(&mut self, cx: &Cx) {
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
    pub(super) fn stop(&mut self, cx: &mut Cx) -> Option<Editing> {
        let e = self.editing.take()?;
        cx.invalidate();
        Some(e)
    }

    /// Takes what was typed into the cell being typed into: a value is an edit, text that is no value is marked.
    pub(super) fn take(&mut self, cx: &mut Cx) {
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
    pub(super) fn open_choices_of(&mut self, place: usize, column: usize, cell: Rect, cx: &mut Cx) {
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
    pub(super) fn input_of(column: &TableColumn, cell: Rect) -> Rect {
        if column.kind != CellKind::Bool {
            return cell.inset(3.0);
        }
        let (width, height) = if column.switch { (32.0, 18.0) } else { (16.0, 16.0) };
        const ROOM: f64 = 4.0;
        Rect::new(cell.x + (cell.width - width) / 2.0 - ROOM, cell.y + (cell.height - height) / 2.0 - ROOM, width + 2.0 * ROOM, height + 2.0 * ROOM)
    }

    pub(super) fn choose(&mut self, place: usize, cx: &mut Cx) {
        if self.rows.choose(&self.base, place) {
            self.show_place(place);
        }
        cx.invalidate();
    }

    pub(super) fn apply(&mut self, mv: Move, cx: &mut Cx) {
        let parent = self.is_tree().then(|| self.parent_field());
        let key = self.key_field();
        if let Some(now) = self.rows.apply(&self.base, mv, &key, parent.as_deref()) {
            self.sync();
            self.show_place(now);
        }
        cx.invalidate();
    }

    /// The button of a cell of an edited folder column, while it is in sight; none where nothing picks folders.
    pub fn folder_button_box(&self, place: usize, field: &str) -> Option<Rect> {
        let edited = self.columns.iter().any(|c| c.field == field && c.kind == CellKind::Folder && c.edit);
        if !edited || !crate::files::system().picks_folders() {
            return None;
        }
        self.rows.records.get(place)?;
        self.cell_box(place, field).map(calendar_button)
    }

    /// Asks the system's folder picker for the folder of a cell, off the screen's thread.
    pub(super) fn choose_folder(&mut self, place: usize, field: &str, cx: &mut Cx) {
        let picker = crate::files::system();
        let title = crate::parts::inputs::folder::choose_name();
        let field = field.to_string();
        cx.later(self.base.node.id(), move || FolderPicked(place, field, picker.open_folder(&crate::files::PickerOptions::titled(&title)).and_then(|f| f.path)));
    }

    /// The swatch of the color in a cell of a color column, while it is in sight; none for a cell without a color.
    pub fn swatch_box(&self, place: usize, field: &str) -> Option<Rect> {
        color_of(&rows::value_of(self.rows.records.get(place)?, field))?;
        self.swatch_place(place, field)
    }

    /// Where the swatch of a cell of a color column lies, with a color or without, while it is in sight.
    pub(super) fn swatch_place(&self, place: usize, field: &str) -> Option<Rect> {
        let column = self.columns.iter().find(|c| c.field == field && c.kind == CellKind::Color)?;
        self.rows.records.get(place)?;
        let cell = self.cell_box(place, field)?;
        let pad = self.look.padding;
        let left = if column.edit { cell.x + 3.0 + pad * 1.5 } else { cell.x + pad * 2.0 };
        Some(Rect::new(left, cell.y + (cell.height - SWATCH) / 2.0, SWATCH, SWATCH))
    }
}
