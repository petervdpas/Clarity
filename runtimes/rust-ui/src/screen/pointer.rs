//! Routing the pointer: what is under it and its shape, moving, pressing
//! (the context menu with the right button), releasing, a click and the wheel.

use super::*;

impl Screen {
    /// The deepest element at a point: in the top overlay that has it, else in the window.
    pub fn at(&self, x: f64, y: f64) -> Option<ElementId> {
        for overlay in self.overlays.iter().rev() {
            if let Some(inside) = element::at(overlay.content.as_ref(), x, y) {
                return Some(inside);
            }
            if overlay.modal {
                return None;
            }
        }
        element::at(self.root.as_ref(), x, y)
    }

    pub(super) fn input(e: &dyn Element, x: f64, y: f64, modifiers: Modifiers, clicks: u32) -> PointerInput {
        Self::input_of(e, x, y, modifiers, clicks, PointerButton::Left)
    }

    pub(super) fn input_of(e: &dyn Element, x: f64, y: f64, modifiers: Modifiers, clicks: u32, button: PointerButton) -> PointerInput {
        let b = e.node().bounds;
        PointerInput { screen_x: x, screen_y: y, x: x - b.x, y: y - b.y, modifiers, clicks, button }
    }

    /// The tree an element is in: the top overlay that holds it, else the window.
    pub(super) fn tree_of(&self, at: ElementId) -> &dyn Element {
        self.overlays.iter().rev().map(|o| o.content.as_ref()).find(|c| element::find(*c, at).is_some()).unwrap_or(self.root.as_ref())
    }

    /// The ids from the element of that id up to its tree's root, itself first.
    pub(super) fn upward(&self, at: ElementId) -> Vec<ElementId> {
        let mut ids = element::chain(self.tree_of(at), at).unwrap_or_default();
        ids.reverse();
        ids
    }

    /// The shape over a point: of what holds the pointer, else of the deepest
    /// element there or one it is in; a disabled one shows the plain pointer.
    pub(super) fn shape_at(&self, at: Option<ElementId>, x: f64, y: f64) -> Cursor {
        let Some(from) = self.pressed.or(at) else { return Cursor::Default };
        if !self.element(from).is_some_and(element::enabled_in_tree) {
            return Cursor::Default;
        }
        for id in self.upward(from) {
            if let Some(e) = self.element(id) {
                if let Some(shape) = element::safely_read(e, None, |e| e.cursor_at(x, y)) {
                    return shape;
                }
            }
        }
        Cursor::Default
    }

    pub(super) fn shape(&mut self, shape: Cursor) {
        if shape != self.cursor {
            self.cursor = shape;
            self.cursor_changed = true;
        }
    }

    pub fn pointer_moved(&mut self, x: f64, y: f64) {
        self.mark();
        let mut cx = self.cx();
        let held = self.held;
        if let Some(e) = self.pressed.and_then(|p| self.element_mut(p)) {
            let input = Self::input_of(e, x, y, Modifiers::NONE, 1, held);
            element::safely(e, &mut cx, (), |e, cx| e.on_pointer_moved(input, cx));
        }
        let at = self.at(x, y);
        if at.is_some() && at != self.pressed {
            if let Some(e) = at.and_then(|a| self.element_mut(a)).filter(|e| element::enabled_in_tree(&**e)) {
                let input = Self::input(e, x, y, Modifiers::NONE, 1);
                element::safely(e, &mut cx, (), |e, cx| e.on_pointer_moved(input, cx));
            }
        }
        let shape = self.shape_at(at, x, y);
        self.shape(shape);
        let tip = if self.pressed.is_none() { self.tip_of(at, x, y) } else { None };
        self.rest(tip, x, y);
        if at != self.hovered {
            let was = self.hovered;
            self.set_flag(was, |n| n.hovered = false);
            self.hovered = at;
            self.set_flag(at, |n| n.hovered = true);
            self.redraw = true;
        }
        self.settle(cx);
    }

    pub fn pointer_left(&mut self) {
        self.hide_tip();
        if self.pressed.is_none() {
            self.shape(Cursor::Default);
        }
        if let Some(was) = self.hovered.take() {
            self.set_flag(Some(was), |n| n.hovered = false);
            self.redraw = true;
        }
    }

