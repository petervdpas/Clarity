//! The dropdown (DESIGN 7.9, conformance/forms.json): a closed box showing
//! the label of its value's choice, which opens its list over the window
//! below it (above when there is no room below). Only the person picking
//! another choice is a change; what code sets is not. Its choices are an
//! enum's values or the records a reference can point at (an empty one
//! first), as the host offers them; the settings language and theme offer
//! the project's languages and themes. As Avalonia's ComboBox: closed, Up
//! and Down pick the choice before or after at once, Home and End the first
//! and last, and a typed letter jumps to the next choice it starts; Enter,
//! Space, F4 and Alt+Down open the list. Up, Down, Page Up, Page Down, Home
//! and End move through the open list, a letter marks, Enter or Space
//! picks, Escape closes it.
//!
//! A dropdown marked `typed` or `search` is one of the variants of the
//! typed dropdown module; `make` makes the variant that fits, else this one.

use std::rc::Rc;

use serde_json::Value;

use crate::element::{Cx, Element, ElementId};
use crate::geometry::Rect;
use crate::input::{Key, KeyInput};
use crate::markup::PartSpec;
use crate::painter::{Painter, TextAlign};
use crate::part_element;
use crate::parts::inputs::choice_list::{self, ListState, SharedList, MOST_SHOWN};
use crate::parts::inputs::choices;
use crate::parts::inputs::typed_dropdown;
use crate::parts::{Choice, Part, PartBase, PartHost};

/// The part of a dropdown's spec: its typed or search variant when one fits, else the plain dropdown.
pub fn make(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    if let Some(variant) = typed_dropdown::variant_of(&spec, host.as_ref()) {
        return Box::new(typed_dropdown::TypeAheadPart::new(spec, host, variant));
    }
    Box::new(DropdownPart::new(spec, host))
}

/// The choices of a part bound to `bound`: the host's, else the project's languages or themes for those settings.
pub fn offered_for(host: &dyn PartHost, bound: Option<&str>) -> Vec<Choice> {
    let offered = choices::offered_for(host, bound);
    if !offered.is_empty() {
        return offered;
    }
    let as_choices = |offers: Vec<crate::choices::Offer>| offers.into_iter().map(|o| Choice { value: Value::from(o.name), label: o.label, cells: None }).collect();
    match bound {
        Some("settings.language") => as_choices(crate::choices::languages()),
        Some("settings.theme") => as_choices(crate::choices::themes()),
        _ => Vec::new(),
    }
}

/// A plain dropdown of choices.
pub struct DropdownPart {
    base: PartBase,
    /// Its list while it is open: the overlay's id and the state it shares with it.
    list: Option<(ElementId, SharedList)>,
}

impl DropdownPart {
    pub fn new(spec: PartSpec, host: Rc<dyn PartHost>) -> DropdownPart {
        DropdownPart { base: PartBase::new(spec, host), list: None }
    }

    /// What it offers: the host's choices of its bound field, or the settings' languages and themes.
    pub fn choices(&self) -> Vec<Choice> {
        offered_for(self.base.host.as_ref(), self.base.bound())
    }

    /// Its value; none unbound.
    pub fn value(&self) -> Option<Value> {
        self.base.bound().map(|path| self.base.host.read(path).unwrap_or(Value::Null))
    }

    /// Where its value is among its choices; none when it is none of them.
    pub fn chosen(&self) -> Option<usize> {
        let value = self.value()?;
        choices::chosen(&self.choices(), &value)
    }

    /// The label it shows: its choice's.
    pub fn shown(&self) -> String {
        let choices = self.choices();
        self.chosen().and_then(|at| choices.get(at)).map(|c| c.label.clone()).unwrap_or_default()
    }

    /// The placeholder it shows, in the muted color, while nothing is chosen.
    pub fn placeholder_shown(&self) -> Option<String> {
        if !self.shown().is_empty() {
            return None;
        }
        self.base.spec.text("placeholder").map(|p| self.base.host.text(Some(p))).filter(|p| !p.is_empty())
    }

