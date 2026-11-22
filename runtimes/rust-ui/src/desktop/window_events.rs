//! What the system says about a window: the pointer, the wheel, keys, an
//! input method's text and place, resizing, the focus and closing.

use super::*;

impl Host {
    /// What the system says of a window, and what the person did on it.
    pub(super) fn on_window(&mut self, event_loop: &ActiveEventLoop, i: usize, event: WindowEvent) {
        // The screen reader's adapter hears every event first (the window's focus and place).
        let win = &mut self.wins[i];
        if let (Some(reader), Some(shown)) = (win.reader.as_mut(), win.shown.as_ref()) {
            reader.adapter.process_event(&shown.window, &event);
        }
        // Where the pointer is, kept also while a dialog over the window takes its input.
        match &event {
            WindowEvent::CursorEntered { .. } => {
                self.wins[i].inside = true;
                self.wins[i].pointer.entered();
            }
            WindowEvent::CursorLeft { .. } => self.wins[i].inside = false,
            WindowEvent::Focused(true) => {
                self.wins[i].pointer.focused();
                let key = self.wins[i].key;
                self.set_active(Some(key));
            }
            WindowEvent::CursorMoved { position, .. } => {
                let (x, y) = self.at(i, position.x, position.y);
                self.wins[i].pointer.told(x, y);
            }
            _ => {}
        }
        if self.blocked(i) && (is_input(&event) || matches!(event, WindowEvent::CloseRequested)) {
            // A window with a dialog over it (its own or a file dialog) stays, and takes nothing.
            return;
        }
        match event {
            WindowEvent::CloseRequested => {
                let key = self.wins[i].key;
                // The program's main window closing ends the program.
                if self.wins[i].owner.is_none() && self.wins.iter().filter(|w| w.owner.is_none()).count() == 1 {
                    let owned: Vec<u64> = self.wins.iter().map(|w| w.key).collect();
                    for key in owned.into_iter().rev() {
                        self.close(key);
                    }
                    event_loop.exit();
                    return;
                }
                self.close(key);
            }
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => self.fit(i),
            WindowEvent::ThemeChanged(theme) => self.follow_theme(Some(theme)),
            WindowEvent::RedrawRequested => {
                self.draw(i);
                self.tell_reader(i);
            }
            WindowEvent::ModifiersChanged(modifiers) => self.wins[i].held = modifiers_of(modifiers.state()),
            WindowEvent::Focused(false) => {
                let win = &mut self.wins[i];
                win.held = Modifiers::NONE;
                // Losing the focus mid-press (Alt+Tab during a drag) drops the drag.
                win.screen.pointer_cancelled();
            }
            WindowEvent::CursorMoved { .. } => {
                let win = &mut self.wins[i];
                let (x, y) = win.pointer.at();
                win.screen.pointer_moved(x, y);
            }
            WindowEvent::CursorLeft { .. } => self.wins[i].screen.pointer_left(),
            WindowEvent::MouseInput { state, button, .. } => {
                let Some(button) = button_of(button) else { return };
                let elapsed = self.now();
                let win = &mut self.wins[i];
                // A move the system left untold went where the pointer last moved.
                if let Some((x, y)) = win.pointer.settle() {
                    win.screen.pointer_moved(x, y);
                }
                let (x, y) = win.pointer.at();
                match state {
                    // The window gives only presses: a double or triple click is counted here.
                    ElementState::Pressed => {
                        let clicks = win.presses.press(x, y, button, elapsed);
                        win.screen.pointer_pressed_with(x, y, button, clicks, win.held);
                    }
                    ElementState::Released => win.screen.pointer_released(x, y, button),
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let per_unit = self.wins[i].shown.as_ref().map(Shown::per_unit).unwrap_or(1.0);
                let (dx, dy) = wheel_of(delta, per_unit);
                let win = &mut self.wins[i];
                if let Some((x, y)) = win.pointer.settle() {
                    win.screen.pointer_moved(x, y);
                }
                let (x, y) = win.pointer.at();
                win.screen.wheel(x, y, dx, dy, win.held);
            }
            // Keys the system repeats for a key held as the window got the focus are not the person's.
            WindowEvent::KeyboardInput { event, is_synthetic: false, .. } => {
                let win = &mut self.wins[i];
                let key = KeyInput::new(key_of(&event.logical_key, event.physical_key), win.held);
                match event.state {
                    ElementState::Pressed => {
                        let _ = win.screen.key_pressed(key);
                        if let Some(text) = typed_text(event.text.as_deref(), win.held) {
                            win.screen.text_typed(&text);
                        }
                    }
                    ElementState::Released => win.screen.key_released(key),
                }
            }
            // An input method composes text, then commits it as typed text.
            WindowEvent::Ime(ime) => {
                let screen = &mut self.wins[i].screen;
                match ime {
                    Ime::Preedit(text, cursor) if !text.is_empty() => screen.compose(Some(Composition { text, marked: cursor })),
                    Ime::Preedit(..) | Ime::Disabled => screen.compose(None),
                    Ime::Commit(text) => {
                        screen.compose(None);
                        screen.text_typed(&text);
                    }
                    Ime::Enabled => {}
                }
            }
            _ => {}
        }
    }

    /// The input method's window lies by the caret of the part that types.
    pub(super) fn place_ime(&mut self) {
        for win in self.wins.iter_mut() {
            let caret = win.screen.caret_area();
            if caret == win.caret {
                continue;
            }
            win.caret = caret;
            if let (Some(caret), Some(shown)) = (caret, &win.shown) {
                shown.window.set_ime_cursor_area(LogicalPosition::new(caret.x, caret.y), LogicalSize::new(caret.width.max(1.0), caret.height.max(1.0)));
            }
        }
    }

    /// The pointer takes the system's shape of what is under it.
    pub(super) fn shape_pointers(&mut self) {
        for win in self.wins.iter_mut() {
            if win.screen.take_cursor_changed() {
                if let Some(shown) = &win.shown {
                    shown.window.set_cursor(standard_of(win.screen.cursor()));
                }
            }
        }
    }
}
