//! A table in tree mode (conformance/trees.json): its rows under their parents
//! by key or parent field, opened and closed, with the fold bar.

use super::*;

impl TablePart {
    /// Whether its rows are a tree, under their parents.
    pub fn is_tree(&self) -> bool {
        matches!(self.base.spec.kind.as_str(), "tree" | "treeTable")
    }

    /// Whether a tree shows Expand all and Collapse all above its rows: as code set `foldButtons`, else as its line says (`fold buttons`).
    pub fn folds(&self) -> bool {
        self.is_tree() && self.base.state().and_then(|s| s.fold_buttons).unwrap_or_else(|| self.base.spec.flag("foldButtons"))
    }

    /// The bar above the rows that holds Expand all and Collapse all; no height without them.
    pub(super) fn fold_bar(&self) -> Rect {
        Rect { height: if self.folds() { self.row_height() } else { 0.0 }, ..self.inner() }
    }

    /// Where Expand all (`expand`) or Collapse all is; none without fold buttons.
    pub fn fold_box(&self, expand: bool) -> Option<Rect> {
        if !self.folds() {
            return None;
        }
        let bar = self.fold_bar();
        let (open, close) = (self.fold_text.0.ceil() + 2.0 * self.look.padding, self.fold_text.1.ceil() + 2.0 * self.look.padding);
        let x = bar.x + self.look.padding;
        Some(if expand { Rect::new(x, bar.y, open, bar.height) } else { Rect::new(x + open + self.look.padding, bar.y, close, bar.height) })
    }

    /// Opens every row of the tree (`expand`), or closes them all.
    pub fn fold_all(&mut self, expand: bool) {
        self.opened = if expand { self.tree_rows().into_iter().map(|r| r.id).filter(|id| !id.is_null()).collect() } else { Vec::new() };
        self.rows.scroller.scroll_to(0.0);
        self.place_rows();
    }

    pub(super) fn tree_rows(&self) -> Vec<clarity_runtime::reorder::Row> {
        let key = self.key_field();
        let parent = self.parent_field();
        self.rows
            .records
            .iter()
            .map(|r| clarity_runtime::reorder::Row {
                id: trees::key_of(rows::value_of(r, &key)),
                parent: if self.is_tree() { trees::key_of(rows::value_of(r, &parent)) } else { Value::Null },
            })
            .collect()
    }

    /// The field that names a row of a tree: its `key` (a tree by key), else the record's id.
    pub fn key_field(&self) -> String {
        self.base.spec.text("rowKey").filter(|k| !k.is_empty()).unwrap_or("id").to_string()
    }

    /// The key that names the row at `place`, as the tree's rows have it.
    pub(super) fn key_at(&self, place: usize) -> Option<Value> {
        self.rows.records.get(place).map(|r| trees::key_of(rows::value_of(r, &self.key_field())))
    }

    /// The field of a record that holds its parent's key, in a tree: its
    /// `parent` (a tree by key), else as the application's facts say, else `parent`.
    pub fn parent_field(&self) -> String {
        if let Some(parent) = self.base.spec.text("parentKey").filter(|p| !p.is_empty()) {
            return parent.to_string();
        }
        let facts = |f: &str| self.base.bound().and_then(|b| self.base.host.field(&rows::column_path(b, f))).and_then(|f| f.parent);
        facts("id").or_else(|| facts("parent")).unwrap_or_else(|| "parent".to_string())
    }

    pub(super) fn tree_shown(&self) -> Vec<TreeShown> {
        trees::shown_tree(&self.tree_rows(), &self.opened)
    }

    /// Where a row of a tree is: how deep, and whether rows are under it.
    pub fn tree_place(&self, place: usize) -> Option<TreeShown> {
        self.tree_shown().into_iter().find(|s| s.0 == place)
    }

    /// Whether a row of a tree shows the rows under it.
    pub fn is_open(&self, place: usize) -> bool {
        self.key_at(place).is_some_and(|id| !id.is_null() && self.opened.iter().any(|o| rows::same(o, &id)))
    }

    /// Shows or hides the rows under a row of a tree.
    pub fn toggle(&mut self, place: usize) {
        let Some(id) = self.key_at(place).filter(|id| !id.is_null()) else { return };
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

    /// Expand all and Collapse all above a tree's rows, as words that read as buttons, with a line under the bar.
    pub(super) fn draw_fold_bar(&self, p: &Painter) {
        let look = p.look;
        let bar = self.fold_bar();
        p.fill(Rect { y: bar.bottom() - look.border(), height: look.border(), ..bar }, look.color("border"), 0.0);
        for (expand, key, english) in [(true, "expandAll", "Expand all"), (false, "collapseAll", "Collapse all")] {
            let Some(b) = self.fold_box(expand) else { continue };
            p.text(&locale::translate(key, english), b, look.color("accent"), look.size("text"), "text", false, TextAlign::Center);
        }
    }
}
