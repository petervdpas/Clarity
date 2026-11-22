//! The pointer in a text editor: pressing, dragging, releasing, the wheel, and
//! the focus moving.

use super::*;

impl TextEditor {
    /// The main button went down at a point from its top left.
    pub fn pressed(&mut self, at: PointerInput, cx: &mut Cx) {
        self.grab = None;
        // On its scroll bar, in its right margin: the thumb drags, a press beside it pages.
        if let Some((_, thumb)) = self.bar(cx.fonts(), cx.look()) {
            let inner = self.inner(cx.look());
            if at.x >= self.node.bounds.width - self.side_gap(cx.look()) - 4.0 {
                match thumb.pressed(at.y - (inner.y - self.node.bounds.y), inner.height) {
                    Ok(grab) => self.grab = Some(grab),
                    Err(to) => {
                        self.scroll_y = to;
                        self.follow = false;
                        cx.invalidate();
                    }
                }
                return;
            }
        }
        if !self.enabled {
            return;
        }
        let place = self.place_at(at.x, at.y, cx.fonts(), cx.look());
        if at.clicks >= 3 {
            self.select_all();
            self.moved(cx);
            return;
        }
        if at.clicks == 2 {
            let (start, end) = self.word_at(place);
            self.select(start, end);
            self.moved(cx);
            return;
        }
        self.dragging = true;
        self.move_to(place, at.modifiers.has(Modifiers::SHIFT), cx);
    }

    /// The pointer moved; while the button is down it selects.
    pub fn dragged(&mut self, at: PointerInput, cx: &mut Cx) {
        if let (Some(grab), true) = (self.grab, self.node.pressed) {
            if let Some((_, thumb)) = self.bar(cx.fonts(), cx.look()) {
                let inner = self.inner(cx.look());
                self.scroll_y = thumb.dragged(at.y - (inner.y - self.node.bounds.y), grab);
                self.follow = false;
                cx.invalidate();
            }
            return;
        }
        if self.dragging && self.node.pressed {
            let place = self.place_at(at.x, at.y, cx.fonts(), cx.look());
            self.move_to(place, true, cx);
        }
    }

    pub fn released(&mut self) {
        self.grab = None;
        self.dragging = false;
    }

    /// The wheel scrolls lines that do not fit; a positive turn shows what is further down. True when it scrolled.
    pub fn wheel(&mut self, dy: f64, cx: &mut Cx) -> bool {
        if !self.multiline {
            return false;
        }
        let laid = self.layout(cx.fonts(), cx.look());
        let most = crate::geometry::scroll_room(laid.lines.len() as f64 * laid.height, self.inner(cx.look()).height);
        if most <= 0.0 {
            return false;
        }
        let was = self.scroll_y;
        self.scroll_y = (self.scroll_y + dy).clamp(0.0, most);
        self.follow = false;
        cx.invalidate();
        self.scroll_y != was
    }

    /// It got or lost the focus.
    pub fn focus_moved(&mut self, focused: bool, cx: &mut Cx) {
        self.node.focused = focused;
        self.dragging = false;
        if !focused {
            self.composing = None;
            // Left, its text is taken: from then on Ctrl+Z takes back the window's edit of it.
            self.forget_typing();
        }
        cx.invalidate();
    }
}
