//! The person's grant of an AI agent's call (docs/ledger/app-mcp.md), in
//! Clarity's own dialog over the window: who wants to read or change the
//! app, what the command does and the values it is given; Allow once, Allow
//! for this session, Refuse. Refuse has the focus and Escape refuses, so
//! nothing is allowed by a key pressed in passing. The lines keep to an area
//! of at most TEXT_MOST high, and the window's height, that scrolls (wheel,
//! its bar, and Up, Down, Page Up, Page Down, Home, End), so the dialog keeps
//! a dialog's size and its buttons show whatever an agent sends.

use std::cell::Cell;
use std::rc::Rc;

use crate::dialogs::{backdrop, DialogButton};
use crate::element::{Cx, Element, ElementId, Node};
use crate::geometry::Rect;
use crate::grid::row_height;
use crate::input::{Key, KeyInput, Modifiers, PointerInput};
use crate::locale;
use crate::painter::{Painter, TextAlign};
use crate::window::messages::wrap;

const MIN_WIDTH: f64 = 360.0;
const MAX_WIDTH: f64 = 560.0;
const LINE: f64 = 20.0;
const GAP: f64 = 8.0;
/// The most the lines take, about fourteen of them: more scroll.
const TEXT_MOST: f64 = 280.0;
/// Room at the right of the lines for their bar.
const BAR_ROOM: f64 = 10.0;

/// What the person chose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrantChoice {
    Once,
    Session,
    Refuse,
}

/// What the person chose, shared with whoever opened the dialog: read it once it closed; none is a refusal.
pub type Granted = Rc<Cell<Option<GrantChoice>>>;

pub struct GrantDialog {
    node: Node,
    title: String,
    granted: Granted,
    panel: Rect,
}

/// The dialog's lines, in an area that scrolls when they are more than it holds.
pub struct GrantText {
    node: Node,
    lines: Vec<String>,
    /// The lines wrapped to the area's width, as last arranged.
    wrapped: Vec<String>,
    scroll_y: f64,
    max_scroll: f64,
    /// Where on the thumb the pointer took it, while it drags.
    grab: Option<f64>,
}

impl GrantText {
    fn new(lines: Vec<String>) -> GrantText {
        GrantText { node: Node::new(), lines, wrapped: Vec::new(), scroll_y: 0.0, max_scroll: 0.0, grab: None }
    }

    /// How far the lines are scrolled down.
    pub fn scroll_y(&self) -> f64 {
        self.scroll_y
    }

    /// How far they can scroll: 0 when all show.
    pub fn max_scroll(&self) -> f64 {
        self.max_scroll
    }

    /// How high all the lines are at `width`.
    fn content_height(&self, width: f64, fonts: &crate::fonts::Fonts, size: f64) -> f64 {
        self.lines.iter().map(|l| wrap(l, width, fonts, size).len()).sum::<usize>() as f64 * LINE
    }

    fn bar(&self) -> crate::geometry::Thumb {
        crate::geometry::Thumb { track: self.node.bounds.height, content: self.wrapped.len() as f64 * LINE, scroll: self.scroll_y, max: self.max_scroll }
    }

    fn thumb(&self) -> Rect {
        let b = self.node.bounds;
        let bar = self.bar();
        Rect::new(b.right() - BAR_ROOM / 2.0 - 2.0, b.y + bar.at(), 4.0, bar.length())
    }

    /// Scrolls to `to`, kept within the lines; true when it moved.
    fn scroll_to(&mut self, to: f64, cx: &mut Cx) -> bool {
        let to = to.clamp(0.0, self.max_scroll);
        if to == self.scroll_y {
            return false;
        }
        self.scroll_y = to;
        cx.invalidate();
        true
    }

