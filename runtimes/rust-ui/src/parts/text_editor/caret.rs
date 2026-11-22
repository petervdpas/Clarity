//! The caret and the selection: moving by character, word and line, going
//! sideways and up and down, and a place under the pointer.

use super::*;

impl TextEditor {
    /// Moves the caret to a point from its top left, selecting nothing: where a row dropped on it goes in.
    pub fn caret_to(&mut self, x: f64, y: f64, cx: &mut Cx) {
        let place = self.place_at(x, y, cx.fonts(), cx.look());
        self.select(place, place);
        self.moved(cx);
    }

    /// Puts Markdown's `mark` on both sides of the selection, as one step,
    /// and keeps the text inside selected (the caret between them with none).
    pub(super) fn mark(&mut self, mark: &str, cx: &mut Cx) {
        let (start, _) = self.selection();
        let inner = self.selected_text();
        self.insert(&format!("{mark}{inner}{mark}"), cx);
        let from = start + mark.chars().count();
        self.select(from, from + inner.chars().count());
        self.moved(cx);
    }

    pub(super) fn move_to(&mut self, place: usize, extend: bool, cx: &mut Cx) {
        self.typing = false;
        self.caret = place.min(self.chars.len());
        if !extend {
            self.anchor = self.caret;
        }
        self.moved(cx);
    }

    pub(super) fn before(&self, at: usize) -> usize {
        at.saturating_sub(1)
    }

    pub(super) fn after(&self, at: usize) -> usize {
        (at + 1).min(self.chars.len())
    }

    /// The start of the word before a place: past spaces, then past the word.
    pub(super) fn word_before(&self, mut at: usize) -> usize {
        while at > 0 && !word_char(self.chars[at - 1]) {
            at -= 1;
        }
        while at > 0 && word_char(self.chars[at - 1]) {
            at -= 1;
        }
        at
    }

    pub(super) fn word_after(&self, mut at: usize) -> usize {
        let len = self.chars.len();
        while at < len && !word_char(self.chars[at]) {
            at += 1;
        }
        while at < len && word_char(self.chars[at]) {
            at += 1;
        }
        at
    }

    /// The word at a place, or the one character there when it is no word (a space, a sign).
    pub(super) fn word_at(&self, at: usize) -> (usize, usize) {
        if self.chars.is_empty() {
            return (0, 0);
        }
        let mut i = at.min(self.chars.len() - 1);
        if !word_char(self.chars[i]) && i > 0 && word_char(self.chars[i - 1]) {
            i -= 1;
        }
        if !word_char(self.chars[i]) {
            return (i, self.after(i));
        }
        let mut start = i;
        while start > 0 && word_char(self.chars[start - 1]) {
            start -= 1;
        }
        let mut end = i;
        while end < self.chars.len() && word_char(self.chars[end]) {
            end += 1;
        }
        (start, end)
    }

    /// Where Left or Right takes the caret. In text of one direction a
    /// character back or on. Where Hebrew or Arabic is on the caret's line,
    /// the place next to it as the line shows, so Left goes left in text
    /// that runs right to left; a word at a time goes the paragraph's way.
    pub(super) fn sideways(&self, to_left: bool, command: bool, shift: bool, cx: &Cx) -> usize {
        let (start, end) = self.selection();
        let laid = self.layout(cx.fonts(), cx.look());
        let n = line_of(&laid, self.caret);
        let shape = &laid.shapes[n];
        if !shape.has_right_to_left() {
            return if to_left {
                if command {
                    self.word_before(self.caret)
                } else if self.caret > 0 && self.anchor != self.caret && !shift {
                    start
                } else {
                    self.before(self.caret)
                }
            } else if command {
                self.word_after(self.caret)
            } else if self.anchor != self.caret && !shift {
                end
            } else {
                self.after(self.caret)
            };
        }
        if command {
            return if to_left != shape.right_to_left() { self.word_before(self.caret) } else { self.word_after(self.caret) };
        }
        if self.anchor != self.caret && !shift {
            // The selection gives way at its end on that side.
            return if (laid.across[start] < laid.across[end]) == to_left { start } else { end };
        }
        let line = laid.lines[n];
        let step = shape.step(self.caret - line.start, !to_left);
        if step != self.caret - line.start {
            return line.start + step;
        }
        // At the line's edge: on to the next line from its end, back to the line before from its start.
        let onward = to_left == shape.right_to_left();
        if onward && self.caret == line.end {
            self.after(self.caret)
        } else if !onward && self.caret == line.start {
            self.before(self.caret)
        } else {
            self.caret
        }
    }

    /// Selects from `from` to `to`, the caret at `to`.
    pub fn select(&mut self, from: usize, to: usize) {
        self.anchor = from.min(self.chars.len());
        self.caret = to.min(self.chars.len());
        self.follow = true;
    }

    pub fn select_all(&mut self) {
        self.select(0, self.chars.len());
    }

    pub(super) fn line_start(&self, place: usize, cx: &Cx) -> usize {
        let laid = self.layout(cx.fonts(), cx.look());
        laid.lines[line_of(&laid, place)].start
    }

    pub(super) fn line_end(&self, place: usize, cx: &Cx) -> usize {
        let laid = self.layout(cx.fonts(), cx.look());
        laid.lines[line_of(&laid, place)].end
    }

    /// The place a line up or down, as near across as the caret is.
    pub(super) fn vertically(&self, by: i64, cx: &Cx) -> usize {
        let laid = self.layout(cx.fonts(), cx.look());
        let to = line_of(&laid, self.caret) as i64 + by;
        if to < 0 {
            return 0;
        }
        if to as usize >= laid.lines.len() {
            return self.chars.len();
        }
        nearest(&laid, to as usize, laid.across[self.caret])
    }

    /// The place nearest a point, from its top left.
    pub fn place_at(&self, x: f64, y: f64, fonts: &Fonts, look: &ThemeLook) -> usize {
        let laid = self.layout(fonts, look);
        let inner = self.inner(look);
        let b = self.node.bounds;
        let line = if self.multiline {
            (((y - (inner.y - b.y) + self.scroll_y) / laid.height).floor().max(0.0) as usize).min(laid.lines.len() - 1)
        } else {
            0
        };
        let across = x - (inner.x - b.x) + self.scroll_x - self.align_shift(&laid, line, inner);
        nearest(&laid, line, across)
    }
}
