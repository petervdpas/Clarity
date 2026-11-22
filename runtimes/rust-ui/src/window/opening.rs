//! Opening and closing windows: a window in place on top of the one before,
//! a dialog over it, a window apart in a desktop window of its own, and what
//! shows in the window below as one closes.

use super::*;

impl WindowStack {
    /// Opens window `name` on top: the program opens it (its `on open` runs), then it shows.
    pub(super) fn open_window(&mut self, name: &str, with: Map<String, Value>, cx: Option<&mut Cx>) {
        let reply = match self.running.program.open_window(name, with) {
            Ok(reply) => reply,
            Err(text) => {
                self.early.push(Effect::Say(vec![Message { level: Level::Error, text, toast: None }]));
                return;
            }
        };
        let host = self.host_for(name, reply);
        let view = view_of(self.running.files, &host, name);
        // The window below is parked while this one is on top.
        if let Some(index) = self.view_index() {
            let mut below = self.node.children_mut().remove(index);
            if let Some(cx) = cx {
                element::detach_all(below.as_mut(), cx);
                cx.focus(None);
            }
            if let Some(top) = self.stack.last_mut() {
                top.parked = Some(below);
            }
        }
        self.node.children_mut().insert(0, Box::new(view));
        self.stack.push(Shown { host, parked: None });
    }

    /// Opens a window a window asked for: a dialog, or one a route marked
    /// `in new window` opens (`new_window`), in a desktop window of its own
    /// where the host gives them; a dialog over the window, else in place.
    pub(super) fn open(&mut self, name: &str, with: Map<String, Value>, from: Option<&Rc<ProgramHost>>, asked: Option<Asked>, new_window: bool, cx: &mut Cx) {
        let dialog = self.running.facts.is_dialog(name) || asked.is_some();
        let opener = from.or(self.current()).map(|h| h.instance());
        if self.running.apart.get() && (dialog || new_window) {
            self.aparts.push(Apart::Window(ApartWindow { window: name.to_string(), with, opener: opener.unwrap_or(0), asked, dialog }));
            return;
        }
        if dialog {
            self.show_dialog(name, with, opener, asked, cx);
        } else {
            self.open_window(name, with, Some(cx));
        }
        cx.invalidate();
    }

    /// A dialog over the window, sized to it, its keys its own; what it says shows in the window under it.
    pub(super) fn show_dialog(&mut self, name: &str, with: Map<String, Value>, opener: Option<u64>, asked: Option<Asked>, cx: &mut Cx) {
        let reply = match self.running.program.open_window(name, with) {
            Ok(reply) => reply,
            Err(text) => {
                self.said.push(Message { level: Level::Error, text, toast: None });
                return;
            }
        };
        let host = self.host_for(name, reply);
        let (files, made_for, window) = (self.running.files, host.clone(), name.to_string());
        let make = Box::new(move || view_of(files, &made_for, &window));
        let size = self.running.facts.window(name).and_then(|w| w.size);
        let content = DialogOverlay::new(self.running.title_of(name), size, make).with_host(host.clone());
        let first = content.first();
        let overlay = cx.open(Overlay { opener: Some(self.node.id()), ..Overlay::dialog(Box::new(content)) });
        cx.focus(first);
        self.dialogs.push(Dialog { host, overlay, opener, asked });
    }

    /// The dialog closes: its `on close` runs, the window that opened it (or
    /// the one on top) is told, and what it closed with goes to the handler
    /// that asked for it.
    pub(super) fn close_dialog(&mut self, index: usize, cx: &mut Cx) {
        let dialog = self.dialogs.remove(index);
        let effects = dialog.host.closing();
        let result = dialog.host.take_result();
        cx.close(dialog.overlay);
        let below = dialog.opener.and_then(|o| self.host_of(o)).or_else(|| self.current().cloned());
        if let Some(below) = &below {
            below.returned();
        }
        self.take(below.clone(), effects, cx);
        if let (Some(asked), Some(result), Some(below)) = (dialog.asked, result, below) {
            let mut values = asked.values;
            values.push(result);
            below.answer(&asked.handler, values);
        }
        cx.invalidate();
    }

    /// Closes the window on top: its `on close` runs, the window below shows
    /// and is told; the last one ends the desktop window it is in.
    pub(super) fn close_top(&mut self, cx: &mut Cx) {
        let Some(top) = self.stack.pop() else { return };
        let effects = top.host.closing();
        let result = top.host.take_result();
        if let Some(index) = self.view_index() {
            self.node.remove(index, cx);
        }
        cx.focus(None);
        if self.stack.is_empty() {
            if self.unconnected.is_some() && self.reconnect(cx) {
                self.take(None, effects, cx);
                cx.invalidate();
                return;
            }
            if let Some((opener, asked)) = self.opener.take() {
                // What it said just before (a toast its handler said with the close) goes with it to its opener.
                let mut said = self.toasts();
                said.extend(effects.into_iter().flat_map(|e| if let Effect::Say(m) = e { m } else { Vec::new() }));
                self.closed = Some(ApartClosed { opener, result, asked, said });
            }
            self.ended = true;
            cx.invalidate();
            return;
        }
        if let Some(view) = self.stack.last_mut().and_then(|s| s.parked.take()) {
            self.node.children_mut().insert(0, view);
        }
        let below = self.stack.last().map(|s| s.host.clone());
        if let Some(below) = &below {
            below.returned();
        }
        self.take(below, effects, cx);
        cx.invalidate();
    }