    /// The keys that scroll the lines, whatever has the focus in the dialog.
    fn scroll_key(&mut self, key: Key, cx: &mut Cx) -> bool {
        let page = (self.node.bounds.height - LINE).max(LINE);
        let to = match key {
            Key::Up => self.scroll_y - LINE,
            Key::Down => self.scroll_y + LINE,
            Key::PageUp => self.scroll_y - page,
            Key::PageDown => self.scroll_y + page,
            Key::Home => 0.0,
            Key::End => self.max_scroll,
            _ => return false,
        };
        self.scroll_to(to, cx);
        true
    }
}

impl Element for GrantText {
    crate::element_node!();

    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.node.bounds = area;
        let size = cx.look().size("text");
        let width = (area.width - BAR_ROOM).max(40.0);
        self.wrapped = self.lines.iter().flat_map(|l| wrap(l, width, cx.fonts(), size)).collect();
        self.max_scroll = crate::geometry::scroll_room(self.wrapped.len() as f64 * LINE, area.height);
        self.scroll_y = self.scroll_y.clamp(0.0, self.max_scroll);
    }

    fn draw(&self, p: &Painter) {
        let look = p.look;
        let b = self.node.bounds;
        let size = look.size("text");
        {
            let _clip = p.clip(b);
            let first = (self.scroll_y / LINE).floor() as usize;
            let mut y = b.y + first as f64 * LINE - self.scroll_y;
            for line in self.wrapped.iter().skip(first) {
                if y > b.bottom() {
                    break;
                }
                p.text(line, Rect::new(b.x, y, b.width - BAR_ROOM, LINE), look.color("text"), size, "text", false, TextAlign::Left);
                y += LINE;
            }
        }
        if self.max_scroll > 0.0 {
            // A thin bar at the right says how much of the lines show, and where; it drags and pages.
            let color = if self.grab.is_some() || self.node.hovered { "muted" } else { "strong_border" };
            p.fill(self.thumb(), look.color(color), 2.0);
        }
    }

    fn on_wheel(&mut self, _dx: f64, dy: f64, _held: Modifiers, cx: &mut Cx) -> bool {
        self.scroll_to(self.scroll_y + dy, cx)
    }

    fn on_pointer_pressed(&mut self, at: PointerInput, cx: &mut Cx) {
        self.grab = None;
        if self.max_scroll > 0.0 && at.x >= self.node.bounds.width - BAR_ROOM {
            match self.bar().pressed(at.y, self.node.bounds.height) {
                Ok(grab) => self.grab = Some(grab),
                Err(to) => {
                    self.scroll_to(to, cx);
                }
            }
        }
    }

    fn on_pointer_moved(&mut self, at: PointerInput, cx: &mut Cx) {
        let Some(grab) = self.grab.filter(|_| self.node.pressed) else { return };
        let to = self.bar().dragged(at.y, grab);
        self.scroll_to(to, cx);
    }

    fn on_pointer_released(&mut self, _at: PointerInput, cx: &mut Cx) {
        if self.grab.take().is_some() {
            cx.invalidate();
        }
    }
}

impl GrantDialog {
    /// The dialog: its title, and its lines (what the command does, then each value given).
    pub fn new(title: String, lines: Vec<String>) -> GrantDialog {
        let mut node = Node::new();
        let dialog = node.id();
        let granted: Granted = Rc::default();
        let button = |key: &str, english: &str, accent: bool, choice: GrantChoice| {
            let granted = granted.clone();
            DialogButton::new(locale::translate(key, english), accent, Rc::new(move |cx: &mut Cx| {
                granted.set(Some(choice));
                cx.close(dialog);
            }))
        };
        // In the order they show, left to right; Refuse is the first, and takes the focus.
        node.add(Box::new(button("agentRefuse", "Refuse", false, GrantChoice::Refuse)));
        node.add(Box::new(button("agentAllowSession", "Allow for this session", false, GrantChoice::Session)));
        node.add(Box::new(button("agentAllowOnce", "Allow once", true, GrantChoice::Once)));
        node.add(Box::new(GrantText::new(lines)));
        GrantDialog { node, title, granted, panel: Rect::default() }
    }

