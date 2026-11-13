//! What a window's handlers say: an info message as a toast at the top right
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
/// How long a toast about an AI agent stays (its change, with Undo; one turned
/// away): longer than an info toast, to read and Undo, and then it goes, so a
/// run of changes never fills the window.
pub const AGENT_TOAST_DURATION: Duration = Duration::from_millis(10000);

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

/// What a toast offers besides closing: its word (Undo) and what it does.
#[derive(Clone)]
pub struct ToastAction {
    pub word: String,
    pub act: std::rc::Rc<dyn Fn(&mut Cx)>,
}

/// How long a declared toast shows when the toast gives no lifespan, in milliseconds.
pub const DEFAULT_TOAST_LIFESPAN_MS: u32 = 5000;

/// The info messages and the project's toasts, each in its corner of the
/// window (an info message at the top right), each with a small x that
/// closes it, a toast's own action (an agent's change has Undo), its
/// level's color and its icon.
#[derive(Default)]
pub struct ToastList {
    node: Node,
    toasts: Vec<Shown>,
    /// Each toast's place, its x's, and its action's.
    placed: Vec<(Rect, Rect, Option<Rect>)>,
}

/// One toast shown: what it says, when it goes, its action and its icon's drawing.
struct Shown {
    message: Message,
    until: Instant,
    action: Option<ToastAction>,
    icon: Option<String>,
}

/// The corner of the window a toast shows in.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Corner {
    TopRight,
    TopLeft,
    BottomRight,
    BottomLeft,
}

impl Shown {
    fn corner(&self) -> Corner {
        match self.message.toast.as_ref().map(|t| t.corner.as_str()) {
            Some("topLeft") => Corner::TopLeft,
            Some("bottomRight") => Corner::BottomRight,
            Some("bottomLeft") => Corner::BottomLeft,
            _ => Corner::TopRight,
        }
    }

    /// The theme's color of its level: a plain message and an info toast the accent.
    fn color(&self) -> &'static str {
        match self.message.toast.as_ref().map(|t| t.level.as_str()) {
            Some("success") => "success",
            Some("warning") => "warning",
            Some("error") => "error",
            _ => "accent",
        }
    }
}

const TOAST_WIDTH: f64 = 420.0;
const LINE: f64 = 20.0;
/// Room for a toast's icon before its text.
const ICON_ROOM: f64 = 26.0;

impl ToastList {
    /// The toasts shown now, oldest first.
    pub fn toasts(&self) -> Vec<Message> {
        self.toasts.iter().map(|t| t.message.clone()).collect()
    }

    /// The words of the toasts' actions, the toasts' order.
    pub fn actions(&self) -> Vec<Option<String>> {
        self.toasts.iter().map(|t| t.action.as_ref().map(|a| a.word.clone())).collect()
    }

    /// Where each toast is, the toasts' order.
    pub fn places(&self) -> Vec<Rect> {
        self.placed.iter().map(|(toast, _, _)| *toast).collect()
    }

    /// The theme color of each toast's level, the toasts' order.
    pub fn colors(&self) -> Vec<&'static str> {
        self.toasts.iter().map(Shown::color).collect()
    }

    /// Whether each toast shows an icon, the toasts' order.
    pub fn icons(&self) -> Vec<bool> {
        self.toasts.iter().map(|t| t.icon.is_some()).collect()
    }

    /// Shows an info message until `until`.
    pub fn add(&mut self, message: Message, until: Instant, cx: &mut Cx) {
        self.add_with(message, until, None, cx);
    }

    /// Shows an info message until `until`, with its action.
    pub fn add_with(&mut self, message: Message, until: Instant, action: Option<ToastAction>, cx: &mut Cx) {
        self.push(Shown { message, until, action, icon: None }, cx);
    }

    /// Shows a toast the project declares until `until`, with its icon's drawing.
    pub fn add_toast(&mut self, message: Message, until: Instant, icon: Option<String>, cx: &mut Cx) {
        self.push(Shown { message, until, action: None, icon }, cx);
    }

    /// Its one wake is at the first toast to go, so a long one never puts off a short one.
    fn push(&mut self, shown: Shown, cx: &mut Cx) {
        self.toasts.push(shown);
        if let Some(first) = self.toasts.iter().map(|t| t.until).min() {
            cx.wake(self.node.id(), first);
        }
        cx.invalidate();
    }

    /// Runs the action of the toast `index`, which then goes.
    pub fn act(&mut self, index: usize, cx: &mut Cx) {
        if index >= self.toasts.len() {
            return;
        }
        let shown = self.toasts.remove(index);
        if index < self.placed.len() {
            self.placed.remove(index);
        }
        if let Some(action) = shown.action {
            (action.act)(cx);
        }
        cx.invalidate();
    }

    /// The toasts whose time is up go.
    pub fn expire(&mut self, now: Instant, cx: &mut Cx) {
        let before = self.toasts.len();
        self.toasts.retain(|t| t.until > now);
        if self.toasts.len() != before {
            cx.invalidate();
        }
        if let Some(next) = self.toasts.iter().map(|t| t.until).min() {
            cx.wake(self.node.id(), next);
        }
    }

    fn text_of(message: &Message) -> String {
        message.text.show()
    }
}

