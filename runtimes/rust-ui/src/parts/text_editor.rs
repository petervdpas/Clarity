//! A place to type text: one line, or several lines that wrap (a memo). It
//! has a caret and a selection, takes typed text, the keys that move and
//! delete, copying, cutting and pasting through the screen's clipboard, and
//! the pointer: a press places the caret, a drag selects, a double click
//! selects a word. A long line scrolls sideways; wrapped lines scroll up and
//! down. Empty, it shows its placeholder; a password shows dots and is never
//! copied out.
//!
//! It is an element of its own, and the parts that edit text (text box,
//! memo, password, number and spin box) hold one and hand it what the
//! person does. Places in the text are counted in characters (Unicode
//! scalar values), never in bytes.
//!
//! Its right-click menu has Cut, Copy, Paste and Select all. Ctrl+Z takes
//! back its own typing first, while it has the focus; once its text is
//! taken (it is left, or set), Ctrl+Z goes on to the window's edits. Text is
//! shaped as it is drawn (`Fonts::shape`): Arabic joined, Hebrew right to
//! left, and the arrow keys move the way they point where directions mix.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use skia_safe::{Color, Paint};

use crate::element::{Cx, Element, Node};
use crate::element_node;
use crate::fonts::{Fonts, ShapedText};
use crate::frame::menus::MenuEntry;
use crate::geometry::Rect;
use crate::input::{Cursor, Key, KeyInput, PointerInput, Modifiers};
use crate::painter::{Painter, TextAlign};
use crate::theme::ThemeLook;

/// What a password shows for each character.
const DOT: char = '*';

/// What the right-click menu asked for; it is done when the editor wakes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MenuAsk {
    Cut,
    Copy,
    Paste,
    SelectAll,
}

/// The text, caret and selection as they were before one step of typing.
#[derive(Debug, Clone, PartialEq)]
struct Typed {
    chars: Vec<char>,
    caret: usize,
    anchor: usize,
}

/// One line as drawn: where it starts and ends in the text, without the line break that ends it.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Line {
    start: usize,
    end: usize,
}

/// The lines, how high each is, and where each place of the text is across its line.
#[derive(Clone)]
struct Laid {
    lines: Vec<Line>,
    /// Each line shaped as it is drawn, so the caret and the pointer land on its letters.
    shapes: Vec<Rc<ShapedText>>,
    height: f64,
    across: Vec<f64>,
    // What it was laid out for: the width inside, the text size, the font and the text's version.
    width: f64,
    size: f64,
    font: String,
    version: u64,
}

pub struct TextEditor {
    node: Node,
    chars: Vec<char>,
    caret: usize,
    anchor: usize,
    dragging: bool,
    follow: bool,
    multiline: bool,
    password: bool,
    /// Shown while it is empty, in the muted color.
    pub placeholder: Option<String>,
    /// Whether the person may use it at all; disabled it takes no focus and draws faded.
    pub enabled: bool,
    /// Read-only: the caret moves and it copies, but nothing is typed.
    pub read_only: bool,
    /// Marked as holding something wrong: its border is the error color.
    pub marked: bool,
    /// How wide the place inside its right edge is that its holder draws in (a password's buttons).
    pub right_inset: f64,
    pub align: TextAlign,
    /// Whether it draws the input's frame; without one it is text to read.
    pub framed: bool,
    /// Whether a press or Tab gives it the focus.
    pub takes_focus: bool,
    /// Whether Tab comes to it (to `key_first`) instead of moving the focus on.
    pub takes_tab: bool,
    /// Enter in one line: true when the holder took it.
    pub entered: Option<Box<dyn FnMut() -> bool>>,
    /// A key, before it does what it does here: true when the holder took it.
    pub key_first: Option<Box<dyn FnMut(KeyInput) -> bool>>,
    /// How many characters the person may type at most; none: any number. Text set from elsewhere is not cut.
    pub max_length: Option<usize>,
    /// Whether Ctrl+Z takes back its typing: not in a part with `no undo`, whose holder sets it false.
    pub undoes: bool,
    /// The theme's font it writes in: `text`, or `mono` for code.
    pub font_role: &'static str,
    /// The language whose words it colors (`code csharp`); none for plain text.
    pub language: Option<&'static crate::parts::code_colors::Language>,
    /// The colored stretches of the text, for the version they were found in.
    inked: RefCell<Option<(u64, Vec<(usize, usize, crate::parts::code_colors::Ink)>)>>,
    undo: Vec<Typed>,
    redo: Vec<Typed>,
    /// Whether the last step was typing characters, which the next typed character joins.
    typing: bool,
    /// What its right-click menu asked for, done when it wakes.
    menu_asked: Rc<Cell<Option<MenuAsk>>>,
    edits: u64,
    version: u64,
    scroll_x: f64,
    scroll_y: f64,
    laid: RefCell<Option<Laid>>,
    /// What an input method composes at the caret, not typed yet.
    composing: Option<crate::element::Composition>,
    /// Where the caret was last drawn, on the screen: an input method shows its window by it.
    caret_drawn: std::cell::Cell<Option<Rect>>,
}

