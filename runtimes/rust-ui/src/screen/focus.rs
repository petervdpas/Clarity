//! Focus and keys: the focus and the tab order, the window's keys, keys and
//! text typed to the focused part, and an input method's text and caret.

use super::*;

impl Screen {
    pub(super) fn focus_now(&mut self, id: Option<ElementId>, cx: &mut Cx) {
        let id = id.filter(|i| self.element(*i).is_some());
        if id == self.focused {
            return;
        }
        let was = self.focused;
        self.set_flag(was, |n| n.focused = false);
        self.focused = id;
        self.set_flag(id, |n| n.focused = true);
        if let Some(e) = was.and_then(|w| self.element_mut(w)) {
            element::safely(e, cx, (), |e, cx| e.on_focus_changed(false, cx));
        }
        if let Some(e) = id.and_then(|i| self.element_mut(i)) {
            element::safely(e, cx, (), |e, cx| e.on_focus_changed(true, cx));
        }
        let pointing = self.pointing;
        element::safely(self.root.as_mut(), cx, (), |root, cx| root.focus_moved(id, pointing, cx));
        self.redraw = true;
    }

    /// Gives the focus to the element of that id, or takes it away.
    pub fn focus(&mut self, id: Option<ElementId>) {
        let mut cx = self.cx();
        cx.focus(id);
        self.settle(cx);
    }

    /// The elements Tab reaches, in Tab order: in the dialog on top, else in
    /// what lies over the window that has the focus (a slide-out), else in the window.
    pub fn tab_order(&self) -> Vec<ElementId> {
        match self.overlays.last() {
            Some(top) if top.modal => element::focus_takers(top.content.as_ref()),
            _ => {
                let holding = self.focused.and_then(|f| self.overlays.iter().rev().find(|o| element::find(o.content.as_ref(), f).is_some()));
                let inside = holding.map(|o| element::focus_takers(o.content.as_ref())).filter(|t| !t.is_empty());
                inside.unwrap_or_else(|| element::focus_takers(self.root.as_ref()))
            }
        }
    }

    /// The focus goes to the next element in Tab order, or the one before.
    pub fn move_focus(&mut self, back: bool) {
        let takers = self.tab_order();
        if takers.is_empty() {
            return;
        }
        let at = self.focused.and_then(|f| takers.iter().position(|t| *t == f));
        let next = if back {
            match at {
                Some(i) if i > 0 => i - 1,
                _ => takers.len() - 1,
            }
        } else {
            at.map(|i| (i + 1) % takers.len()).unwrap_or(0)
        };
        self.focus(Some(takers[next]));
    }

    /// The window's own keys (its hook, then the root's): a button's
    /// shortcut, Enter for the default button, Escape for the cancel
    /// button. Not asked while a dialog is on top: its keys are its own.
    pub(super) fn window_keys(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        if self.modal_on_top() {
            return false;
        }
        let hooked = self.keys.as_mut().is_some_and(|keys| keys(key, cx));
        hooked || element::safely(self.root.as_mut(), cx, false, |root, cx| root.window_key(key, cx))
    }

    /// The part with the focus, or the first part it is in: in the overlays, top first, then in the window.
    /// Found once per element that has the focus; looked for again while it is in no part yet.
    pub(super) fn focused_part(&self) -> Option<FocusedPart> {
        let focused = self.focused;
        let (at, known) = self.focus_part.borrow().clone();
        if at == focused && (known.is_some() || focused.is_none()) {
            return known;
        }
        let found = focused.and_then(|f| {
            let roots = self.overlays.iter().rev().map(|o| o.content.as_ref()).chain([self.root.as_ref()]);
            roots.into_iter().find_map(|root| crate::window::focused_part_in(root, f))
        });
        *self.focus_part.borrow_mut() = (focused, found.clone());
        found
    }

    /// The focused element, when the person may use it.
    pub(super) fn focused_enabled(&self) -> Option<ElementId> {
        self.focused.filter(|f| self.element(*f).is_some_and(element::enabled_in_tree))
    }