    /// The desktop window it is in closes: its dialogs and the window on top
    /// hear their `on close`, and the window that opened it is told.
    pub fn closing(&mut self, cx: &mut Cx) {
        while !self.dialogs.is_empty() {
            self.close_dialog(self.dialogs.len() - 1, cx);
        }
        let Some(top) = self.stack.last() else { return };
        let effects = top.host.closing();
        if let Some((opener, asked)) = self.opener.take() {
            let said = effects.into_iter().flat_map(|e| if let Effect::Say(m) = e { m } else { Vec::new() }).collect();
            self.closed = Some(ApartClosed { opener, result: None, asked, said });
        }
        self.ended = true;
    }

    /// A desktop window of its own, which shows `request.window`; it shares what the program's windows share.
    pub fn apart(running: Rc<Running>, request: ApartWindow) -> WindowStack {
        let mut stack = WindowStack::empty(running);
        stack.opener = Some((request.opener, request.asked.clone()));
        stack.dialog = request.dialog;
        stack.open_window(&request.window, request.with, None);
        if stack.stack.is_empty() {
            stack.ended = true;
        }
        stack
    }

    /// A desktop window of its own that a window here opened closed: that
    /// window is told, sees what it said, and the handler that asked for its
    /// value runs with it.
    pub fn apart_closed(&mut self, closed: ApartClosed, cx: &mut Cx) {
        let Some(host) = self.host_of(closed.opener).or_else(|| self.current().cloned()) else { return };
        host.returned();
        let mut effects = Vec::new();
        if !closed.said.is_empty() {
            effects.push(Effect::Say(closed.said));
        }
        if let (Some(asked), Some(result)) = (closed.asked, closed.result) {
            let mut values = asked.values;
            values.push(result);
            host.answer(&asked.handler, values);
        }
        effects.extend(host.take_effects());
        self.take(Some(host), effects, cx);
        cx.invalidate();
    }

    /// What the host is asked to open in desktop windows of their own since the last time.
    pub fn take_aparts(&mut self) -> Vec<Apart> {
        std::mem::take(&mut self.aparts)
    }

    /// This desktop window of its own closed: what to tell the window that opened it; taken once.
    pub fn take_closed(&mut self) -> Option<ApartClosed> {
        self.closed.take()
    }

    /// Does what replies asked: messages show, windows open and close,
    /// questions are asked, text goes on the clipboard; `from` is the window whose reply it was.
    pub(super) fn take(&mut self, from: Option<Rc<ProgramHost>>, effects: Vec<Effect>, cx: &mut Cx) {
        for effect in effects {
            match effect {
                Effect::Say(messages) => {
                    for message in messages {
                        // A toast the project declares: its own lifespan and icon, whatever its level.
                        if let Some(toast) = &message.toast {
                            let ms = toast.lifespan.unwrap_or(messages::DEFAULT_TOAST_LIFESPAN_MS);
                            let until = cx.now() + std::time::Duration::from_millis(u64::from(ms));
                            let icon = toast.icon.as_deref().and_then(|name| from.as_ref().and_then(|host| crate::parts::PartHost::icon_path(host.as_ref(), name)));
                            self.toasts_mut().add_toast(message, until, icon, cx);
                        } else if message.level == Level::Info {
                            let until = cx.now() + messages::INFO_DURATION;
                            self.toasts_mut().add(message, until, cx);
                        } else {
                            self.said.push(message);
                        }
                    }
                }
                Effect::Open(window, with) => self.open(&window, with, from.as_ref(), None, false, cx),
                Effect::OpenNew(window, with) => self.open(&window, with, from.as_ref(), None, true, cx),
                Effect::Close => {
                    let dialog = from.as_ref().and_then(|f| self.dialogs.iter().position(|d| Rc::ptr_eq(&d.host, f)));
                    match dialog {
                        Some(index) => self.close_dialog(index, cx),
                        None => {
                            let top = self.current().cloned();
                            if from.is_none() || from.as_ref().zip(top.as_ref()).is_some_and(|(f, t)| Rc::ptr_eq(f, t)) {
                                self.close_top(cx);
                            }
                        }
                    }
                }
                Effect::Copy(text) => cx.clipboard().set_text(&text),
                Effect::Ask(asked) => match &asked.form {
                    AskForm::Window { window, with } => {
                        let what = Asked { handler: asked.handler.clone(), values: asked.values.clone() };
                        self.open(window, with.clone(), from.as_ref(), Some(what), false, cx);
                    }
                    _ => {
                        // A question the app's own start asks has no window to answer it.
                        if let Some(host) = from.clone().or_else(|| self.current().cloned()) {
                            self.asks.push((asked, host));
                        }
                    }
                },
                Effect::Keys => self.show_keys(cx),
                Effect::Quit => {
                    self.quitting = true;
                    self.ended = true;
                    cx.invalidate();
                }
                Effect::Call(call) => {
                    if let Some(host) = from.clone().or_else(|| self.current().cloned()) {
                        self.start_call(host, call, cx);
                    }
                }
                // On a splash that is the sign-in window, the main window follows once `on start` answered and the splash showed its time.
                Effect::SignedIn if self.splash_signs_in => {
                    self.signed = true;
                    self.follow_start(cx);
                }
                Effect::SignedIn => self.signed_in(cx),
                Effect::SignedOut => self.signed_out(cx),
                Effect::Provider(signin) => {
                    if let Some(host) = from.clone().or_else(|| self.current().cloned()) {
                        self.start_provider(host, signin, cx);
                    }
                }
            }
        }
        self.say_next(cx);
        self.ask_next(cx);
    }
}
