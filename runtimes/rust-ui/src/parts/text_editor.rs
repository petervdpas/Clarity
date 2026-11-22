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

mod caret;
mod editing;
mod painting;
mod pointer;
mod undo;

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
    /// Where on its scroll bar's thumb the pointer took it, while dragging.
    grab: Option<f64>,
    /// Where its thumb was drawn last.
    thumb_place: Cell<Option<Rect>>,
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
    /// A memo of Markdown: Ctrl+B and Ctrl+I make the selection bold and italic.
    pub markdown: bool,
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
            grab: None,
            thumb_place: Cell::new(None),
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
            markdown: false,
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

    fn editable(&self) -> bool {
        self.enabled && !self.read_only
    }

    // -- Its own undo -------------------------------------------------------

    // -- Keys ---------------------------------------------------------------

    // -- The pointer ----------------------------------------------------------

    // -- Lines, as they are laid out --------------------------------------------

    // -- Drawing ----------------------------------------------------------------

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
