//! A table's order and pages (conformance/table-order.json, table-pages.json):
//! sorting by a header, the page shown, its pager and turning to a row.

use super::*;

impl TablePart {
    pub(super) fn sortable(&self) -> bool {
        self.base.spec.flag("sortable") && !self.is_tree()
    }

    pub(super) fn page_size(&self) -> Option<usize> {
        self.base.spec.number("page").filter(|s| *s >= 1.0 && !self.is_tree()).map(|s| s as usize)
    }

    /// The places of the rows in the order they show: by the table's order, a tree under their parents; all of them, before a page is cut out.
    pub(super) fn ordered(&self) -> Vec<usize> {
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
    pub(super) fn show_place(&mut self, place: usize) {
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

    pub(super) fn pager(&self) -> Rect {
        let inner = self.inner();
        let height = if self.page_size().is_some() { self.row_height() } else { 0.0 };
        Rect::new(inner.x, inner.bottom() - height, inner.width, height)
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

    pub(super) fn pager_text_box(&self) -> Option<Rect> {
        let previous = self.pager_box(PageButton::Previous)?;
        let next = self.pager_box(PageButton::Next)?;
        Some(Rect::new(previous.right(), previous.y, next.x - previous.right(), previous.height))
    }

    pub(super) fn draw_pager(&self, p: &Painter, paged: Paged) {
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
