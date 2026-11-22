//! What an AI agent changed in a running program (docs/ledger/app-mcp.md):
//! the windows read again, what it said, and what it took back.

use super::*;

impl WindowStack {
    /// An AI agent changed the app's data (docs/ledger/app-mcp.md): the window on
    /// top reads it again, as when it is on top again (its `on return`), and a
    /// toast says what the agent did.
    /// `step` is the database step of the change, which the toast's Undo takes back; 0 for none.
    pub fn agent_changed(&mut self, toast: String, step: u64, cx: &mut Cx) {
        self.reread(cx);
        let action = (step > 0).then(|| {
            let (program, undone) = (self.running.program.clone(), self.agent_undone.clone());
            messages::ToastAction { word: crate::locale::translate("agentUndo", "Undo"), act: Rc::new(move |_cx: &mut Cx| *undone.borrow_mut() = Some(program.agent_undo(step))) }
        });
        // It goes by itself after a while, so a run of changes never fills the window.
        let until = cx.now() + messages::AGENT_TOAST_DURATION;
        self.toasts_mut().add_with(Message { level: Level::Info, text: Text::plain(toast), toast: None }, until, action, cx);
        cx.invalidate();
    }

    /// A toast about agents (an agent turned away), which goes by itself after a while.
    pub fn agent_said(&mut self, toast: String, cx: &mut Cx) {
        let until = cx.now() + messages::AGENT_TOAST_DURATION;
        self.toasts_mut().add(Message { level: Level::Info, text: Text::plain(toast), toast: None }, until, cx);
        cx.invalidate();
    }

    /// The window on top reads its data again, as when it is on top again (its `on return`).
    pub(super) fn reread(&mut self, cx: &mut Cx) {
        if let Some(host) = self.current().cloned() {
            host.returned();
            let effects = host.take_effects();
            if !effects.is_empty() {
                self.take(Some(host), effects, cx);
            }
        }
    }

    /// The Undo of an agent's change answered: the window reads its data again and says so, or says why not.
    pub(super) fn take_agent_undone(&mut self, cx: &mut Cx) {
        let Some(answered) = self.agent_undone.borrow_mut().take() else { return };
        match answered {
            Ok(()) => {
                self.reread(cx);
                let until = cx.now() + messages::INFO_DURATION;
                let said = crate::locale::translate("agentUndone", "The agent's change is undone.");
                self.toasts_mut().add(Message { level: Level::Info, text: Text::plain(said), toast: None }, until, cx);
            }
            Err(why) => {
                self.said.push(Message { level: Level::Error, text: why, toast: None });
                self.say_next(cx);
            }
        }
        cx.invalidate();
    }
}
