//! A screen's tooltips: what a part says where the pointer rests, its place,
//! and hiding it.

use super::*;

impl Screen {
    /// A tooltip in Clarity's look: the tooltip part's colors, else a panel with a strong border.
    pub(super) fn draw_tip(p: &Painter, tip: &Tooltip) {
        let look = p.look.part("tooltip", None);
        let radius = look.radius.map(|r| r as f64).unwrap_or_else(|| p.look.radius("input"));
        p.fill(tip.place, look.background.unwrap_or_else(|| p.look.color("panel")), radius);
        p.stroke(tip.place, look.border.unwrap_or_else(|| p.look.color("strong_border")), p.look.border(), radius);
        p.text(&tip.text, tip.place.inset(TIP_PAD), look.text.unwrap_or_else(|| p.look.color("text")), p.look.size("small"), "text", false, TextAlign::Left);
    }

    /// What the element at a point, or one it is in, says about that place.
    pub(super) fn tip_of(&self, at: Option<ElementId>, x: f64, y: f64) -> Option<String> {
        let at = at?;
        let tree = self.overlays.iter().rev().map(|o| o.content.as_ref()).find(|c| element::find(*c, at).is_some()).unwrap_or(self.root.as_ref());
        let ids = element::chain(tree, at)?;
        ids.iter().rev().filter_map(|id| element::find(tree, *id)).find_map(|e| e.tooltip_at(x, y).or_else(|| Self::designed_tip(e)).filter(|t| !t.is_empty()))
    }

    /// The tooltip a part is designed with (`tooltip "@saveTooltip"`), in the person's language; none for none.
    pub(super) fn designed_tip(e: &dyn Element) -> Option<String> {
        let base = e.as_part()?.base();
        let written = base.spec.text("tooltip").filter(|t| !t.is_empty())?;
        Some(base.host.text(Some(written)))
    }

    /// The pointer rests on `text` from now (none: on nothing that says something).
    pub(super) fn rest(&mut self, text: Option<String>, x: f64, y: f64) {
        if text == self.tip_waiting {
            return;
        }
        self.hide_tip();
        self.tip_point = (x, y);
        if text.is_some() {
            self.tip_due = Some((self.now)() + TIP_PAUSE);
            self.wakes_changed = true;
        }
        self.tip_waiting = text;
    }

    /// The tooltip goes, and none waits.
    pub(super) fn hide_tip(&mut self) {
        self.tip_waiting = None;
        self.tip_due = None;
        if self.tip.take().is_some() {
            self.redraw = true;
        }
    }

    /// Where a tooltip shows: under the pointer, inside the screen.
    pub(super) fn tip_place(&self, text: &str) -> Rect {
        let size = self.look.size("small");
        let width = self.fonts.measure(text, "text", size, false) + 2.0 * TIP_PAD;
        let height = size + 2.0 * TIP_PAD + 2.0;
        let (px, py) = self.tip_point;
        let x = px.min(self.width - width).max(0.0);
        let y = if py + 20.0 + height > self.height { py - 8.0 - height } else { py + 20.0 };
        Rect::new(x, y.max(0.0), width, height)
    }
}
