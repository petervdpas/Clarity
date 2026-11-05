//! What lies over a window: a dropdown's list, a menu, a popover, a
//! slide-out, a dialog. It is drawn over everything at its place and takes
//! the pointer first; a press outside it (and outside what it opened from),
//! or Escape, closes the one on top (unless it is modal, or stays: a pinned
//! slide-out). And the clipboard, as the host gives it.

use std::rc::Rc;

use crate::element::{Element, ElementId};
use crate::geometry::Rect;

/// Something over the window.
pub struct Overlay {
    pub content: Box<dyn Element>,
    /// Where it lies, on the screen.
    pub place: Rect,
    /// A dialog: a press outside does not close it, what is under takes
    /// nothing, and its content hears keys first (`Element::dialog_key`).
    pub modal: bool,
    /// It covers the whole screen, and follows it when it is resized.
    pub covers: bool,
    /// The element it opened from (a dropdown's own box): a press there is no
    /// press outside, so it goes to that element and the overlay stays.
    pub anchor: Option<ElementId>,
    /// The element told when it closes (`Element::on_overlay_closed`).
    pub opener: Option<ElementId>,
    /// Runs when it closed: by a press outside, Escape, or a close.
    pub closed: Option<Box<dyn FnOnce()>>,
    /// A press outside it closes it and then goes on to what is under (a
    /// dropdown's list, a calendar); else the press only closes it (a menu).
    pub through: bool,
    /// While this says so, a press outside it leaves it open and goes on to
    /// what is under, and Escape closes it only while the focus is in it (a
    /// pinned slide-out beside the window's parts).
    pub stays: Option<Rc<dyn Fn() -> bool>>,
}

impl Overlay {
    pub fn new(content: Box<dyn Element>, place: Rect) -> Overlay {
        Overlay { content, place, modal: false, covers: false, anchor: None, opener: None, closed: None, through: false, stays: None }
    }

    /// A dialog over the whole screen: modal, and covering it.
    pub fn dialog(content: Box<dyn Element>) -> Overlay {
        Overlay { modal: true, covers: true, ..Overlay::new(content, Rect::default()) }
    }

    /// Whether it stays open now when the person presses outside it.
    pub fn staying(&self) -> bool {
        self.stays.as_ref().is_some_and(|stays| stays())
    }

    /// Its id: its content's.
    pub fn id(&self) -> ElementId {
        self.content.node().id()
    }
}

/// Copying and pasting text: the system's, as the host gives it.
pub trait Clipboard {
    fn get_text(&mut self) -> Option<String>;
    fn set_text(&mut self, text: &str);
}

/// A clipboard of the screen's own, until the host gives the system's.
#[derive(Debug, Default)]
pub struct OwnClipboard {
    text: Option<String>,
}

impl Clipboard for OwnClipboard {
    fn get_text(&mut self) -> Option<String> {
        self.text.clone()
    }

    fn set_text(&mut self, text: &str) {
        self.text = Some(text.to_string());
    }
}
