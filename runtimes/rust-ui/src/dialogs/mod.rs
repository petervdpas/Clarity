//! Clarity's own dialogs (DESIGN 7.9), never the system's: a question a
//! handler asks (`ask`), an AI agent's call the person allows or refuses (`grant`), a dialog window shown over the window that opened
//! it where the host opens no desktop window of its own (`over`), the keys
//! window (`keys`) and the bar of a long call with Stop (`long_call`). What
//! they share is here: their buttons, and the dimmed window with the panel
//! they are drawn on.

pub mod ask;
pub mod grant;
pub mod keys;
pub mod long_call;
pub mod over;

use std::rc::Rc;

use crate::element::{Cx, Element, Node};
use crate::geometry::Rect;
use crate::input::{Key, KeyInput, Modifiers};
use crate::painter::{Painter, TextAlign};

pub use ask::AskDialog;
pub use grant::{GrantChoice, GrantDialog, GrantText};
pub use keys::KeysWindow;
pub use long_call::LongCallBar;
pub use over::DialogOverlay;

/// What a dialog's button does when it is pressed.
pub type Pressed = Rc<dyn Fn(&mut Cx)>;

/// Whether a dialog's button can be pressed now (Stop once pressed cannot).
pub type Usable = Rc<dyn Fn() -> bool>;

/// A dialog's button: OK in the accent's color, the others in the button's;
/// it takes the focus by Tab, and Enter or Space presses it. One that cannot
/// be pressed now is greyed and takes nothing.
pub struct DialogButton {
    node: Node,
    pub text: String,
    accent: bool,
    pressed: Pressed,
    usable: Option<Usable>,
}

impl DialogButton {
    pub fn new(text: String, accent: bool, pressed: Pressed) -> DialogButton {
        DialogButton { node: Node::new(), text, accent, pressed, usable: None }
    }

    /// The same button, pressed only while `usable` says it can be.
    pub fn usable_while(mut self, usable: Usable) -> DialogButton {
        self.usable = Some(usable);
        self
    }

    /// Whether it can be pressed now.
    pub fn usable(&self) -> bool {
        self.usable.as_ref().is_none_or(|u| u())
    }

    /// Presses it, as a click does, while it can be.
    pub fn press(&mut self, cx: &mut Cx) {
        if self.usable() {
            (self.pressed)(cx);
        }
    }
}

impl Element for DialogButton {
    crate::element_node!();

    fn focusable(&self) -> bool {
        true
    }

    fn is_enabled(&self) -> bool {
        self.usable()
    }

    fn on_click(&mut self, cx: &mut Cx) {
        self.press(cx);
    }

    fn on_key(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        if key.modifiers != Modifiers::NONE || !matches!(key.key, Key::Enter | Key::Space) {
            return false;
        }
        self.press(cx);
        true
    }

    fn draw(&self, p: &Painter) {
        let _dimmed = p.dimmed(self.usable());
        let look = p.look;
        let state = if self.node.pressed { Some("pressed") } else if self.node.hovered { Some("hover") } else { None };
        let values = look.part("button", state);
        let radius = values.radius.map(|r| r as f64).unwrap_or_else(|| look.radius("button"));
        let bounds = self.node.bounds;
        let fill = if self.accent { look.color("accent") } else { values.background.unwrap_or_else(|| look.color("panel")) };
        p.fill(bounds, fill, radius);
        p.stroke(bounds, values.border.unwrap_or_else(|| look.color("border")), look.border(), radius);
        if self.node.focused {
            p.stroke(bounds.inset(-2.0), look.color("focus"), 2.0, radius + 2.0);
        }
        let color = if self.accent { look.color("on_accent") } else { values.text.unwrap_or_else(|| look.color("text")) };
        let size = values.size.map(|s| s as f64).unwrap_or_else(|| look.size("text"));
        p.text(&self.text, bounds, color, size, "text", values.bold(), TextAlign::Center);
    }
}

/// The window under a dialog dimmed (it takes nothing while the dialog is open) and the dialog's panel.
pub fn backdrop(p: &Painter, window: Rect, panel: Rect) {
    let look = p.look;
    p.fill(window, look.color("text").with_a(56), 0.0);
    let radius = look.radius("panel");
    p.fill(panel, look.color("panel"), radius);
    p.stroke(panel, look.color("strong_border"), look.border(), radius);
}
