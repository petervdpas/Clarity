//! The window host's part of signing in (docs/APP-CAPABILITIES-PLAN.md
//! step 4): a `signin <strategy>` click, by a sign-in button of a sign-in
//! view (the fields of its own view) or a hand-built one on the splash (the
//! parts named after the inputs); an "other way" showing another sign-in
//! view; `signout`; and the refusal a sign-in view shows.

use std::collections::BTreeMap;

use clarity_runtime::ui::Text;
use serde_json::{Map, Value};

use super::{Effect, ProgramHost};
use crate::markup::{HandlerCall, PartSpec};

/// The strategy a click target signs in with: `signin password`, or `signin(password)`.
fn signin_strategy(call: &HandlerCall) -> Option<String> {
    match (call.handler.as_str(), call.args.as_slice()) {
        ("signin", [strategy]) => Some(strategy.trim().to_string()),
        (handler, []) => handler.strip_prefix("signin ").map(|s| s.trim().to_string()).filter(|s| !s.is_empty() && !s.contains(' ')),
        _ => None,
    }
}

/// The view each part inside a content's views stands on, by the part's name.
pub(super) fn in_views(parts: &[PartSpec], into: &mut BTreeMap<String, String>) {
    fn all(parts: &[PartSpec], view: &str, into: &mut BTreeMap<String, String>) {
        for part in parts {
            if let Some(name) = part.name() {
                into.insert(name.to_string(), view.to_string());
            }
            all(&part.children, view, into);
            for page in &part.pages {
                all(&page.children, view, into);
            }
        }
    }
    for part in parts {
        for page in &part.pages {
            match &page.view {
                Some(view) => all(&page.children, view, into),
                None => in_views(&page.children, into),
            }
        }
        in_views(&part.children, into);
    }
}

impl ProgramHost {
    /// A click that signs in or out, or shows another sign-in view: what it
    /// did, true when it ran; none for any other click.
    pub(super) fn run_signin(&self, part: Option<&str>, call: &HandlerCall) -> Option<bool> {
        // Unless a window has that name.
        if !matches!(call.event.as_str(), "click" | crate::parts::NEW_WINDOW_CLICK) || self.files.window(&call.handler).is_some() {
            return None;
        }
        if let Some(strategy) = signin_strategy(call) {
            // A sign-in button of a sign-in view takes its view's fields; a hand-built one the parts named after the inputs.
            let outer = part.and_then(|p| p.strip_suffix(&format!(".{}", crate::parts::signin_view::BUTTON)));
            if let Some(button) = outer.filter(|p| self.specs.get(*p).is_some_and(|s| s.kind == "signinButton")) {
                // A provider signs in through the browser first, off the screen's thread.
                if let Some(options) = self.provider_options(&strategy) {
                    self.effects.borrow_mut().push(Effect::Provider(super::ProviderSignin { button: button.to_string(), strategy, options }));
                    self.changed();
                    return Some(true);
                }
                return Some(self.sign_in_view(button, &strategy, Map::new()));
            }
            return Some(self.sign_in_by_names(&strategy));
        }
        if let (crate::parts::signin_view::SHOW, [view]) = (call.handler.as_str(), call.args.as_slice()) {
            return Some(self.show_signin_view(view));
        }
        if call.handler == "signout" && call.args.is_empty() {
            self.flush();
            return Some(match self.program.signout() {
                Ok(()) => {
                    self.effects.borrow_mut().push(Effect::SignedOut);
                    self.changed();
                    true
                }
                Err(text) => self.answered(Err(text)),
            });
        }
        None
    }

    /// A provider strategy's settings for the browser part; none for a strategy that is no provider.
    fn provider_options(&self, strategy: &str) -> Option<Map<String, Value>> {
        let provider = clarity_auth::catalog().iter().any(|s| s.name == strategy && s.provider);
        let facts = self.facts.signin()?.strategies.iter().find(|s| s.name == strategy)?.provider.clone();
        provider.then_some(facts)
    }

    /// The browser came back from the provider of `signin`: the button's
    /// view signs in with what it brought, or shows why it did not.
    pub fn provider_answered(&self, signin: &super::ProviderSignin, brought: Result<Map<String, Value>, String>) -> bool {
        match brought {
            Ok(inputs) => self.sign_in_view(&signin.button, &signin.strategy, inputs),
            Err(why) => {
                *self.refusal.borrow_mut() = Some(Text::plain(why));
                self.changed();
                false
            }
        }
    }

