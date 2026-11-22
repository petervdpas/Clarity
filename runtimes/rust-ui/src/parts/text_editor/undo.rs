//! Undoing typing: what was typed is remembered and taken back, and whether
//! the text changed since.

use super::*;

impl TextEditor {
    /// Whether it has typing of its own to take back (true) or to do again (false).
    pub fn can_undo(&self, back: bool) -> bool {
        if back { !self.undo.is_empty() } else { !self.redo.is_empty() }
    }

    /// The text is taken or set: its typing is no longer its own to take back.
    pub fn forget_typing(&mut self) {
        self.undo.clear();
        self.redo.clear();
        self.typing = false;
    }

    /// Keeps the text as it is now as a step to take back; typed characters in a row are one step, a word at a time.
    pub(super) fn remember(&mut self, typed: bool, added: &[char]) {
        let joins = typed && self.typing && !self.undo.is_empty() && !added.iter().any(|c| c.is_whitespace());
        if !joins {
            self.undo.push(Typed { chars: self.chars.clone(), caret: self.caret, anchor: self.anchor });
        }
        self.redo.clear();
        self.typing = typed;
    }

    /// Takes back (`back`) or does again one step of its own typing; false when it has none.
    pub fn undo_typing(&mut self, back: bool, cx: &mut Cx) -> bool {
        let step = if back { self.undo.pop() } else { self.redo.pop() };
        let Some(step) = step else {
            return false;
        };
        let now = Typed { chars: self.chars.clone(), caret: self.caret, anchor: self.anchor };
        if back { self.redo.push(now) } else { self.undo.push(now) }
        self.chars = step.chars;
        self.caret = step.caret.min(self.chars.len());
        self.anchor = step.anchor.min(self.chars.len());
        self.typing = false;
        self.changed(cx);
        true
    }

    /// Text as it can stand here: one line has no line breaks, several lines have only `\n`.
    pub(super) fn clean(&self, value: &str) -> Vec<char> {
        let value = value.replace("\r\n", "\n").replace('\r', "\n");
        value.chars().map(|c| if c == '\n' && !self.multiline { ' ' } else { c }).collect()
    }

    pub(super) fn moved(&mut self, cx: &mut Cx) {
        self.follow = true;
        self.scroll_to_caret(cx.fonts(), cx.look());
        cx.invalidate();
    }

    pub(super) fn changed(&mut self, cx: &mut Cx) {
        self.version += 1;
        self.edits += 1;
        self.moved(cx);
    }
}
