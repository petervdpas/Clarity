//! A button (DESIGN 7.9): its click runs its handlers and opens the window
//! its `opens` names. Drawn in the theme's `button` look, with its hover and
//! pressed looks, a style over it, the ring of a default button, the focus
//! ring in the focus color and the dimmed look of a disabled part, as the
//! shared screen draws it. Its icon, when it has one, comes before its
//! caption, or alone in the middle. A split button has an arrow at its right
//! that opens its menu below it (as Down does); picking an item runs the
//! item's handler, and the rest of the button clicks as any button.

use std::rc::Rc;

use std::cell::RefCell;

use skia_safe::Color;

use crate::element::{Cx, Element, ElementId};
use crate::geometry::Rect;
use crate::input::{Key, KeyInput, PointerInput};
use crate::markup::{CommandSpec, PartSpec};
use crate::painter::{Corners, Painter, TextAlign};
use crate::part_element;
use crate::parts::containers::icon::draw_icon;
use crate::parts::inputs::choice_list::{self, ListState, SharedList};
use crate::parts::{Part, PartBase, PartHost};
use crate::theme::LookValues;

/// How large the icon of a button or toggle is drawn.
pub const ICON: f64 = 16.0;

/// How far an icon is from the caption after it.
const ICON_GAP: f64 = 6.0;

/// Draws a press part's icon (its SVG path) and caption in the middle of
/// `area`, inside its padding: the icon before the caption, or alone; the
/// caption cut off with an ellipsis where it does not fit.
#[allow(clippy::too_many_arguments)]
pub fn draw_label(p: &Painter, icon: Option<&str>, caption: &str, area: Rect, color: Color, size: f64, font: &str, bold: bool) {
    let inner = area.inset(p.look.padding());
    let Some(icon) = icon.filter(|i| !i.is_empty()) else {
        p.text(caption, inner, color, size, font, bold, TextAlign::Center);
        return;
    };
    let room = (inner.width - ICON - ICON_GAP).max(0.0);
    let shown = if caption.is_empty() { String::new() } else { p.fit(caption, room, size, font, bold) };
    let text = if shown.is_empty() { 0.0 } else { p.fonts.measure(&shown, font, size, bold) };
    let whole = ICON + if text > 0.0 { ICON_GAP + text } else { 0.0 };
    let x = (inner.x + (inner.width - whole) / 2.0).max(inner.x);
    draw_icon(p, Some(icon), Rect::new(x, area.y + (area.height - ICON) / 2.0, ICON, ICON), color);
    if text > 0.0 {
        let left = x + ICON + ICON_GAP;
        p.text(&shown, Rect::new(left, area.y, (inner.right() - left).max(0.0), area.height), color, size, font, bold, TextAlign::Left);
    }
}

/// The SVG path of the icon a part's spec names, as its host gives it; none without one.
pub fn icon_of(base: &PartBase) -> Option<String> {
    base.spec.text("icon").filter(|i| !i.is_empty()).and_then(|i| base.host.icon_path(i))
}

pub struct ButtonPart {
    base: PartBase,
    /// Its round corners: all, or only the outer ones in a button group.
    pub corners: Corners,
    /// Where the pointer is across, on the screen: over a split button's arrow a press opens its menu.
    pointer_x: f64,
    /// A split button's open menu: the list's id, the state it shares with the list, and the items it shows.
    menu: Option<(ElementId, SharedList, Vec<CommandSpec>)>,
}

pub fn make(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    Box::new(ButtonPart { base: PartBase::new(spec, host), corners: Corners::ALL, pointer_x: f64::MIN, menu: None })
}

impl ButtonPart {
    /// How wide a split button's arrow is.
    pub const ARROW_WIDTH: f64 = 24.0;

    /// Whether it is a split button: a button with an arrow that opens its menu.
    pub fn split(&self) -> bool {
        self.base.spec.flag("split")
    }