    /// Its list's overlay id, while it is open.
    pub fn list(&self) -> Option<ElementId> {
        self.list.as_ref().map(|(id, _)| *id)
    }

    /// The choice the open list marks, which Enter picks.
    pub fn highlighted(&self) -> Option<usize> {
        self.list.as_ref().and_then(|(_, state)| state.borrow().highlighted)
    }

    fn usable(&self) -> bool {
        self.base.enabled() && !self.base.read_only() && self.base.bound().is_some()
    }

    /// Opens its list below it, its value's choice marked.
    pub fn open(&mut self, cx: &mut Cx) {
        if !self.usable() || self.list.is_some() {
            return;
        }
        let labels: Vec<String> = self.choices().into_iter().map(|c| c.label).collect();
        let bounds = self.base.node.bounds;
        let place = choice_list::below(cx.whole(), bounds, bounds.width, labels.len());
        let state: SharedList = Rc::new(std::cell::RefCell::new(ListState::new(labels, self.chosen())));
        let id = choice_list::open(cx, state.clone(), place, self.base.node.id());
        self.list = Some((id, state));
    }

    /// Closes its list.
    pub fn close(&mut self, cx: &mut Cx) {
        if let Some((id, _)) = self.list.take() {
            cx.close(id);
        }
        cx.invalidate();
    }

    /// The person picked the choice at `at`: a change when it differs from its value.
    pub fn pick(&mut self, at: usize, cx: &mut Cx) {
        let choices = self.choices();
        let Some(choice) = choices.get(at) else { return };
        let Some(path) = self.base.bound().map(str::to_string) else { return };
        if !self.usable() || self.value().is_some_and(|v| choices::same(&choice.value, &v)) {
            return;
        }
        self.base.host.write(self.base.name().unwrap_or(""), &path, choice.value.clone());
        self.base.fire("change");
        cx.invalidate();
    }

    /// Moves the mark of its open list `by` lines.
    fn move_by(&self, by: i64, cx: &mut Cx) {
        if let Some((_, state)) = self.list.as_ref() {
            state.borrow_mut().move_by(by);
            cx.invalidate();
        }
    }
}

impl Element for DropdownPart {
    part_element!();

    fn focusable(&self) -> bool {
        self.base.enabled()
    }

    /// A click on its box opens its list, or closes it when it is open.
    fn on_click(&mut self, cx: &mut Cx) {
        if self.list.is_none() {
            self.open(cx);
        } else {
            self.close(cx);
        }
    }

    fn on_key(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        if self.list.is_none() {
            if !self.usable() || key.control() {
                return false;
            }
            let alt = key.alt();
            if matches!(key.key, Key::Enter | Key::Space | Key::F4) || (alt && matches!(key.key, Key::Down | Key::Up)) {
                self.open(cx);
                return true;
            }
            if alt {
                return false;
            }
            let count = self.choices().len();
            if count == 0 {
                return false;
            }
            let now = self.chosen();
            let at = match key.key {
                Key::Down => now.map_or(0, |n| (n + 1).min(count - 1)),
                Key::Up => now.map_or(0, |n| n.saturating_sub(1)),
                Key::Home => 0,
                Key::End => count - 1,
                _ => return false,
            };
            self.pick(at, cx);
            return true;
        }
        let count = self.list.as_ref().map(|(_, s)| s.borrow().count() as i64).unwrap_or(0);
        match key.key {
            Key::Down => self.move_by(1, cx),
            Key::Up => self.move_by(-1, cx),
            Key::PageDown => self.move_by(MOST_SHOWN as i64, cx),
            Key::PageUp => self.move_by(-(MOST_SHOWN as i64), cx),
            Key::Home => self.move_by(-count, cx),
            Key::End => self.move_by(count, cx),
            Key::Enter | Key::Space => {
                let at = self.highlighted();
                self.close(cx);
                if let Some(at) = at {
                    self.pick(at, cx);
                }
            }
            Key::Escape => self.close(cx),
            _ => return false,
        }
        true
    }

