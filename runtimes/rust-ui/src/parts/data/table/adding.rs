//! Adding and removing a table's rows: the add strip, its button, and removing
//! the chosen row, asked first where the table confirms.

use super::*;

impl TablePart {
    /// Whether it has an Add button above its header (`add`) and a trash button on every row (`remove`).
    pub fn adds(&self) -> bool {
        self.base.spec.flag("add")
    }

    pub fn removes(&self) -> bool {
        self.base.spec.flag("remove")
    }

    /// Whether removing a row asks first (`remove confirm`).
    pub fn confirms(&self) -> bool {
        self.base.spec.flag("confirm")
    }

    /// The chosen row is removed: its `on remove` runs, after the person said yes where the table asks first.
    pub(super) fn remove_chosen(&mut self, cx: &mut Cx) {
        let Some(place) = self.rows.chosen(&self.base) else { return };
        if !self.confirms() {
            self.base.fire("remove");
            return;
        }
        use clarity_runtime::ui::{Ask, AskForm, Slot, Text};
        let row = self.columns.first().map(|c| self.shown(place, &c.field)).unwrap_or_default();
        let word = |key: &str, english: &str| Text { text: english.to_string(), values: Default::default(), key: Some(key.to_string()) };
        let mut question = word("removeRowAsk", "Remove {row}?");
        question.values.insert("row".to_string(), Slot::text(row));
        let ask = Ask { question, form: AskForm::Confirm { yes: Some(word("removeRow", "Remove")), no: Some(word("keepRow", "Keep")) }, handler: String::new(), values: Vec::new() };
        let dialog = crate::dialogs::ask::AskDialog::new(ask, None);
        let first = dialog.first();
        let answered = dialog.answered();
        let (said, me) = (self.remove_said.clone(), self.base.node.id());
        let mut overlay = Overlay { opener: Some(me), ..Overlay::dialog(Box::new(dialog)) };
        overlay.closed = Some(Box::new(move || said.set(answered.borrow().is_some())));
        cx.open(overlay);
        cx.focus(Some(first));
    }

    /// The strip above it where its Add button stands; none high without one.
    pub(super) fn add_strip(&self) -> Rect {
        let b = self.base.node.bounds;
        let height = if self.adds() { (crate::grid::row_height() + 4.0).min(b.height) } else { 0.0 };
        Rect::new(b.x, b.y, b.width, height)
    }

    /// Its Add button, at the right above its header; none without `add`.
    pub fn add_button(&self) -> Option<Rect> {
        if !self.adds() {
            return None;
        }
        let strip = self.add_strip();
        let width = (self.add_text.ceil() + OWN_ICON + 4.0 * self.look.padding + 6.0).min(strip.width);
        Some(Rect::new(strip.right() - width, strip.y, width, (strip.height - 4.0).max(0.0)))
    }

    /// The trash button of the row of the record at `place`, in the last column, while the row is in sight; none without `remove`.
    pub fn remove_box(&self, place: usize) -> Option<Rect> {
        if !self.removes() {
            return None;
        }
        let row = self.row_box(place)?;
        let end = self.lefts(row.x).last().map(|(x, w)| x + w).unwrap_or(row.x);
        Some(Rect::new(end, row.y, REMOVE_WIDTH, row.height))
    }

    /// Its Add button: a button's look, the plus and Clarity's word.
    pub(super) fn draw_add(&self, p: &Painter, button: Rect) {
        let look = p.look;
        let hover = self.base.node.hovered && self.rows.pointer.is_some_and(|(x, y)| button.contains(x, y)) && !self.locked();
        let values = look.part("button", if hover { Some("hover") } else { None });
        let radius = values.radius.map(|r| r as f64).unwrap_or_else(|| look.radius("button"));
        let _dim = p.dimmed(!self.locked());
        p.fill(button, values.background.unwrap_or_else(|| look.color("raised")), radius);
        p.stroke(button, values.border.unwrap_or_else(|| look.color("strong_border")), look.border(), radius);
        let ink = values.text.unwrap_or_else(|| look.color("text"));
        let pad = 2.0 * look.padding();
        let icon = Rect::new(button.x + pad, button.y + (button.height - OWN_ICON) / 2.0, OWN_ICON, OWN_ICON);
        crate::parts::containers::icon::draw_icon(p, self.base.host.icon_path("plus").as_deref(), icon, ink);
        let words = Rect::new(icon.right() + 6.0, button.y, (button.right() - icon.right() - 6.0 - pad).max(0.0), button.height);
        p.text(&add_word(), words, ink, look.size("text"), "text", false, TextAlign::Left);
    }
}