    /// Where a split button's arrow is.
    pub fn arrow(&self) -> Rect {
        let b = self.base.node.bounds;
        Rect::new(b.right() - Self::ARROW_WIDTH, b.y, Self::ARROW_WIDTH, b.height)
    }

    fn on_arrow(&self) -> bool {
        self.split() && self.pointer_x >= self.arrow().x
    }

    /// Whether a split button's menu is open.
    pub fn menu_open(&self) -> bool {
        self.menu.is_some()
    }

    /// Opens a split button's menu below it: its items that are no separator, by their captions.
    pub fn open_menu(&mut self, cx: &mut Cx) {
        if self.menu.is_some() || !self.split() || !self.base.enabled() {
            return;
        }
        let Some(spec) = self.base.spec.menu.as_ref() else { return };
        let items: Vec<CommandSpec> = spec.items.iter().filter(|i| i.text("kind") != Some("separator")).cloned().collect();
        if items.is_empty() {
            return;
        }
        let labels = items.iter().map(|i| self.base.host.text(i.text("caption"))).collect();
        let bounds = self.base.node.bounds;
        let place = choice_list::below(cx.whole(), bounds, bounds.width.max(160.0), items.len());
        let state: SharedList = Rc::new(RefCell::new(ListState::new(labels, None)));
        let id = choice_list::open(cx, state.clone(), place, self.base.node.id());
        self.menu = Some((id, state, items));
        cx.invalidate();
    }

    fn close_menu(&mut self, cx: &mut Cx) {
        if let Some((id, _, _)) = self.menu.take() {
            cx.close(id);
            cx.invalidate();
        }
    }

    /// The person picked the item at `at` of the open menu: the menu closes and the item runs.
    fn pick_menu(&mut self, at: Option<usize>, cx: &mut Cx) {
        let Some((id, _, items)) = self.menu.take() else { return };
        cx.close(id);
        cx.invalidate();
        if let Some(item) = at.and_then(|at| items.get(at)) {
            self.base.pick(item);
        }
    }

    /// Draws a push of the button at `area`, with `corners`, and its caption on it (an arrow's push has none).
    fn draw_push(&self, p: &Painter, area: Rect, values: &LookValues, corners: Corners) {
        let radius = values.radius.map(|r| r as f64).unwrap_or_else(|| p.look.radius("button"));
        let fill = values.background.unwrap_or_else(|| p.look.color("raised"));
        let line = if self.base.spec.flag("default") { p.look.color("accent") } else { values.border.unwrap_or_else(|| p.look.color("border")) };
        p.plate(area, fill, line, p.look.border(), radius, corners);
    }

    /// Its caption as shown: what code set, else as the design wrote it, translated.
    pub fn caption(&self) -> String {
        self.base.caption().unwrap_or_default()
    }

    /// Clicked, as a click on it, Enter or Space, or its shortcut does: its handlers run and the window it opens opens.
    pub fn click(&mut self, cx: &mut Cx) {
        if self.base.enabled() {
            self.base.fire("click");
            cx.invalidate();
        }
    }

    /// How a push looks now: the theme's button look, its style over it, and the hover or pressed look over both.
    fn push_look(&self, p: &Painter, hover: bool, pressed: bool) -> LookValues {
        let state = if !self.base.enabled() || !(hover || pressed) {
            None
        } else if pressed {
            Some("pressed")
        } else {
            Some("hover")
        };
        p.look.pressable("button", self.base.spec.text("style"), state)
    }
}

impl Element for ButtonPart {
    part_element!();

    fn focusable(&self) -> bool {
        self.base.enabled()
    }

    fn on_click(&mut self, cx: &mut Cx) {
        if !self.base.enabled() {
            return;
        }
        if self.on_arrow() {
            if self.menu.is_some() {
                self.close_menu(cx);
            } else {
                self.open_menu(cx);
            }
        } else {
            self.click(cx);
        }
    }

    fn on_pointer_moved(&mut self, at: PointerInput, cx: &mut Cx) {
        if self.split() && self.pointer_x != at.screen_x {
            self.pointer_x = at.screen_x;
            cx.invalidate();
        }
    }