impl Default for TextEditor {
    fn default() -> Self {
        TextEditor::new(false)
    }
}

/// Whether a character belongs to a word: a letter, a digit or `_`.
fn word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

impl TextEditor {
    /// An empty editor of one line, or of several lines that wrap.
    pub fn new(multiline: bool) -> TextEditor {
        TextEditor {
            node: Node::new(),
            chars: Vec::new(),
            caret: 0,
            anchor: 0,
            dragging: false,
            follow: false,
            multiline,
            password: false,
            placeholder: None,
            enabled: true,
            read_only: false,
            marked: false,
            right_inset: 0.0,
            align: TextAlign::Left,
            framed: true,
            takes_focus: true,
            takes_tab: false,
            entered: None,
            key_first: None,
            max_length: None,
            undoes: true,
            font_role: "text",
            language: None,
            inked: RefCell::new(None),
            undo: Vec::new(),
            redo: Vec::new(),
            typing: false,
            menu_asked: Rc::new(Cell::new(None)),
            edits: 0,
            version: 0,
            scroll_x: 0.0,
            scroll_y: 0.0,
            laid: RefCell::new(None),
            composing: None,
            caret_drawn: std::cell::Cell::new(None),
        }
    }

    /// An input method composes `composing` at the caret (none: it stopped);
    /// what it commits comes as typed text. Read-only it composes nothing.
    pub fn compose(&mut self, composing: Option<crate::element::Composition>, cx: &mut Cx) {
        self.composing = composing.filter(|c| !c.text.is_empty() && !self.read_only && self.enabled);
        cx.invalidate();
    }

    /// What an input method composes now; none while it composes nothing.
    pub fn composing(&self) -> Option<&crate::element::Composition> {
        self.composing.as_ref()
    }

    /// Where the caret was last drawn, while it has the focus.
    pub fn caret_area(&self) -> Option<Rect> {
        if self.node.focused { self.caret_drawn.get() } else { None }
    }

    /// Whether it holds several lines, which wrap; Enter is then a new line.
    pub fn is_multiline(&self) -> bool {
        self.multiline
    }

    /// The text it holds.
    pub fn text(&self) -> String {
        self.chars.iter().collect()
    }

    /// Sets the text; no edit. The caret goes to its end.
    pub fn set_text(&mut self, value: &str) {
        let clean = self.clean(value);
        if clean == self.chars {
            return;
        }
        self.chars = clean;
        self.caret = self.chars.len();
        self.anchor = self.caret;
        self.version += 1;
        // Text set from elsewhere is no typing to take back.
        self.forget_typing();
        // With the focus the caret shows at its end; else it shows from its start.
        self.follow = self.node.focused;
        if !self.node.focused {
            self.scroll_x = 0.0;
            self.scroll_y = 0.0;
        }
    }

    /// Where the caret is, as a place in the text in characters (0 is before the first).
    pub fn caret(&self) -> usize {
        self.caret
    }

    /// The selection, from its start to its end; empty when the two are the same.
    pub fn selection(&self) -> (usize, usize) {
        (self.caret.min(self.anchor), self.caret.max(self.anchor))
    }

    pub fn selected_text(&self) -> String {
        let (start, end) = self.selection();
        self.chars[start..end].iter().collect()
    }

