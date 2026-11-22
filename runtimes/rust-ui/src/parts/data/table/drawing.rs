//! Drawing a table: its header, its rows in sight and each cell by its input.

use super::*;

impl TablePart {
    pub(super) fn draw_header(&self, p: &Painter, header: Rect) {
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

    pub(super) fn draw_rows(&self, p: &Painter) {
        let shown = self.shown_places();
        let height = self.row_height();
        let hover = match self.rows.pointer {
            Some((_, y)) if self.base.node.hovered && !self.rows.dragging() => self.row_at(y).map(|r| r.0),
            _ => None,
        };
        let chosen = self.rows.chosen(&self.base);
        let striped = rows::striped(&self.base);
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
            self.draw_row(p, *place, row, state, striped && at % 2 == 1);
            if let Some((target, where_)) = self.rows.drop {
                if target == *place {
                    rows::draw_drop(p, row, where_);
                }
            }
        }
    }

    pub(super) fn draw_row(&self, p: &Painter, place: usize, row: Rect, state: Option<&str>, stripe: bool) {
        let look = p.look;
        let text = rows::draw_row_back(p, row, state, stripe).text.unwrap_or_else(|| look.color("text"));
        // The row's trash button, red under the pointer.
        if let Some(trash) = self.remove_box(place).map(|b| Rect { height: b.height - look.border(), ..b }) {
            let over = self.base.node.hovered && self.rows.pointer.is_some_and(|(x, y)| trash.contains(x, y)) && !self.locked();
            let icon = Rect::new(trash.x + (trash.width - OWN_ICON) / 2.0, trash.y + (trash.height - OWN_ICON) / 2.0, OWN_ICON, OWN_ICON);
            let _dim = p.dimmed(!self.locked());
            crate::parts::containers::icon::draw_icon(p, self.base.host.icon_path("trash").as_deref(), icon, look.color(if over { "error" } else { "muted" }));
        }
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

    pub(super) fn draw_cell(&self, p: &Painter, place: usize, column: &TableColumn, mut cell: Rect, text: Color, first: bool) {
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
            let shown = self.shown(place, &column.field);
            // A link column: its text in the accent color, underlined, as a link reads.
            if column.link.is_some() && !shown.is_empty() {
                let ink = if inert { text } else { look.color("accent") };
                p.text(&shown, area, ink, size, "text", false, align);
                let wide = p.fonts.measure(&shown, "text", size, false).min(area.width);
                let base_line = area.y + area.height / 2.0 + size * 0.6;
                let from = if align == TextAlign::Right { area.right() - wide } else { area.x };
                p.line((from, base_line), (from + wide, base_line), ink, 1.0);
                return;
            }
            p.text(&shown, area, text, size, "text", false, align);
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
            CellKind::Folder if column.edit && crate::files::system().picks_folders() => {
                // A typed path beside the button that opens the folder picker.
                self.draw_input(p, cell, "", place, &column.field, false, TextAlign::Left, 0.0);
                let button = calendar_button(cell);
                let text = Rect { width: (cell.width - CALENDAR_BUTTON - 3.0).max(0.0), ..cell };
                self.draw_input(p, text, &self.shown(place, &column.field), place, &column.field, true, TextAlign::Left, 0.0);
                let middle = button.y + button.height / 2.0;
                for k in [-4.0, 0.0, 4.0] {
                    p.fill(Rect::new(button.x + button.width / 2.0 + k - 1.0, middle - 1.0, 2.0, 2.0), look.color("muted"), 1.0);
                }
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
    pub(super) fn draw_swatch(&self, p: &Painter, place: usize, column: &TableColumn) -> f64 {
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
    pub(super) fn draw_input(&self, p: &Painter, cell: Rect, text: &str, place: usize, field: &str, typed: bool, align: TextAlign, lead: f64) {
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
}
