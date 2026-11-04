//! What a window's handlers say: an info message as a note at the top right
//! that goes away by itself (or by its x), a warning or an error in
//! Clarity's own dialog over the window, read and closed with OK, never the
//! system's. The dialog's top edge is in the color of its level.

use std::time::{Duration, Instant};

use clarity_runtime::ui::{Level, Message};

use crate::element::{Cx, Element, Node};
use crate::fonts::Fonts;
use crate::geometry::Rect;
use crate::grid::row_height;
use crate::input::{Cursor, Key, KeyInput, PointerInput};
use crate::locale;
use crate::painter::{Painter, TextAlign};
use crate::window::view::CONTENT_PADDING;

/// How long an info message stays (clarity_spec::layout MESSAGE_MS).
pub const INFO_DURATION: Duration = Duration::from_millis(5000);

/// A text in lines that fit `width`, broken between words; a word wider than the line keeps a line of its own.
pub fn wrap(text: &str, width: f64, fonts: &Fonts, size: f64) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        let mut line = String::new();
        for word in paragraph.split(' ').filter(|w| !w.is_empty()) {
            let longer = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
            if !line.is_empty() && fonts.measure(&longer, "text", size, false) > width {
                lines.push(std::mem::replace(&mut line, word.to_string()));
            } else {
                line = longer;
            }
        }
        lines.push(line);
    }
    lines
}

/// The info messages, as notes at the top right of the window, each with a small x that closes it.
#[derive(Default)]
pub struct NoteList {
    node: Node,
    /// Each note with the time it goes.
    notes: Vec<(Message, Instant)>,
    placed: Vec<(Rect, Rect)>,
}

const NOTE_WIDTH: f64 = 420.0;
const LINE: f64 = 20.0;

impl NoteList {
    /// The notes shown now, oldest first.
    pub fn notes(&self) -> Vec<Message> {
        self.notes.iter().map(|(m, _)| m.clone()).collect()
    }

    /// Shows an info message until `until`.
    pub fn add(&mut self, message: Message, until: Instant, cx: &mut Cx) {
        self.notes.push((message, until));
        cx.wake(self.node.id(), until);
        cx.invalidate();
    }

    /// The notes whose time is up go.
    pub fn expire(&mut self, now: Instant, cx: &mut Cx) {
        let before = self.notes.len();
        self.notes.retain(|(_, until)| *until > now);
        if self.notes.len() != before {
            cx.invalidate();
        }
        if let Some(next) = self.notes.iter().map(|(_, u)| *u).min() {
            cx.wake(self.node.id(), next);
        }
    }

    fn text_of(message: &Message) -> String {
        message.text.show()
    }
}

impl Element for NoteList {
    crate::element_node!();

    fn arrange(&mut self, window: Rect, cx: &mut Cx) {
        self.placed.clear();
        let width = NOTE_WIDTH.min((window.width - 2.0 * CONTENT_PADDING).max(0.0));
        let x = window.right() - CONTENT_PADDING - width;
        let mut y = window.y + CONTENT_PADDING;
        let size = cx.look().size("text");
        for (message, _) in &self.notes {
            let lines = wrap(&Self::text_of(message), width - 48.0, cx.fonts(), size).len() as f64;
            let note = Rect::new(x, y, width, lines * LINE + 20.0);
            self.placed.push((note, Rect::new(note.right() - 30.0, note.y + 6.0, 24.0, 24.0)));
            y = note.bottom() + 8.0;
        }
        self.node.bounds = if self.placed.is_empty() {
            Rect::new(window.right(), window.y, 0.0, 0.0)
        } else {
            Rect::new(x, window.y + CONTENT_PADDING, width, y - 8.0 - window.y - CONTENT_PADDING)
        };
    }

    fn draw(&self, p: &Painter) {
        let look = p.look;
        let size = look.size("text");
        for ((message, _), (note, close)) in self.notes.iter().zip(&self.placed) {
            let radius = look.radius("panel");
            p.fill(*note, look.color("raised"), radius);
            p.stroke(*note, look.color("border"), look.border(), radius);
            p.fill(Rect { width: 4.0, ..*note }, look.color("accent"), radius);
            let lines = wrap(&Self::text_of(message), note.width - 48.0, p.fonts, size);
            for (i, line) in lines.iter().enumerate() {
                p.text(line, Rect::new(note.x + 14.0, note.y + 10.0 + i as f64 * LINE, note.width - 48.0, LINE), look.color("text"), size, "text", false, TextAlign::Left);
            }
            p.text("x", *close, look.color("muted"), size, "text", false, TextAlign::Center);
        }
    }

    fn hit(&self, x: f64, y: f64) -> bool {
        self.placed.iter().any(|(note, _)| note.contains(x, y))
    }

    /// A note's x says what it does.
    fn tooltip_at(&self, x: f64, y: f64) -> Option<String> {
        self.placed.iter().any(|(_, close)| close.contains(x, y)).then(|| locale::translate("dismiss", "Dismiss"))
    }

