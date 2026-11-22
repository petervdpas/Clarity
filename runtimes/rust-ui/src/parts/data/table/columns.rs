//! A table's columns and cells: reading the columns, their widths and places,
//! the header, which row and cell are where, and what a cell shows.

use super::*;

impl TablePart {
    pub(super) fn has_header(&self) -> bool {
        self.base.spec.kind != "tree"
    }

    pub(super) fn resizable(&self) -> bool {
        self.base.spec.flag("resizable")
    }

    pub fn columns(&self) -> &[TableColumn] {
        &self.columns
    }

    pub(super) fn read_columns(&mut self) {
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

    pub(super) fn column_of(&self, field: &str, width: Option<f64>) -> TableColumn {
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
        } else if facts.as_ref().is_some_and(|f| f.folder) {
            (CellKind::Folder, false)
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
        let link = match base.spec.props.get("links") {
            Some(Value::Array(links)) => links.iter().find(|l| l.get("column").and_then(Value::as_str) == Some(field)).and_then(|l| l.get("handler")).and_then(Value::as_str).map(str::to_string),
            _ => None,
        };
        TableColumn { field: field.to_string(), label, kind, edit, switch, width, choices, integer, format, link }
    }

    /// The fields it shows: a tree the fields of `show`, else its record's first text; a table its `columns`.
    pub(super) fn fields(&self) -> Vec<(String, Option<f64>)> {
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

    pub(super) fn first_text(&self) -> String {
        self.rows
            .records
            .first()
            .and_then(Value::as_object)
            .and_then(|r| r.iter().find(|(_, v)| v.is_string()).map(|(k, _)| k.clone()))
            .unwrap_or_else(|| "name".to_string())
    }

    /// The link a point is on: the row and the handler of a link column's cell, where its text is.
    pub(super) fn link_at(&self, x: f64, y: f64) -> Option<(usize, String)> {
        let (place, c, _) = self.cell_at(x, y)?;
        let column = self.columns.get(c)?;
        if column.edit {
            return None;
        }
        column.link.clone().map(|handler| (place, handler))
    }

    pub(super) fn width_of(&self, column: &TableColumn) -> f64 {
        self.person_widths.get(&column.field).copied().or(column.width).unwrap_or(0.0)
    }

    /// A check's column is as wide as its header.
    pub(super) fn check_width(&self, column: &TableColumn) -> f64 {
        let label = self.widths.get(&column.field).copied().unwrap_or(0.0);
        56f64.max(label.ceil() + 4.0 * self.look.padding + 16.0)
    }

    /// Each column's left edge and width from `left`: a designed width as it
    /// is, a check its own, the others sharing what is left of the place the
    /// rows show in (at least a little each, so together they may be wider
    /// and scroll sideways).
    pub(super) fn lefts(&self, left: f64) -> Vec<(f64, f64)> {
        // The trash buttons' column keeps its own width at the end.
        let room = self.body().width - if self.removes() { REMOVE_WIDTH } else { 0.0 };
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

    pub(super) fn column_index(&self, field: &str) -> Option<usize> {
        self.columns.iter().position(|c| c.field == field)
    }

    pub(super) fn header(&self) -> Rect {
        let bar = self.fold_bar();
        Rect { y: bar.bottom(), height: if self.has_header() { self.row_height() } else { 0.0 }, ..self.inner() }
    }

    /// The header's columns as they lie: scrolled sideways with the rows.
    pub(super) fn header_row(&self) -> Rect {
        let header = self.header();
        Rect { x: header.x - self.rows.scroller.scrolled_x(), width: header.width.max(self.rows.scroller.content_width), ..header }
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

    /// The box of the cell of `field` in the row of `place`, while it is in sight.
    pub fn cell_box(&self, place: usize, field: &str) -> Option<Rect> {
        let c = self.column_index(field)?;
        let row = self.row_box(place)?;
        let (x, width) = self.lefts(row.x)[c];
        Some(Rect::new(x, row.y, width, row.height))
    }

    /// The box of the row of the record at `place` on the screen; none while it is not in sight.
    pub fn row_box(&self, place: usize) -> Option<Rect> {
        let at = self.shown_places().iter().position(|p| *p == place)?;
        let height = self.row_height();
        self.rows.scroller.row_in_sight(at, height, height)
    }

    pub(super) fn row_at(&self, y: f64) -> Option<(usize, Rect)> {
        let shown = self.shown_places();
        let height = self.row_height();
        let at = self.rows.scroller.index_at(y, height, shown.len())?;
        self.rows.scroller.row_in_sight(at, height, height).map(|b| (shown[at], b))
    }

    pub(super) fn cell_at(&self, x: f64, y: f64) -> Option<(usize, usize, Rect)> {
        let (place, row) = self.row_at(y)?;
        let lefts = self.lefts(row.x);
        lefts.iter().enumerate().find(|(_, (lx, w))| x >= *lx && x < lx + w).map(|(c, (lx, w))| (place, c, Rect::new(*lx, row.y, *w, row.height)))
    }

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

    pub(super) fn shown_value(&self, column: &TableColumn, value: &Value) -> String {
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
}