impl Element for ToastList {
    crate::element_node!();

    /// Each corner stacks its toasts: down from the top, up from the bottom, oldest nearest the corner.
    fn arrange(&mut self, window: Rect, cx: &mut Cx) {
        self.placed.clear();
        let width = TOAST_WIDTH.min((window.width - 2.0 * CONTENT_PADDING).max(0.0));
        let size = cx.look().size("text");
        let mut next = [window.y + CONTENT_PADDING, window.y + CONTENT_PADDING, window.bottom() - CONTENT_PADDING, window.bottom() - CONTENT_PADDING];
        for shown in &self.toasts {
            let corner = shown.corner();
            let text_width = width - 48.0 - if shown.icon.is_some() { ICON_ROOM } else { 0.0 };
            let lines = wrap(&Self::text_of(&shown.message), text_width, cx.fonts(), size).len() as f64;
            let room = if shown.action.is_some() { LINE } else { 0.0 };
            let height = lines * LINE + 20.0 + room;
            let x = match corner {
                Corner::TopLeft | Corner::BottomLeft => window.x + CONTENT_PADDING,
                Corner::TopRight | Corner::BottomRight => window.right() - CONTENT_PADDING - width,
            };
            let slot = corner as usize;
            let y = match corner {
                Corner::TopRight | Corner::TopLeft => {
                    let y = next[slot];
                    next[slot] = y + height + 8.0;
                    y
                }
                Corner::BottomRight | Corner::BottomLeft => {
                    let y = next[slot] - height;
                    next[slot] = y - 8.0;
                    y
                }
            };
            let toast = Rect::new(x, y, width, height);
            let act = shown.action.as_ref().map(|_| Rect::new(toast.x + 14.0, toast.bottom() - 8.0 - LINE, 96.0, LINE));
            self.placed.push((toast, Rect::new(toast.right() - 30.0, toast.y + 6.0, 24.0, 24.0), act));
        }
        // Its bounds hold every toast, so the pointer reaches each.
        self.node.bounds = self.placed.iter().map(|(t, _, _)| *t).reduce(|a, b| {
            let (x, y) = (a.x.min(b.x), a.y.min(b.y));
            Rect::new(x, y, a.right().max(b.right()) - x, a.bottom().max(b.bottom()) - y)
        }).unwrap_or(Rect::new(window.right(), window.y, 0.0, 0.0));
    }

    fn draw(&self, p: &Painter) {
        let look = p.look;
        let size = look.size("text");
        for (shown, (toast, close, act)) in self.toasts.iter().zip(&self.placed) {
            let radius = look.radius("panel");
            let color = look.color(shown.color());
            p.fill(*toast, look.color("raised"), radius);
            p.stroke(*toast, look.color("border"), look.border(), radius);
            p.fill(Rect { width: 4.0, ..*toast }, color, radius);
            let mut left = toast.x + 14.0;
            if shown.icon.is_some() {
                crate::parts::containers::icon::draw_icon(p, shown.icon.as_deref(), Rect::new(left, toast.y + 10.0, 18.0, 18.0), color);
                left += ICON_ROOM;
            }
            let text_width = toast.right() - 34.0 - left;
            let lines = wrap(&Self::text_of(&shown.message), text_width, p.fonts, size);
            for (i, line) in lines.iter().enumerate() {
                p.text(line, Rect::new(left, toast.y + 10.0 + i as f64 * LINE, text_width, LINE), look.color("text"), size, "text", false, TextAlign::Left);
            }
            p.text("x", *close, look.color("muted"), size, "text", false, TextAlign::Center);
            if let (Some(action), Some(act)) = (&shown.action, act) {
                p.text(&action.word, *act, look.color("accent"), size, "text", true, TextAlign::Left);
            }
        }
    }

    fn hit(&self, x: f64, y: f64) -> bool {
        self.placed.iter().any(|(toast, _, _)| toast.contains(x, y))
    }

    /// A toast's x says what it does.
    fn tooltip_at(&self, x: f64, y: f64) -> Option<String> {
        self.placed.iter().any(|(_, close, _)| close.contains(x, y)).then(|| locale::translate("dismiss", "Dismiss"))
    }

    /// A toast's x shows the hand: a press there closes the toast.
    fn cursor_at(&self, x: f64, y: f64) -> Option<Cursor> {
        self.placed.iter().any(|(_, close, act)| close.contains(x, y) || act.is_some_and(|a| a.contains(x, y))).then_some(Cursor::Hand)
    }

    fn on_pointer_pressed(&mut self, at: PointerInput, cx: &mut Cx) {
        if let Some(i) = self.placed.iter().position(|(_, _, act)| act.is_some_and(|a| a.contains(at.screen_x, at.screen_y))) {
            self.act(i, cx);
            return;
        }
        if let Some(i) = self.placed.iter().position(|(_, close, _)| close.contains(at.screen_x, at.screen_y)) {
            self.toasts.remove(i);
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
