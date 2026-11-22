//! What a handler says and asks: its messages, its questions in Clarity's own
//! dialog, and the grants an agent asks the person for.

use super::*;

impl WindowStack {
    /// The next warning or error, in Clarity's dialog over the window, one at a time.
    pub(super) fn say_next(&mut self, cx: &mut Cx) {
        if self.saying.is_some() {
            return;
        }
        let Some(message) = self.said.first().cloned() else { return };
        let dialog = SaidDialog::new(message);
        let ok = dialog.ok();
        let overlay = Overlay { opener: Some(self.node.id()), ..Overlay::dialog(Box::new(dialog)) };
        self.saying = Some(cx.open(overlay));
        cx.focus(Some(ok));
    }

    /// The next question, in Clarity's own dialog over the window, one at a time.
    pub(super) fn ask_next(&mut self, cx: &mut Cx) {
        if self.asking.is_some() || self.asks.is_empty() {
            return;
        }
        let (asked, host) = self.asks.remove(0);
        let choices = ask::choices_of(&asked.form, &self.running.facts, self.running.program.as_ref());
        let dialog = AskDialog::new(asked.clone(), choices);
        let first = dialog.first();
        let answered = dialog.answered();
        let overlay = cx.open(Overlay { opener: Some(self.node.id()), ..Overlay::dialog(Box::new(dialog)) });
        cx.focus(Some(first));
        self.asking = Some(Asking { overlay, answered, ask: asked, host });
    }

    /// Asks the person to allow an AI agent's call, in Clarity's own dialog over
    /// the window, after the questions before it; `answer` gets the choice (a
    /// refusal when the dialog closed without one).
    pub fn ask_grant(&mut self, title: String, lines: Vec<String>, answer: Box<dyn FnOnce(crate::dialogs::GrantChoice)>, cx: &mut Cx) {
        self.grants.push(Granting { title, lines, answer });
        self.grant_next(cx);
        cx.invalidate();
    }

    pub(super) fn grant_next(&mut self, cx: &mut Cx) {
        if self.granting.is_some() || self.grants.is_empty() {
            return;
        }
        let next = self.grants.remove(0);
        let dialog = crate::dialogs::GrantDialog::new(next.title.clone(), next.lines.clone());
        let (first, granted) = (dialog.first(), dialog.granted());
        let overlay = cx.open(Overlay { opener: Some(self.node.id()), ..Overlay::dialog(Box::new(dialog)) });
        cx.focus(Some(first));
        self.granting = Some((overlay, granted, next));
    }

    /// Says something among the messages, as a handler's reply does.
    pub fn say(&mut self, level: Level, text: Text, cx: &mut Cx) {
        self.take(None, vec![Effect::Say(vec![Message { level, text, toast: None }])], cx);
    }
}
