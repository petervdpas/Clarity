//! The person's grant of an AI agent's call (docs/ledger/app-mcp.md), in
//! Clarity's own dialog over the window: who wants to read or change the
//! app, what the command does and the values it is given; Allow once, Allow
//! for this session, Refuse. Refuse has the focus and Escape refuses, so
//! nothing is allowed by a key pressed in passing.

use std::cell::Cell;
use std::rc::Rc;

use crate::dialogs::{backdrop, DialogButton};
use crate::element::{Cx, Element, ElementId, Node};
use crate::geometry::Rect;
use crate::grid::row_height;
use crate::input::{Key, KeyInput, Modifiers};
use crate::locale;
use crate::painter::{Painter, TextAlign};
use crate::window::messages::wrap;

const MIN_WIDTH: f64 = 360.0;
const MAX_WIDTH: f64 = 560.0;
const LINE: f64 = 20.0;
const GAP: f64 = 8.0;

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
    lines: Vec<String>,
    granted: Granted,
    panel: Rect,
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
        GrantDialog { node, title, lines, granted, panel: Rect::default() }
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

    fn wrapped(&self, width: f64, fonts: &crate::fonts::Fonts, size: f64) -> (Vec<String>, Vec<String>) {
        let title = wrap(&self.title, width, fonts, size);
        let lines = self.lines.iter().flat_map(|l| wrap(l, width, fonts, size)).collect();
        (title, lines)
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
        let (title, lines) = self.wrapped(width - 40.0, cx.fonts(), size);
        let row = row_height();
        let height = 20.0 + title.len() as f64 * LINE + 12.0 + lines.len() as f64 * LINE + 16.0 + row + 20.0;
        self.panel = Rect::new(area.x + (area.width - width) / 2.0, area.y + ((area.height - height) / 2.0).max(16.0), width, height);
        let y = self.panel.bottom() - 20.0 - row;
        let mut x = self.panel.right() - 20.0 - buttons;
        for (child, width) in self.node.children_mut().iter_mut().zip(widths) {
            child.arrange(Rect::new(x, y, width, row), cx);
            x += width + GAP;
        }
    }

    fn draw(&self, p: &Painter) {
        let look = p.look;
        backdrop(p, self.node.bounds, self.panel);
        let size = look.size("text");
        let inner = self.panel.width - 40.0;
        let (title, lines) = self.wrapped(inner, p.fonts, size);
        let mut y = self.panel.y + 20.0;
        for line in &title {
            p.text(line, Rect::new(self.panel.x + 20.0, y, inner, LINE), look.color("text"), size, "text", true, TextAlign::Left);
            y += LINE;
        }
        y += 12.0;
        for line in &lines {
            p.text(line, Rect::new(self.panel.x + 20.0, y, inner, LINE), look.color("text"), size, "text", false, TextAlign::Left);
            y += LINE;
        }
        crate::element::draw_children(&self.node, p);
    }

    /// Escape refuses; Enter presses the button with the focus (Refuse at first).
    fn dialog_key(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        if key.modifiers != Modifiers::NONE || key.key != Key::Escape {
            return false;
        }
        self.granted.set(Some(GrantChoice::Refuse));
        cx.close(self.node.id());
        true
    }
}