    pub fn granted(&self) -> Granted {
        self.granted.clone()
    }

    /// Its buttons: Refuse, Allow for this session, Allow once.
    pub fn buttons(&self) -> Vec<&DialogButton> {
        self.node.children().iter().filter_map(|c| c.as_any().downcast_ref::<DialogButton>()).collect()
    }

    /// What takes the focus as it opens: Refuse.
    pub fn first(&self) -> ElementId {
        self.node.children()[0].node().id()
    }

    pub fn panel(&self) -> Rect {
        self.panel
    }

    /// Its lines' area, which scrolls.
    pub fn text(&self) -> &GrantText {
        self.node.children().iter().find_map(|c| c.as_any().downcast_ref::<GrantText>()).expect("the grant dialog holds its lines")
    }

    fn text_mut(&mut self) -> &mut GrantText {
        self.node.children_mut().iter_mut().find_map(|c| c.as_any_mut().downcast_mut::<GrantText>()).expect("the grant dialog holds its lines")
    }
}

impl Element for GrantDialog {
    crate::element_node!();

    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.node.bounds = area;
        let size = cx.look().size("text");
        let widths: Vec<f64> = self.buttons().iter().map(|b| (cx.fonts().measure(&b.text, "text", size, false) + 32.0).max(96.0)).collect();
        let buttons: f64 = widths.iter().sum::<f64>() + GAP * (widths.len() as f64 - 1.0);
        let width = (buttons + 40.0).clamp(MIN_WIDTH, MAX_WIDTH).min((area.width - 32.0).max(200.0));
        let title = wrap(&self.title, width - 40.0, cx.fonts(), size);
        let row = row_height();
        // The panel without its lines; the lines get what is left of TEXT_MOST and the window's height.
        let around = 20.0 + title.len() as f64 * LINE + 12.0 + 16.0 + row + 20.0;
        let content = self.text().content_height(width - 40.0 - BAR_ROOM, cx.fonts(), size);
        let text_height = content.min(TEXT_MOST).min((area.height - 32.0 - around).max(LINE));
        let height = around + text_height;
        self.panel = Rect::new(area.x + (area.width - width) / 2.0, area.y + ((area.height - height) / 2.0).max(16.0), width, height);
        let text_area = Rect::new(self.panel.x + 20.0, self.panel.y + 20.0 + title.len() as f64 * LINE + 12.0, width - 40.0, text_height);
        self.text_mut().arrange(text_area, cx);
        let y = self.panel.bottom() - 20.0 - row;
        let mut x = self.panel.right() - 20.0 - buttons;
        for (child, width) in self.node.children_mut().iter_mut().filter(|c| c.as_any().downcast_ref::<DialogButton>().is_some()).zip(widths) {
            child.arrange(Rect::new(x, y, width, row), cx);
            x += width + GAP;
        }
    }

    fn draw(&self, p: &Painter) {
        let look = p.look;
        backdrop(p, self.node.bounds, self.panel);
        let size = look.size("text");
        let inner = self.panel.width - 40.0;
        let mut y = self.panel.y + 20.0;
        for line in wrap(&self.title, inner, p.fonts, size) {
            p.text(&line, Rect::new(self.panel.x + 20.0, y, inner, LINE), look.color("text"), size, "text", true, TextAlign::Left);
            y += LINE;
        }
        crate::element::draw_children(&self.node, p);
    }

    /// Escape refuses; Enter presses the button with the focus (Refuse at first);
    /// Up, Down, Page Up, Page Down, Home and End scroll the lines.
    fn dialog_key(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        if key.modifiers != Modifiers::NONE {
            return false;
        }
        if key.key != Key::Escape {
            return self.text_mut().scroll_key(key.key, cx);
        }
        self.granted.set(Some(GrantChoice::Refuse));
        cx.close(self.node.id());
        true
    }
}
