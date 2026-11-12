//! The window stack's part of signing in (docs/APP-CAPABILITIES-PLAN.md
//! step 4): the sign-in window shows before the main window, in a desktop
//! window of its own on the desktop (or the splash that is it); a sign-in
//! closes it and opens the main window; a signout closes every window and
//! shows it again.

use std::rc::Rc;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use serde_json::{Map, Value};

use super::host::ProviderSignin;
use super::*;

impl WindowStack {
    /// People sign in first: the sign-in window shows, before the app's
    /// `on start` and its main window. Fail safe: a sign-in window that does
    /// not open ends the program, saying why, rather than showing the rest.
    pub(super) fn show_signin(&mut self, page: String, cx: Option<&mut Cx>) {
        self.open_window(&page, Map::new(), cx);
        self.signin = Some(page);
        if self.stack.is_empty() {
            self.ended = true;
        }
    }

    /// Closes every window and dialog of this stack, each hearing its `on
    /// close`, without ending it: for signing in and out.
    pub(super) fn close_all(&mut self, cx: &mut Cx) {
        while !self.dialogs.is_empty() {
            let dialog = self.dialogs.pop().expect("a dialog");
            let _ = dialog.host.closing();
            cx.close(dialog.overlay);
        }
        while let Some(top) = self.stack.pop() {
            let _ = top.host.closing();
        }
        if let Some(index) = self.view_index() {
            self.node.remove(index, cx);
        }
        cx.focus(None);
    }

    /// Someone signed in: the sign-in window closes, the app's `on start` runs and its main window opens.
    pub(super) fn signed_in(&mut self, cx: &mut Cx) {
        self.close_all(cx);
        // The sign-in window in a desktop window of its own: the main window gets one of its own, and the sign-in window ends.
        if self.own_signin_window && self.running.apart.get() {
            let reply = self.running.program.start_app();
            self.aparts.push(Apart::Main(reply));
            self.ended = true;
            cx.invalidate();
            return;
        }
        self.start_main(Some(cx));
        if self.stack.is_empty() {
            self.ended = true;
        }
        cx.invalidate();
    }

    /// The person signed out: every window closes and the sign-in window
    /// shows again; a program that names none starts over as it began.
    pub(super) fn signed_out(&mut self, cx: &mut Cx) {
        self.close_all(cx);
        let page = self.signin.clone().or_else(|| self.running.program.started().signin);
        // On the desktop the sign-in window shows as at the start, in a desktop window of its own.
        if let Some(page) = page.as_ref().filter(|_| self.running.apart.get()) {
            self.aparts.push(Apart::Signin(page.clone()));
            self.ended = true;
            cx.invalidate();
            return;
        }
        match page {
            Some(page) => self.show_signin(page, Some(cx)),
            None => self.start_main(Some(cx)),
        }
        if self.stack.is_empty() {
            self.ended = true;
        }
        cx.invalidate();
    }

    /// The splash `name` that is the sign-in window too: it shows while `on
    /// start` runs, then stays, its parts working, until someone signed in.
    pub fn splash_signing_in(running: Rc<Running>, name: &str) -> WindowStack {
        let mut stack = WindowStack::splash(running, name);
        stack.splash_signs_in = true;
        stack.signin = Some(name.to_string());
        stack
    }

    /// The sign-in window `name` in a desktop window of its own, at the start
    /// or after a signout (also the splash that is the sign-in window): no
    /// start runs and it stays; a sign-in on it runs `on start` and asks for
    /// the main window (`Apart::Main`). A window that does not open ends it.
    pub fn signin_window(running: Rc<Running>, name: &str) -> WindowStack {
        let mut stack = WindowStack::splash(running, name);
        stack.signin = Some(name.to_string());
        stack.own_signin_window = true;
        if stack.stack.is_empty() {
            stack.ended = true;
        }
        stack
    }

    /// A sign-in with a provider: the browser goes to the provider off the
    /// screen's thread, the bar of a long call over the window, its Stop
    /// ending the wait; what the browser brings back comes to `provider_done`.
    pub(super) fn start_provider(&mut self, host: Rc<ProgramHost>, signin: ProviderSignin, cx: &mut Cx) {
        if self.long.is_some() {
            return;
        }
        let stop = Arc::new(AtomicBool::new(false));
        let bar = cx.open(Overlay::dialog(Box::new(LongCallBar::new(Arc::new(std::sync::Mutex::new(None)), stop.clone()))));
        self.running.calling.set(true);
        self.long = Some((host, bar));
        let program = self.running.files.title.to_string();
        cx.later(self.node.id(), move || {
            let brought = through_browser(&signin, &program, &stop);
            ProviderDone { signin, brought }
        });
    }

    /// The browser came back from the provider (or the wait ended): the bar
    /// goes and the button's view signs in with what it brought.
    pub(super) fn provider_done(&mut self, done: ProviderDone, cx: &mut Cx) {
        self.running.calling.set(false);
        if let Some((host, bar)) = self.long.take() {
            cx.close(bar);
            host.provider_answered(&done.signin, done.brought);
        }
        cx.invalidate();
    }
}

/// What a sign-in with a provider gives back on the screen's thread.
pub struct ProviderDone {
    signin: ProviderSignin,
    brought: Result<Map<String, Value>, String>,
}

/// The browser's way to the provider and back, on the desktop.
#[cfg(not(target_os = "emscripten"))]
fn through_browser(signin: &ProviderSignin, program: &str, stop: &AtomicBool) -> Result<Map<String, Value>, String> {
    clarity_auth::providers::through_browser(&signin.strategy, &signin.options, program, stop)
}

/// In a browser the served program signs in with a provider by its own pages, not this screen.
#[cfg(target_os = "emscripten")]
fn through_browser(signin: &ProviderSignin, _program: &str, _stop: &AtomicBool) -> Result<Map<String, Value>, String> {
    Err(format!("signing in with {} is not offered in this screen", signin.strategy))
}