    fn on_pointer_pressed(&mut self, at: PointerInput, _cx: &mut Cx) {
        self.pointer_x = at.screen_x;
    }

    fn on_key(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        if !self.base.enabled() {
            return false;
        }
        if let Some((_, state, _)) = self.menu.as_ref() {
            let state = state.clone();
            match key.key {
                Key::Down => state.borrow_mut().move_by(1),
                Key::Up => state.borrow_mut().move_by(-1),
                Key::Enter | Key::Space => {
                    let at = state.borrow().highlighted;
                    self.pick_menu(at, cx);
                }
                Key::Escape => self.close_menu(cx),
                _ => return false,
            }
            cx.invalidate();
            return true;
        }
        if matches!(key.key, Key::Enter | Key::Space) {
            self.click(cx);
            return true;
        }
        if key.key == Key::Down && key.modifiers.is_empty() && self.split() {
            self.open_menu(cx);
            return true;
        }
        false
    }

    fn on_focus_changed(&mut self, focused: bool, cx: &mut Cx) {
        // Tab moved the focus on; a press in its own menu takes the focus from it for a moment and gives it back.
        if !focused && cx.focused().is_some() {
            self.close_menu(cx);
        }
        cx.invalidate();
    }

    fn on_overlay_closed(&mut self, overlay: ElementId, cx: &mut Cx) {
        let Some((id, state, items)) = self.menu.take() else { return };
        if id != overlay {
            self.menu = Some((id, state, items));
            return;
        }
        let picked = state.borrow().picked;
        cx.invalidate();
        if let Some(item) = picked.and_then(|at| items.get(at)) {
            self.base.pick(item);
        }
    }

    fn on_detached(&mut self, cx: &mut Cx) {
        self.close_menu(cx);
    }

    fn draw(&self, p: &Painter) {
        let node = &self.base.node;
        let bounds: Rect = node.bounds;
        let _dimmed = p.dimmed(self.base.enabled());
        let held = node.pressed && node.hovered;
        let icon = icon_of(&self.base);
        let caption = self.caption();
        let label = |p: &Painter, area: Rect, values: &LookValues| {
            let size = values.size.map(|s| s as f64).unwrap_or_else(|| p.look.size("text"));
            let font = values.font.clone().unwrap_or_else(|| "text".to_string());
            draw_label(p, icon.as_deref(), &caption, area, values.text.unwrap_or_else(|| p.look.color("text")), size, &font, values.bold());
        };
        if !self.split() {
            let values = self.push_look(p, node.hovered, held);
            self.draw_push(p, bounds, &values, self.corners);
            label(p, bounds, &values);
        } else {
            let on_arrow = self.on_arrow();
            let arrow = self.arrow();
            let main = Rect { width: (bounds.width - Self::ARROW_WIDTH).max(0.0), ..bounds };
            let main_look = self.push_look(p, node.hovered && !on_arrow, held && !on_arrow);
            self.draw_push(p, main, &main_look, Corners { left: self.corners.left, right: false });
            label(p, main, &main_look);
            let arrow_look = self.push_look(p, node.hovered && on_arrow, (held && on_arrow) || self.menu.is_some());
            self.draw_push(p, arrow, &arrow_look, Corners { left: false, right: self.corners.right });
            p.down_arrow(arrow, arrow_look.text.unwrap_or_else(|| p.look.color("text")));
        }
        if node.focused {
            let values = self.push_look(p, false, false);
            let radius = values.radius.map(|r| r as f64).unwrap_or_else(|| p.look.radius("button"));
            p.focus_ring(bounds, radius);
        }
    }
}

impl Part for ButtonPart {
    fn base(&self) -> &PartBase {
        &self.base
    }

    fn press(&mut self, cx: &mut Cx) {
        self.click(cx);
    }

    fn set_corners(&mut self, corners: crate::painter::Corners) {
        self.corners = corners;
    }

    fn draws_own_style(&self) -> bool {
        true
    }
}