    /// A key pressed: a dialog on top hears it first, then the part with the
    /// focus (a text box keeps Ctrl+A and its arrows), then the window's own
    /// keys. True when something on the screen took it.
    pub fn key_pressed(&mut self, key: KeyInput) -> bool {
        self.hide_tip();
        self.last_press = Some(super::Press::Key(key));
        self.mark();
        self.key_taken = true;
        let mut cx = self.cx();
        let top_modal = self.modal_on_top();
        // A dialog on top hears its keys before the part with the focus.
        if top_modal {
            let took = match self.overlays.last_mut() {
                Some(top) => element::safely(top.content.as_mut(), &mut cx, false, |c, cx| c.dialog_key(key, cx)),
                None => false,
            };
            if took {
                cx.invalidate();
                self.settle(cx);
                return true;
            }
        }
        let focused = self.focused_enabled();
        let focused_key = |screen: &mut Screen, cx: &mut Cx| match focused.and_then(|f| screen.element_mut(f)) {
            Some(e) => element::safely(e, cx, false, |e, cx| e.on_key(key, cx)),
            None => false,
        };
        // Escape closes the overlay on top, unless the part with the focus takes it (a list that closes itself).
        // One that stays (a pinned slide-out) closes only while the focus is in it.
        let focus_in = |o: &Overlay| focused.is_some_and(|f| element::find(o.content.as_ref(), f).is_some());
        let closing = self.overlays.iter().rev().find(|o| !o.staying() || focus_in(o)).map(Overlay::id);
        if key.key == Key::Escape && closing.is_some() && !top_modal {
            if !focused_key(self, &mut cx) {
                if let Some(top) = closing {
                    cx.close(top);
                }
            }
            self.settle(cx);
            return true;
        }
        if key.key == Key::Tab && !key.control() && !key.alt() {
            // A part that takes Tab itself but lets it go (past a table's last cell) has the focus move on as Tab does.
            let wants_tab = focused.and_then(|f| self.element(f)).is_some_and(|e| e.wants_tab());
            if wants_tab && focused_key(self, &mut cx) {
                self.settle(cx);
                return true;
            }
            self.settle(cx);
            self.move_focus(key.shift());
            return !self.tab_order().is_empty();
        }
        if focused_key(self, &mut cx) {
            cx.invalidate();
            self.settle(cx);
            return true;
        }
        if self.window_keys(key, &mut cx) {
            cx.invalidate();
            self.settle(cx);
            return true;
        }
        self.settle(cx);
        self.key_taken = false;
        false
    }

    /// A key let go: the window hears it first (Alt for its menu bar), then the part with the focus.
    pub fn key_released(&mut self, key: KeyInput) {
        let mut cx = self.cx();
        let top_modal = self.modal_on_top();
        if !top_modal && element::safely(self.root.as_mut(), &mut cx, false, |root, cx| root.window_key_up(key, cx)) {
            cx.invalidate();
            self.settle(cx);
            return;
        }
        if let Some(e) = self.focused.and_then(|f| self.element_mut(f)) {
            element::safely(e, &mut cx, (), |e, cx| e.on_key_up(key, cx));
        }
        self.settle(cx);
    }

    /// Text typed: after a key nothing took, a single character is first the
    /// window's (a character shortcut such as `*`, matched by what the
    /// keyboard typed, not where its key is), else the focused part's.
    pub fn text_typed(&mut self, text: &str) {
        self.mark();
        let mut cx = self.cx();
        let mut chars = text.chars();
        let single = match (chars.next(), chars.next()) {
            (Some(c), None) if !c.is_control() => Some(c),
            _ => None,
        };
        let shortcut = !self.key_taken && single.is_some_and(|c| self.window_keys(KeyInput::typed(c), &mut cx));
        self.key_taken = false;
        if shortcut {
            cx.invalidate();
            self.settle(cx);
            return;
        }
        if let Some(e) = self.focused_enabled().and_then(|f| self.element_mut(f)) {
            element::safely(e, &mut cx, (), |e, cx| e.on_text(text, cx));
        }
        self.settle(cx);
    }

    /// An input method composes text (none: it stopped): the part with the
    /// focus shows it at its caret. What it commits comes as `text_typed`.
    pub fn compose(&mut self, composing: Option<Composition>) {
        let mut cx = self.cx();
        if let Some(e) = self.focused.and_then(|f| self.element_mut(f)) {
            e.on_compose(composing, &mut cx);
        }
        cx.invalidate();
        self.settle(cx);
    }

    /// Where the caret of the part with the focus is, in the screen's units:
    /// an input method shows its window by it; none when no part types.
    pub fn caret_area(&self) -> Option<Rect> {
        self.focused.and_then(|f| self.element(f)).and_then(|e| e.caret_area())
    }

    /// What the person typed in the part with the focus is taken, as when they leave it.
    pub fn take_typed(&mut self) {
        let focused = self.focused;
        let mut cx = self.cx();
        let tree: &mut dyn Element = match self.overlays.iter_mut().rev().find(|o| focused.is_some_and(|f| element::find(o.content.as_ref(), f).is_some())) {
            Some(overlay) => overlay.content.as_mut(),
            None => self.root.as_mut(),
        };
        element::take_typed_from(tree, focused, &mut cx);
        self.settle(cx);
    }
}
