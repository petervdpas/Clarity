//! Editing text: typing and deleting, the keys, the clipboard and the menu.

use super::*;

impl TextEditor {
    /// Puts `typed` where the selection is.
    pub(super) fn insert(&mut self, typed: &str, cx: &mut Cx) {
        self.insert_typed(typed, false, cx);
    }

    /// Puts `typed` where the selection is; `typed_now` when the person typed it (not pasted), so it joins the step before.
    pub(super) fn insert_typed(&mut self, typed: &str, typed_now: bool, cx: &mut Cx) {
        let mut clean = self.clean(typed);
        let (start, end) = self.selection();
        if let Some(most) = self.max_length {
            let room = most.saturating_sub(self.chars.len() - (end - start));
            clean.truncate(room);
        }
        if clean.is_empty() && start == end {
            return;
        }
        self.remember(typed_now && start == end, &clean);
        let count = clean.len();
        self.chars.splice(start..end, clean);
        self.caret = start + count;
        self.anchor = self.caret;
        self.changed(cx);
    }

    pub(super) fn delete_selection(&mut self, cx: &mut Cx) {
        self.remember(false, &[]);
        let (start, end) = self.selection();
        self.chars.drain(start..end);
        self.caret = start;
        self.anchor = start;
        self.changed(cx);
    }

    /// Puts `text` where the selection is (at the caret with none), as one step Ctrl+Z takes back: what code inserts.
    pub fn put(&mut self, text: &str, cx: &mut Cx) {
        if self.editable() {
            self.insert(text, cx);
        }
    }

    /// A key: true when it took it.
    pub fn key(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        if let Some(first) = self.key_first.as_mut() {
            if first(key) {
                return true;
            }
        }
        let command = key.control() || key.meta();
        let shift = key.shift();
        let alt = key.modifiers.has(Modifiers::ALT);
        let len = self.chars.len();
        match key.key {
            Key::Left => {
                let to = self.sideways(true, command, shift, cx);
                self.move_to(to, shift, cx);
            }
            Key::Right => {
                let to = self.sideways(false, command, shift, cx);
                self.move_to(to, shift, cx);
            }
            Key::Home => {
                let to = if command || !self.multiline { 0 } else { self.line_start(self.caret, cx) };
                self.move_to(to, shift, cx);
            }
            Key::End => {
                let to = if command || !self.multiline { len } else { self.line_end(self.caret, cx) };
                self.move_to(to, shift, cx);
            }
            Key::Up if self.multiline => {
                let to = self.vertically(-1, cx);
                self.move_to(to, shift, cx);
            }
            Key::Down if self.multiline => {
                let to = self.vertically(1, cx);
                self.move_to(to, shift, cx);
            }
            Key::Backspace => {
                if !self.editable() {
                    return true;
                }
                if self.caret == self.anchor && self.caret > 0 {
                    self.anchor = if command { self.word_before(self.caret) } else { self.before(self.caret) };
                }
                if self.caret != self.anchor {
                    self.delete_selection(cx);
                }
            }
            Key::Delete => {
                if !self.editable() {
                    return true;
                }
                if self.caret == self.anchor && self.caret < len {
                    self.anchor = if command { self.word_after(self.caret) } else { self.after(self.caret) };
                }
                if self.caret != self.anchor {
                    self.delete_selection(cx);
                }
            }
            Key::Enter if self.multiline => {
                if self.editable() {
                    self.insert("\n", cx);
                }
            }
            Key::Enter => {
                return match self.entered.as_mut() {
                    Some(entered) => entered(),
                    None => false,
                };
            }
            Key::A if command => {
                self.select_all();
                self.moved(cx);
            }
            Key::B if command && self.markdown && self.editable() => self.mark("**", cx),
            Key::I if command && self.markdown && self.editable() => self.mark("*", cx),
            Key::C if command => self.copy(cx),
            Key::X if command => self.cut(cx),
            Key::V if command => self.paste(cx),
            // Its own typing first; with none left, the key goes on to the window's edits.
            Key::Z if command && !alt => return self.undoes && self.undo_typing(!shift, cx),
            Key::Y if command && !alt => return self.undoes && self.undo_typing(false, cx),
            _ => return false,
        }
        true
    }

    /// Text typed: it goes in where the selection is.
    pub fn typed(&mut self, typed: &str, cx: &mut Cx) {
        if !self.editable() {
            return;
        }
        // Keys that type no character (Backspace, Escape) come as keys.
        let printable: String = typed.chars().filter(|c| !c.is_control() || *c == '\t' || (self.multiline && *c == '\n')).collect();
        if !printable.is_empty() {
            self.insert_typed(&printable, true, cx);
        }
    }

    /// What a memo's toolbox asked: cut, copy or paste, as its keys do.
    pub fn clipboard_action(&mut self, ask: crate::parts::data::canvas::toolbox::TextAsk, cx: &mut Cx) {
        use crate::parts::data::canvas::toolbox::TextAsk;
        match ask {
            TextAsk::Cut => self.cut(cx),
            TextAsk::Copy => self.copy(cx),
            TextAsk::Paste => self.paste(cx),
        }
    }

    pub(super) fn copy(&mut self, cx: &mut Cx) {
        if self.caret != self.anchor && !self.password {
            let selected = self.selected_text();
            cx.clipboard().set_text(&selected);
        }
    }

    pub(super) fn cut(&mut self, cx: &mut Cx) {
        if self.caret != self.anchor && !self.password && self.editable() {
            let selected = self.selected_text();
            cx.clipboard().set_text(&selected);
            self.delete_selection(cx);
        }
    }

    pub(super) fn paste(&mut self, cx: &mut Cx) {
        if self.editable() {
            let pasted = cx.clipboard().get_text().unwrap_or_default();
            if !pasted.is_empty() {
                self.insert(&pasted, cx);
            }
        }
    }

    /// Does what its right-click menu asked for: Cut, Copy, Paste or Select all.
    pub fn run_menu_asked(&mut self, cx: &mut Cx) {
        match self.menu_asked.take() {
            Some(MenuAsk::Cut) => self.cut(cx),
            Some(MenuAsk::Copy) => self.copy(cx),
            Some(MenuAsk::Paste) => self.paste(cx),
            Some(MenuAsk::SelectAll) => {
                self.select_all();
                self.moved(cx);
            }
            None => {}
        }
    }

    /// A right press: Cut, Copy, Paste and Select all, in the person's
    /// language, each usable when it can do something. What is picked is
    /// done when `wake` (the editor, or the part that holds it) wakes,
    /// which the menu asks for; it then calls `run_menu_asked`.
    pub fn menu(&self, wake: crate::element::ElementId) -> Option<Vec<MenuEntry>> {
        if !self.enabled || !self.takes_focus {
            return None;
        }
        let selected = self.caret != self.anchor && !self.password;
        let entry = |caption: String, enabled: bool, ask: MenuAsk| {
            let asked = self.menu_asked.clone();
            MenuEntry {
                caption,
                enabled,
                run: Some(Rc::new(move || asked.set(Some(ask)))),
                wake: Some(wake),
                ..MenuEntry::default()
            }
        };
        Some(vec![
            entry(crate::locale::translate("cutText", "Cut"), selected && self.editable(), MenuAsk::Cut),
            entry(crate::locale::translate("copyText", "Copy"), selected, MenuAsk::Copy),
            entry(crate::locale::translate("pasteText", "Paste"), self.editable(), MenuAsk::Paste),
            entry(crate::locale::translate("selectAllText", "Select all"), !self.chars.is_empty(), MenuAsk::SelectAll),
        ])
    }
}