    /// The deepest element at or above `at` that takes the focus; none when that one is disabled.
    pub(super) fn taker(&self, at: ElementId) -> Option<ElementId> {
        let found = self.upward(at).into_iter().find(|id| self.element(*id).is_some_and(|e| e.focusable()))?;
        self.element(found).is_some_and(element::enabled_in_tree).then_some(found)
    }

    pub fn pointer_pressed(&mut self, x: f64, y: f64, button: PointerButton, clicks: u32) {
        self.pointer_pressed_with(x, y, button, clicks, Modifiers::NONE);
    }

    pub fn pointer_pressed_with(&mut self, x: f64, y: f64, button: PointerButton, clicks: u32, modifiers: Modifiers) {
        self.hide_tip();
        if button == PointerButton::Left {
            self.last_press = Some(super::Press::At(x, y));
        }
        // A press outside the overlays closes them, top first, and goes no
        // further, unless the one closed lets it through (a dropdown's list).
        // One that stays (a pinned slide-out) is left open, and the press goes on.
        if let Some(top) = self.overlays.iter().rev().find(|o| !(o.staying() && !o.content.node().bounds.contains(x, y))) {
            let anchored = top.anchor.and_then(|a| self.element(a)).is_some_and(|a| a.node().bounds.contains(x, y));
            if !top.modal && !top.content.node().bounds.contains(x, y) && !top.place.contains(x, y) && !anchored {
                let (id, through) = (top.id(), top.through);
                self.close(id);
                if !through {
                    return;
                }
                return self.pointer_pressed_with(x, y, button, clicks, modifiers);
            }
        }
        self.pointer_moved(x, y);
        // A press starts no new pause: the tip of what was pressed shows again only once the pointer went elsewhere.
        self.tip_due = None;
        // What is disabled (or in a disabled part) takes no press, no focus and no menu.
        let Some(at) = self.at(x, y) else { return };
        if !self.element(at).is_some_and(element::enabled_in_tree) {
            return;
        }
        // A right press is a drag of a part that takes right drags (a canvas draws a line with it); else it opens the menu there.
        let right_drag = button == PointerButton::Right
            && self.element(at).is_some_and(|e| {
                let b = e.node().bounds;
                element::safely_read(e, false, |e| e.right_drags(x - b.x, y - b.y))
            });
        if button == PointerButton::Right && !right_drag {
            self.context_pressed(at, x, y);
            return;
        }
        if button != PointerButton::Left && !right_drag {
            return;
        }
        // A right drag holds the pointer until the right button goes up.
        if self.pressed.is_some() && self.held == PointerButton::Right {
            return;
        }
        self.held = button;
        // Focus by the pointer: what was pressed stays where it is (no scrolling into view), so the release lands on it.
        self.pointing = true;
        let mut cx = self.cx();
        cx.focus(self.taker(at));
        self.settle(cx);
        self.pointing = false;
        self.pressed = Some(at);
        self.set_flag(Some(at), |n| n.pressed = true);
        let mut cx = self.cx();
        if let Some(e) = self.element_mut(at) {
            let input = Self::input_of(e, x, y, modifiers, clicks.max(1), button);
            element::safely(e, &mut cx, (), |e, cx| e.on_pointer_pressed(input, cx));
        }
        self.settle(cx);
        self.redraw = true;
    }

    /// A right press: what takes the focus there takes it (a text box keeps
    /// its selection for Copy), then the element there, or one it is in,
    /// gives its menu, which opens at the pointer (a part's `menu`); a table,
    /// list or tree chooses the row there first, as a left press does.
    pub(super) fn context_pressed(&mut self, at: ElementId, x: f64, y: f64) {
        if let Some(taker) = self.taker(at) {
            self.focus(Some(taker));
        }
        // A table, list or tree chooses the row under the pointer first, menu or not.
        let rows = self.upward(at).into_iter().any(|id| {
            self.element(id).and_then(|e| e.as_part()).is_some_and(|p| matches!(p.base().spec.kind.as_str(), "table" | "list" | "tree" | "treeTable"))
        });
        if rows {
            self.pointer_pressed(x, y, PointerButton::Left, 1);
            self.pointer_released(x, y, PointerButton::Left);
        }
        let found = self.upward(at).into_iter().find_map(|id| {
            let e = self.element(id)?;
            let entries = element::safely_read(e, None, |e| e.context_menu_at(x, y)).filter(|entries| !entries.is_empty())?;
            Some((id, entries))
        });
        let Some((id, entries)) = found else { return };
        let mut cx = self.cx();
        self.context_menus.open((id, 0), entries, Rect::new(x, y - 1.0, 0.0, 0.0), None, &mut cx);
        self.settle(cx);
        self.redraw = true;
    }