    /// A typed letter: the next choice whose label starts with it, picked when the list is closed, marked when it is open.
    fn on_text(&mut self, text: &str, cx: &mut Cx) {
        let typed = text.trim().to_lowercase();
        if !self.usable() || typed.is_empty() {
            return;
        }
        let choices = self.choices();
        let from = match self.list.as_ref() {
            Some((_, state)) => state.borrow().highlighted,
            None => self.chosen(),
        };
        let count = choices.len();
        for step in 1..=count {
            let at = (from.map_or(count - 1, |f| f) + step) % count;
            if !choices[at].label.to_lowercase().starts_with(&typed) {
                continue;
            }
            match self.list.as_ref() {
                Some((_, state)) => {
                    let mut s = state.borrow_mut();
                    s.highlighted = Some(at);
                    s.reveal();
                    cx.invalidate();
                }
                None => self.pick(at, cx),
            }
            return;
        }
    }

    fn on_focus_changed(&mut self, focused: bool, cx: &mut Cx) {
        // Tab moved the focus on; a press in its own list takes the focus from it for a moment and gives it back.
        if !focused && cx.focused().is_some() {
            self.close(cx);
        }
        cx.invalidate();
    }

    fn on_overlay_closed(&mut self, overlay: ElementId, cx: &mut Cx) {
        let Some((id, state)) = self.list.take() else { return };
        if id != overlay {
            self.list = Some((id, state));
            return;
        }
        let picked = state.borrow().picked;
        if let Some(at) = picked {
            self.pick(at, cx);
        }
        cx.invalidate();
    }

    fn on_detached(&mut self, cx: &mut Cx) {
        self.close(cx);
    }

    fn draw(&self, p: &Painter) {
        let look = p.look;
        let node = &self.base.node;
        let b = node.bounds;
        let open = self.list.is_some();
        let values = look.part("input", if node.focused || open { Some("focus") } else { None });
        let radius = values.radius.map(|r| r as f64).unwrap_or_else(|| look.radius("input"));
        let enabled = self.base.enabled();
        let _dimmed = p.dimmed(enabled);
        let error = self.base.state().and_then(|s| s.error.clone()).is_some_and(|e| !e.show().is_empty());
        let line = if error {
            look.color("error")
        } else if node.hovered && enabled {
            values.border.unwrap_or_else(|| look.color("strong_border"))
        } else {
            values.border.unwrap_or_else(|| look.color("border"))
        };
        p.fill(b, values.background.unwrap_or_else(|| look.color("input")), radius);
        p.stroke(b, line, look.border(), radius);
        if node.focused {
            p.focus_ring(b.inset(-2.0), radius + 2.0);
        }
        let pad = look.padding() * 2.0;
        let text = Rect { x: b.x + pad, width: (b.width - 2.0 * pad - 18.0).max(0.0), ..b };
        let color = values.text.unwrap_or_else(|| look.color("text"));
        let size = values.size.map(|s| s as f64).unwrap_or_else(|| look.size("text"));
        let font = values.font.clone().unwrap_or_else(|| "text".to_string());
        match self.placeholder_shown() {
            Some(placeholder) => {
                let shown = p.fit(&placeholder, text.width, size, &font, false);
                p.text(&shown, text, look.color("muted"), size, &font, false, TextAlign::Left);
            }
            None => {
                let shown = p.fit(&self.shown(), text.width, size, &font, values.bold());
                p.text(&shown, text, color, size, &font, values.bold(), TextAlign::Left);
            }
        }
        p.down_arrow(Rect::new(b.right() - pad - 18.0, b.y, 18.0, b.height), color);
    }
}

impl Part for DropdownPart {
    fn base(&self) -> &PartBase {
        &self.base
    }
}