    /// The way of signing in of sign-in view `view`: the strategy of its first button that takes typed inputs.
    pub(super) fn way_of(&self, view: &str) -> Option<String> {
        let typed = |strategy: &str| clarity_auth::catalog().iter().any(|s| s.name == strategy && !s.inputs.is_empty());
        self.specs.iter().filter(|(name, spec)| spec.kind == "signinButton" && self.views.get(*name).is_some_and(|v| v == view)).filter_map(|(_, spec)| spec.text("strategy")).find(|s| typed(s)).map(str::to_string)
    }

    /// Signs in with `strategy`, its inputs (as Clarity.Auth's catalog names
    /// them) the values of this window's parts of those names: a part bound
    /// to a field gives the field's value, an unbound one what was typed. A
    /// missing part gives no input, which the program refuses. A refusal
    /// shows as any refusal does.
    fn sign_in_by_names(&self, strategy: &str) -> bool {
        let names = clarity_auth::catalog().iter().find(|s| s.name == strategy).map(|s| s.inputs).unwrap_or_default();
        let mut inputs = Map::new();
        for name in names {
            if let Some(value) = self.input_value(name) {
                inputs.insert(name.to_string(), Value::String(value));
            }
        }
        match self.signed_in_with(strategy, inputs) {
            Ok(()) => true,
            Err(text) => self.answered(Err(text)),
        }
    }

    /// The one way a sign-in goes, by a `signin <strategy>` button or a
    /// sign-in part: the program checks `inputs`; on success the window
    /// closes and the main window opens (or the splash's rules hold), and a
    /// refusal is given back.
    pub(super) fn signed_in_with(&self, strategy: &str, inputs: Map<String, Value>) -> Result<(), Text> {
        self.flush();
        self.program.signin(strategy, inputs)?;
        self.effects.borrow_mut().push(Effect::SignedIn);
        self.changed();
        Ok(())
    }

    /// A sign-in button of a sign-in view signs in with `strategy`: its
    /// inputs what was typed in the sign-in fields of the button's view,
    /// `remember` whether its "keep me signed in" is ticked. A refusal shows
    /// in the view's refusal, else as any refusal does.
    fn sign_in_view(&self, button: &str, strategy: &str, given: Map<String, Value>) -> bool {
        let view = self.views.get(button);
        let wanted = clarity_auth::catalog().iter().find(|s| s.name == strategy).map(|s| s.inputs).unwrap_or_default();
        let mut inputs = given;
        let mut refusal_shown = false;
        {
            let state = self.state.borrow();
            use crate::parts::signin_view::{inside, BOX, FIELD};
            for (name, spec) in self.specs.iter().filter(|(name, _)| self.views.get(*name) == view) {
                match spec.kind.as_str() {
                    "signinField" => {
                        let typed = state.parts.get(&inside(name, FIELD));
                        if let Some(input) = spec.text("input").filter(|i| wanted.contains(i)) {
                            inputs.insert(input.to_string(), Value::String(typed.and_then(|p| p.text.clone()).unwrap_or_default()));
                        }
                    }
                    "signinRemember" if self.facts.signin().is_some_and(|s| s.remember_asks) => {
                        let ticked = state.parts.get(&inside(name, BOX)).and_then(|p| p.checked).unwrap_or(false);
                        inputs.insert("remember".into(), Value::String(ticked.to_string()));
                    }
                    "signinRefusal" => refusal_shown = true,
                    _ => {}
                }
            }
        }
        match self.signed_in_with(strategy, inputs) {
            Ok(()) => {
                *self.refusal.borrow_mut() = None;
                true
            }
            Err(text) if refusal_shown => {
                *self.refusal.borrow_mut() = Some(text);
                self.changed();
                true
            }
            Err(text) => self.answered(Err(text)),
        }
    }

    /// An "other way" shows sign-in view `view`: the content that holds it shows it, and the last refusal goes.
    fn show_signin_view(&self, view: &str) -> bool {
        let found = self.specs.iter().find_map(|(name, spec)| (spec.kind == "content").then(|| spec.pages.iter().position(|p| p.view.as_deref() == Some(view)).map(|i| (name.clone(), i))).flatten());
        let Some((content, at)) = found else { return false };
        *self.refusal.borrow_mut() = None;
        crate::parts::change_state(self, &content, |s| s.tab = Some(at as i64 + 1));
        true
    }

    /// The text of the part named `name`: its field's value when it is bound, else what was typed in it.
    fn input_value(&self, name: &str) -> Option<String> {
        let spec = self.specs.get(name)?;
        let state = self.state.borrow();
        match spec.text("bind").filter(|b| !b.is_empty()) {
            Some(path) => match super::super::paths::read(&state.data, path)? {
                Value::String(text) => Some(text),
                Value::Null => Some(String::new()),
                other => Some(other.to_string()),
            },
            None => Some(state.parts.get(name).and_then(|p| p.text.clone()).unwrap_or_default()),
        }
    }
}