    pub fn pointer_released(&mut self, x: f64, y: f64, button: PointerButton) {
        if button != self.held || self.pressed.is_none() {
            return;
        }
        if button == PointerButton::Right {
            return self.right_released(x, y);
        }
        let Some(was) = self.pressed.take() else { return };
        self.set_flag(Some(was), |n| n.pressed = false);
        let mut cx = self.cx();
        if let Some(e) = self.element_mut(was) {
            let input = Self::input(e, x, y, Modifiers::NONE, 1);
            element::safely(e, &mut cx, (), |e, cx| e.on_pointer_released(input, cx));
        }
        self.settle(cx);
        self.redraw = true;
        if self.at(x, y) == Some(was) {
            let mut cx = self.cx();
            if let Some(e) = self.element_mut(was) {
                element::safely(e, &mut cx, (), |e, cx| e.on_click(cx));
            }
            self.settle(cx);
        }
        self.mark();
        let shape = self.shape_at(self.at(x, y), x, y);
        self.shape(shape);
    }

    /// The right button went up on a right drag: the part used it, or it opens its menu where it was pressed.
    pub(super) fn right_released(&mut self, x: f64, y: f64) {
        self.held = PointerButton::Left;
        let Some(was) = self.pressed.take() else { return };
        self.set_flag(Some(was), |n| n.pressed = false);
        let mut cx = self.cx();
        let used = match self.element_mut(was) {
            Some(e) => {
                let input = Self::input_of(e, x, y, Modifiers::NONE, 1, PointerButton::Right);
                element::safely(e, &mut cx, true, |e, cx| e.on_right_released(input, cx))
            }
            None => true,
        };
        self.settle(cx);
        self.redraw = true;
        if !used {
            self.context_pressed(was, x, y);
        }
        self.mark();
        let shape = self.shape_at(self.at(x, y), x, y);
        self.shape(shape);
    }

    /// The pointer was taken away mid-press (the window lost the focus
    /// during a drag): what holds it lets go without a release and without a
    /// click, so a drag in progress is dropped, not committed.
    pub fn pointer_cancelled(&mut self) {
        self.held = PointerButton::Left;
        let Some(was) = self.pressed.take() else { return };
        self.set_flag(Some(was), |n| n.pressed = false);
        let mut cx = self.cx();
        if let Some(e) = self.element_mut(was) {
            element::safely(e, &mut cx, (), |e, cx| e.on_pointer_cancelled(cx));
        }
        self.settle(cx);
        self.redraw = true;
        self.shape(Cursor::Default);
    }

    /// A press and its release at a point: a click.
    pub fn click(&mut self, x: f64, y: f64) {
        self.pointer_pressed(x, y, PointerButton::Left, 1);
        self.pointer_released(x, y, PointerButton::Left);
    }

    /// The wheel turned at a point: the deepest element there that scrolls
    /// takes it. In the screen's units: a positive `dy` shows what is further
    /// down, a positive `dx` what is further right, with the keys `held`.
    /// Shift turns a wheel that only goes up and down sideways, as every
    /// system's scrolling does.
    pub fn wheel(&mut self, x: f64, y: f64, dx: f64, dy: f64, held: Modifiers) {
        self.hide_tip();
        let (dx, dy) = if held.has(Modifiers::SHIFT) && dx == 0.0 { (dy, 0.0) } else { (dx, dy) };
        let Some(at) = self.at(x, y) else { return };
        let ids = self.upward(at);
        let mut cx = self.cx();
        for id in ids {
            if let Some(e) = self.element_mut(id) {
                if element::safely(e, &mut cx, false, |e, cx| e.on_wheel(dx, dy, held, cx)) {
                    cx.invalidate();
                    break;
                }
            }
        }
        self.settle(cx);
    }
}