    /// A note's x shows the hand: a press there closes the note.
    fn cursor_at(&self, x: f64, y: f64) -> Option<Cursor> {
        self.placed.iter().any(|(_, close)| close.contains(x, y)).then_some(Cursor::Hand)
    }

    fn on_pointer_pressed(&mut self, at: PointerInput, cx: &mut Cx) {
        if let Some(i) = self.placed.iter().position(|(_, close)| close.contains(at.screen_x, at.screen_y)) {
            self.notes.remove(i);
            self.placed.remove(i);
            cx.invalidate();
        }
    }

    fn on_wake(&mut self, cx: &mut Cx) {
        self.expire(cx.now(), cx);
    }
}

/// A warning or an error in Clarity's own dialog: its text and OK over a dimmed window.
pub struct SaidDialog {
    node: Node,
    pub message: Message,
    panel: Rect,
}

const DIALOG_WIDTH: f64 = 420.0;

impl SaidDialog {
    pub fn new(message: Message) -> SaidDialog {
        let mut node = Node::new();
        let dialog = node.id();
        node.add(Box::new(OkButton { node: Node::new(), dialog }));
        SaidDialog { node, message, panel: Rect::default() }
    }

    /// The OK button's id, which takes the focus as the dialog opens.
    pub fn ok(&self) -> crate::element::ElementId {
        self.node.children()[0].node().id()
    }

    /// Where the dialog itself is, in the middle of the window.
    pub fn panel(&self) -> Rect {
        self.panel
    }
}

impl Element for SaidDialog {
    crate::element_node!();

    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.node.bounds = area;
        let width = DIALOG_WIDTH.min((area.width - 32.0).max(160.0));
        let lines = wrap(&self.message.text.show(), width - 40.0, cx.fonts(), cx.look().size("text")).len() as f64;
        let row = row_height();
        let height = 20.0 + lines * LINE + 20.0 + row + 20.0;
        self.panel = Rect::new(area.x + (area.width - width) / 2.0, area.y + ((area.height - height) / 2.0).max(16.0), width, height);
        let ok = Rect::new(self.panel.right() - 20.0 - 96.0, self.panel.bottom() - 20.0 - row, 96.0, row);
        self.node.children_mut()[0].arrange(ok, cx);
    }

    fn draw(&self, p: &Painter) {
        let look = p.look;
        // The window under it is dimmed: it takes nothing while the dialog is open.
        p.fill(self.node.bounds, look.color("text").with_a(56), 0.0);
        let radius = look.radius("panel");
        p.fill(self.panel, look.color("panel"), radius);
        p.stroke(self.panel, look.color("strong_border"), look.border(), radius);
        let tone = look.color(if self.message.level == Level::Error { "error" } else { "warning" });
        p.fill(Rect { height: 4.0, ..self.panel }, tone, radius);
        let inner = self.panel.width - 40.0;
        let size = look.size("text");
        for (i, line) in wrap(&self.message.text.show(), inner, p.fonts, size).iter().enumerate() {
            p.text(line, Rect::new(self.panel.x + 20.0, self.panel.y + 20.0 + i as f64 * LINE, inner, LINE), look.color("text"), size, "text", false, TextAlign::Left);
        }
        crate::element::draw_children(&self.node, p);
    }

    fn dialog_key(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        if !matches!(key.key, Key::Enter | Key::Space | Key::Escape) {
            return false;
        }
        cx.close(self.node.id());
        true
    }
}

/// The dialog's OK: it has the focus, so Enter takes the dialog away.
pub(crate) struct OkButton {
    node: Node,
    dialog: crate::element::ElementId,
}

impl Element for OkButton {
    crate::element_node!();

    fn focusable(&self) -> bool {
        true
    }

    fn on_click(&mut self, cx: &mut Cx) {
        cx.close(self.dialog);
    }

    fn draw(&self, p: &Painter) {
        let look = p.look;
        let state = if self.node.pressed { Some("pressed") } else if self.node.hovered { Some("hover") } else { None };
        let values = look.part("button", state);
        let radius = values.radius.map(|r| r as f64).unwrap_or_else(|| look.radius("button"));
        let bounds = self.node.bounds;
        p.fill(bounds, values.background.unwrap_or_else(|| look.color("accent")), radius);
        p.stroke(bounds, values.border.unwrap_or_else(|| look.color("border")), look.border(), radius);
        if self.node.focused {
            p.stroke(bounds.inset(-2.0), look.color("focus"), 2.0, radius + 2.0);
        }
        let size = values.size.map(|s| s as f64).unwrap_or_else(|| look.size("text"));
        p.text(&locale::translate("ok", "OK"), bounds, values.text.unwrap_or_else(|| look.color("on_accent")), size, "text", values.bold(), TextAlign::Center);
    }
}