    /// How many times the person changed the text: typed, deleted, cut or pasted.
    pub fn edits(&self) -> u64 {
        self.edits
    }

    /// What is drawn for the text: the text itself, or a `*` for each character of a password.
    pub fn shown(&self) -> String {
        self.shown_chars().into_iter().collect()
    }

    /// A password: each character shows as a `*`, and it is not copied or cut.
    pub fn password(&self) -> bool {
        self.password
    }

    pub fn set_password(&mut self, password: bool) {
        if self.password != password {
            self.password = password;
            self.version += 1;
        }
    }

    /// How far its text is scrolled sideways (one line).
    pub fn scroll_x(&self) -> f64 {
        self.scroll_x
    }

    /// How far its text is scrolled down (several lines).
    pub fn scroll_y(&self) -> f64 {
        self.scroll_y
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

    fn editable(&self) -> bool {
        self.enabled && !self.read_only
    }

    /// Text as it can stand here: one line has no line breaks, several lines have only `\n`.
    fn clean(&self, value: &str) -> Vec<char> {
        let value = value.replace("\r\n", "\n").replace('\r', "\n");
        value.chars().map(|c| if c == '\n' && !self.multiline { ' ' } else { c }).collect()
    }

    fn moved(&mut self, cx: &mut Cx) {
        self.follow = true;
        self.scroll_to_caret(cx.fonts(), cx.look());
        cx.invalidate();
    }

    fn changed(&mut self, cx: &mut Cx) {
        self.version += 1;
        self.edits += 1;
        self.moved(cx);
    }

    // -- Its own undo -------------------------------------------------------

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
    fn remember(&mut self, typed: bool, added: &[char]) {
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

    /// Puts `typed` where the selection is.
    fn insert(&mut self, typed: &str, cx: &mut Cx) {
        self.insert_typed(typed, false, cx);
    }

    /// Puts `typed` where the selection is; `typed_now` when the person typed it (not pasted), so it joins the step before.
    fn insert_typed(&mut self, typed: &str, typed_now: bool, cx: &mut Cx) {
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

    fn delete_selection(&mut self, cx: &mut Cx) {
        self.remember(false, &[]);
        let (start, end) = self.selection();
        self.chars.drain(start..end);
        self.caret = start;
        self.anchor = start;
        self.changed(cx);
    }

    fn move_to(&mut self, place: usize, extend: bool, cx: &mut Cx) {
        self.typing = false;
        self.caret = place.min(self.chars.len());
        if !extend {
            self.anchor = self.caret;
        }
        self.moved(cx);
    }

    fn before(&self, at: usize) -> usize {
        at.saturating_sub(1)
    }

    fn after(&self, at: usize) -> usize {
        (at + 1).min(self.chars.len())
    }

    /// The start of the word before a place: past spaces, then past the word.
    fn word_before(&self, mut at: usize) -> usize {
        while at > 0 && !word_char(self.chars[at - 1]) {
            at -= 1;
        }
        while at > 0 && word_char(self.chars[at - 1]) {
            at -= 1;
        }
        at
    }

    fn word_after(&self, mut at: usize) -> usize {
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
    fn word_at(&self, at: usize) -> (usize, usize) {
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

    // -- Keys ---------------------------------------------------------------

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

    fn copy(&mut self, cx: &mut Cx) {
        if self.caret != self.anchor && !self.password {
            let selected = self.selected_text();
            cx.clipboard().set_text(&selected);
        }
    }

    fn cut(&mut self, cx: &mut Cx) {
        if self.caret != self.anchor && !self.password && self.editable() {
            let selected = self.selected_text();
            cx.clipboard().set_text(&selected);
            self.delete_selection(cx);
        }
    }

    fn paste(&mut self, cx: &mut Cx) {
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

    /// Where Left or Right takes the caret. In text of one direction a
    /// character back or on. Where Hebrew or Arabic is on the caret's line,
    /// the place next to it as the line shows, so Left goes left in text
    /// that runs right to left; a word at a time goes the paragraph's way.
    fn sideways(&self, to_left: bool, command: bool, shift: bool, cx: &Cx) -> usize {
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

    // -- The pointer ----------------------------------------------------------

    /// The main button went down at a point from its top left.
    pub fn pressed(&mut self, at: PointerInput, cx: &mut Cx) {
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
        if self.dragging && self.node.pressed {
            let place = self.place_at(at.x, at.y, cx.fonts(), cx.look());
            self.move_to(place, true, cx);
        }
    }

    pub fn released(&mut self) {
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

    /// Takes its place, and scrolls so the caret shows.
    pub fn place(&mut self, area: Rect, cx: &Cx) {
        self.node.bounds = area;
        self.scroll_to_caret(cx.fonts(), cx.look());
    }

    /// Mirrors the hover, press and focus of the part that holds it.
    pub fn mirror(&mut self, holder: &Node) {
        self.node.hovered = holder.hovered;
        self.node.pressed = holder.pressed;
        self.node.focused = holder.focused;
    }

    // -- Lines, as they are laid out --------------------------------------------

    fn side_gap(&self, look: &ThemeLook) -> f64 {
        if self.framed { look.padding() * 2.0 } else { 0.0 }
    }

    /// The place inside the frame the text is drawn in.
    fn inner(&self, look: &ThemeLook) -> Rect {
        let b = self.node.bounds;
        let top = if self.multiline && self.framed { look.padding() + look.border() } else { 0.0 };
        let side = self.side_gap(look);
        Rect::new(b.x + side, b.y + top, (b.width - 2.0 * side - self.right_inset).max(1.0), (b.height - 2.0 * top).max(1.0))
    }

    /// What is drawn for the text: the text itself, or a dot for each character of a password.
    fn shown_chars(&self) -> Vec<char> {
        if self.password { vec![DOT; self.chars.len()] } else { self.chars.clone() }
    }

    fn layout(&self, fonts: &Fonts, look: &ThemeLook) -> Laid {
        let width = self.inner(look).width;
        let size = look.size("text");
        let font_name = look.font(self.font_role);
        if let Some(laid) = self.laid.borrow().as_ref() {
            if (laid.width - width).abs() < 0.5 && laid.size == size && laid.font == font_name && laid.version == self.version {
                return laid.clone();
            }
        }
        let font = fonts.font(self.font_role, size, false);
        let shown = self.shown_chars();
        let text: String = shown.iter().collect();
        // The text shaped as drawn (Arabic joined, Hebrew right to left): each character's width, in the text's order.
        let whole = fonts.shape(&text, self.font_role, size, false);
        let mut from = vec![0.0; shown.len() + 1];
        for (i, c) in shown.iter().enumerate() {
            let w = if *c == '\n' || i >= whole.len() { 0.0 } else { whole.right_of(i) - whole.left_of(i) };
            from[i + 1] = from[i] + w;
        }
        let lines = if self.multiline { wrap(&shown, &from, width) } else { vec![Line { start: 0, end: shown.len() }] };
        // Each line shaped as it is drawn, so the caret and the pointer land on its letters.
        let shapes: Vec<Rc<ShapedText>> = lines
            .iter()
            .map(|l| {
                if l.start == 0 && l.end == shown.len() {
                    whole.clone()
                } else {
                    fonts.shape(&shown[l.start..l.end].iter().collect::<String>(), self.font_role, size, false)
                }
            })
            .collect();
        // Across is measured from the start of each place's own line.
        let mut across = vec![0.0; shown.len() + 1];
        for (n, line) in lines.iter().enumerate() {
            for i in line.start..=line.end {
                across[i] = shapes[n].caret_x(i - line.start);
            }
        }
        let (spacing, _) = font.metrics();
        let laid = Laid { lines, shapes, height: spacing as f64, across, width, size, font: font_name, version: self.version };
        *self.laid.borrow_mut() = Some(laid.clone());
        laid
    }

    fn line_start(&self, place: usize, cx: &Cx) -> usize {
        let laid = self.layout(cx.fonts(), cx.look());
        laid.lines[line_of(&laid, place)].start
    }

    fn line_end(&self, place: usize, cx: &Cx) -> usize {
        let laid = self.layout(cx.fonts(), cx.look());
        laid.lines[line_of(&laid, place)].end
    }

    /// The place a line up or down, as near across as the caret is.
    fn vertically(&self, by: i64, cx: &Cx) -> usize {
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

    /// How far a line moves right for its alignment, when it fits.
    fn align_shift(&self, laid: &Laid, line: usize, inner: Rect) -> f64 {
        if self.align == TextAlign::Left || self.multiline {
            return 0.0;
        }
        let spare = inner.width - laid.shapes[line].width();
        if spare <= 0.0 {
            0.0
        } else if self.align == TextAlign::Right {
            spare
        } else {
            spare / 2.0
        }
    }

    /// Scrolls so the caret shows, after it moved or the text changed.
    fn scroll_to_caret(&mut self, fonts: &Fonts, look: &ThemeLook) {
        if self.node.bounds.width <= 0.0 {
            return;
        }
        let laid = self.layout(fonts, look);
        let inner = self.inner(look);
        if self.multiline {
            let most = crate::geometry::scroll_room(laid.lines.len() as f64 * laid.height, inner.height);
            if self.follow {
                let top = line_of(&laid, self.caret) as f64 * laid.height;
                if top < self.scroll_y {
                    self.scroll_y = top;
                } else if top + laid.height > self.scroll_y + inner.height {
                    self.scroll_y = top + laid.height - inner.height;
                }
            }
            self.scroll_y = self.scroll_y.clamp(0.0, most);
            self.scroll_x = 0.0;
        } else {
            let full = laid.shapes[0].width();
            let most = (full - inner.width + 1.0).max(0.0);
            if self.follow {
                let x = laid.across[self.caret];
                if x < self.scroll_x {
                    self.scroll_x = x;
                } else if x > self.scroll_x + inner.width - 1.0 {
                    self.scroll_x = x - inner.width + 1.0;
                }
            }
            self.scroll_x = self.scroll_x.clamp(0.0, most);
            self.scroll_y = 0.0;
        }
        self.follow = false;
    }

    // -- Drawing ----------------------------------------------------------------

    /// The colored stretches of its code, found once per version of the text; none for plain text or a password.
    fn inks(&self) -> Option<std::cell::Ref<'_, Vec<(usize, usize, crate::parts::code_colors::Ink)>>> {
        let language = self.language.filter(|_| !self.password)?;
        let fresh = self.inked.borrow().as_ref().is_some_and(|(v, _)| *v == self.version);
        if !fresh {
            *self.inked.borrow_mut() = Some((self.version, crate::parts::code_colors::spans(language, &self.chars)));
        }
        Some(std::cell::Ref::map(self.inked.borrow(), |i| &i.as_ref().expect("found above").1))
    }

    /// Draws the frame, the text, the selection and the caret.
    pub fn paint(&self, p: &Painter) {
        let look = p.look;
        let node = &self.node;
        let state = if !self.enabled {
            None
        } else if node.focused {
            Some("focus")
        } else if node.hovered {
            Some("hover")
        } else {
            None
        };
        // Code to read, not to type in, looks as the theme's code part, beside its input look.
        let reading = self.read_only && self.language.is_some();
        let input = if reading { look.part("code", state).over(&look.part("input", state)) } else { look.part("input", state) };
        let radius = input.radius.map(|r| r as f64).unwrap_or_else(|| look.radius("input"));
        let background = input.background.unwrap_or_else(|| look.color("input"));
        let border = if self.marked { look.color("error") } else { input.border.unwrap_or_else(|| look.color("strong_border")) };
        let faded = !self.enabled;
        let bounds = node.bounds;
        if self.framed {
            p.fill(bounds, fade(background, faded), radius);
            p.stroke(bounds, fade(border, faded), look.border(), radius);
            if node.focused {
                p.focus_ring(bounds, radius);
            }
        }

        let laid = self.layout(p.fonts, look);
        let inner = self.inner(look);
        let size = look.size("text");
        let text_color = fade(if self.framed { input.text.unwrap_or_else(|| look.color("text")) } else { look.color("text") }, faded);
        let font = p.fonts.font(self.font_role, size, false);
        let mut paint = Paint::default();
        paint.set_color(text_color);
        paint.set_anti_alias(true);
        let _clip = p.clip(Rect { x: inner.x - 1.0, width: inner.width + 2.0, ..inner });
        let (_, metrics) = font.metrics();
        let (ascent, descent) = (metrics.ascent as f64, metrics.descent as f64);
        let top_of = |line: usize| {
            if self.multiline { inner.y + line as f64 * laid.height - self.scroll_y } else { inner.y + (inner.height - laid.height) / 2.0 }
        };
        let baseline = |line: usize| top_of(line) + (laid.height - (descent - ascent)) / 2.0 - ascent;

        if self.chars.is_empty() {
            if let Some(placeholder) = self.placeholder.as_deref().filter(|p| !p.is_empty()) {
                let mut muted = Paint::default();
                muted.set_color(fade(look.color("muted"), faded));
                muted.set_anti_alias(true);
                let width = p.fonts.measure(placeholder, self.font_role, size, false);
                let x = if self.align == TextAlign::Left || self.multiline {
                    inner.x
                } else if self.align == TextAlign::Right {
                    inner.right() - width
                } else {
                    inner.x + (inner.width - width) / 2.0
                };
                p.fonts.draw(p.canvas, placeholder, x, baseline(0), self.font_role, size, false, &muted);
            }
        }

        let (start, end) = self.selection();
        for (i, line) in laid.lines.iter().enumerate() {
            let top = top_of(i);
            if top + laid.height < inner.y || top > inner.bottom() {
                continue;
            }
            let left = inner.x - self.scroll_x + self.align_shift(&laid, i, inner);
            let shape = &laid.shapes[i];
            // The selection behind the text, while it has the focus: one stretch, or more where directions mix.
            if node.focused && start != end && start <= line.end && end >= line.start {
                let from = start.max(line.start) - line.start;
                let to = end.min(line.end) - line.start;
                let mut spans = shape.spans(from, to);
                if spans.is_empty() {
                    spans.push((shape.caret_x(from), shape.caret_x(from)));
                }
                // A selected line break shows as a little space, at the line's end.
                if end > line.end && line.end < self.chars.len() {
                    if shape.right_to_left() {
                        spans[0].0 -= size / 3.0;
                    } else if let Some(last) = spans.last_mut() {
                        last.1 += size / 3.0;
                    }
                }
                for (a, b) in spans {
                    p.fill(Rect::new(left + a, top, (b - a).max(1.0), laid.height), look.color("selection"), 0.0);
                }
            }
            if line.end > line.start {
                match self.inks() {
                    // Code: each stretch in its color, where the shaped line puts it.
                    Some(inks) => {
                        let mut at = line.start;
                        let draw = |from: usize, to: usize, ink: Option<crate::parts::code_colors::Ink>| {
                            let piece: String = self.chars[from..to].iter().filter(|c| **c != '\n').collect();
                            if piece.is_empty() {
                                return;
                            }
                            let mut colored = paint.clone();
                            if let Some(ink) = ink {
                                colored.set_color(fade(look.try_color(ink.role()).unwrap_or_else(|| look.color(ink.fallback())), faded));
                            }
                            p.fonts.draw(p.canvas, &piece, left + shape.caret_x(from - line.start), baseline(i), self.font_role, size, false, &colored);
                        };
                        for &(from, to, ink) in inks.iter().filter(|(f, t, _)| *t > line.start && *f < line.end) {
                            let (from, to) = (from.max(line.start), to.min(line.end));
                            if from > at {
                                draw(at, from, None);
                            }
                            draw(from, to, Some(ink));
                            at = to;
                        }
                        if at < line.end {
                            draw(at, line.end, None);
                        }
                    }
                    None => shape.draw(p.canvas, left, baseline(i), &paint, 0.0),
                }
            }
        }

        if node.focused && self.enabled {
            let line = line_of(&laid, self.caret);
            let mut x = inner.x - self.scroll_x + self.align_shift(&laid, line, inner) + laid.across[self.caret];
            let top = top_of(line);
            // What an input method composes shows at the caret on a plate of its own, underlined, the marked part stronger.
            if let Some(composing) = &self.composing {
                let width = p.fonts.measure(&composing.text, "text", size, false);
                let place = Rect::new(x, top, width, laid.height);
                p.fill(place, look.color("input"), 0.0);
                p.fonts.draw(p.canvas, &composing.text, x, baseline(line), "text", size, false, &paint);
                p.fill(Rect::new(x, top + laid.height - 2.0, width, 1.0), text_color, 0.0);
                if let Some((from, to)) = composing.marked.filter(|(f, t)| f < t && *t <= composing.text.len()) {
                    let at = p.fonts.measure(&composing.text[..from], "text", size, false);
                    let wide = p.fonts.measure(&composing.text[from..to], "text", size, false);
                    p.fill(Rect::new(x + at, top + laid.height - 2.0, wide, 2.0), text_color, 0.0);
                }
                x += width;
            }
            p.fill(Rect::new(x.round(), top, 1.0, laid.height), text_color, 0.0);
            self.caret_drawn.set(Some(Rect::new(x.round(), top, 1.0, laid.height)));
        }
    }
}

/// A color at a little over half its strength, for a disabled input.
fn fade(color: Color, faded: bool) -> Color {
    if faded { color.with_a((color.a() as f64 * 0.55) as u8) } else { color }
}

fn line_of(laid: &Laid, place: usize) -> usize {
    laid.lines.iter().rposition(|l| l.start <= place).unwrap_or(0)
}

/// The place on a line nearest `x` across it, on its shaped letters.
fn nearest(laid: &Laid, line: usize, x: f64) -> usize {
    laid.lines[line].start + laid.shapes[line].place_at(x)
}

/// Lines of at most `width`: each paragraph broken after the last space that
/// fits, or within a word longer than the line.
fn wrap(shown: &[char], from: &[f64], width: f64) -> Vec<Line> {
    let mut lines = Vec::new();
    let mut start = 0;
    loop {
        let newline = shown[start..].iter().position(|c| *c == '\n').map(|i| i + start);
        let end = newline.unwrap_or(shown.len());
        let mut line_start = start;
        while from[end] - from[line_start] > width {
            let mut cut = line_start + 1;
            while cut < end && from[cut + 1] - from[line_start] <= width {
                cut += 1;
            }
            // After the last space within the line, when there is one.
            let space = (line_start..cut).rev().find(|i| shown[*i] == ' ');
            let break_at = match space {
                Some(s) if s + 1 > line_start && s + 1 < end => s + 1,
                _ => cut,
            };
            lines.push(Line { start: line_start, end: break_at });
            line_start = break_at;
        }
        lines.push(Line { start: line_start, end });
        match newline {
            None => return lines,
            Some(n) => start = n + 1,
        }
    }
}

impl Element for TextEditor {
    element_node!();

    fn focusable(&self) -> bool {
        self.takes_focus && self.enabled
    }

    fn wants_tab(&self) -> bool {
        self.takes_tab
    }

    fn typing(&self) -> bool {
        true
    }

    fn multiline(&self) -> bool {
        self.multiline
    }

    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.place(area, cx);
    }

    fn draw(&self, p: &Painter) {
        self.paint(p);
    }

    fn on_pointer_pressed(&mut self, at: PointerInput, cx: &mut Cx) {
        self.pressed(at, cx);
    }

    fn on_pointer_moved(&mut self, at: PointerInput, cx: &mut Cx) {
        self.dragged(at, cx);
    }

    fn on_pointer_released(&mut self, _at: PointerInput, _cx: &mut Cx) {
        self.released();
    }

    fn on_wheel(&mut self, _dx: f64, dy: f64, _held: Modifiers, cx: &mut Cx) -> bool {
        self.wheel(dy, cx)
    }

    fn on_focus_changed(&mut self, focused: bool, cx: &mut Cx) {
        self.focus_moved(focused, cx);
    }

    fn on_key(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        self.key(key, cx)
    }

    fn on_text(&mut self, text: &str, cx: &mut Cx) {
        self.typed(text, cx);
    }

    fn on_wake(&mut self, cx: &mut Cx) {
        self.run_menu_asked(cx);
    }

    fn cursor_at(&self, _x: f64, _y: f64) -> Option<Cursor> {
        (self.enabled && self.takes_focus).then_some(Cursor::Text)
    }

    fn context_menu_at(&self, _x: f64, _y: f64) -> Option<Vec<MenuEntry>> {
        self.menu(self.node.id())
    }
}
