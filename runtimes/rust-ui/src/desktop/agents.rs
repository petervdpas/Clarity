//! AI agents in a running desktop program (docs/ledger/app-mcp.md): listening
//! for them, and doing their work on the screen's thread.

use super::*;

impl Host {
    /// Listens for AI agents when the program's app offers them commands: their
    /// grants and calls come back to this loop as work (`Wake::Agent`).
    pub(super) fn listen_for_agents(&mut self, proxy: EventLoopProxy<Wake>) {
        let Some(program) = self.program() else { return };
        let proxy = Mutex::new(proxy);
        let post: crate::agents::Post = Arc::new(move |work| {
            if let Ok(proxy) = proxy.lock() {
                let _ = proxy.send_event(Wake::Agent(work));
            }
        });
        self.agents = crate::agents::listen(&program, post);
    }

    /// The program the windows run, from the first window that runs one.
    pub(super) fn program(&self) -> Option<Arc<dyn Program>> {
        self.wins.iter().find_map(|w| w.screen.find::<WindowStack>().map(|stack| stack.running().program.clone()))
    }

    /// An AI agent's work on this thread: a grant asked in Clarity's dialog over
    /// the window the person works in, or a call run, after which every window
    /// reads its values again.
    pub(super) fn agent_work(&mut self, work: crate::agents::AgentWork) {
        match work {
            crate::agents::AgentWork::Grant { asked, app, reply } => {
                let active = self.active.and_then(|k| self.index_by_key(k));
                let at = active.filter(|i| self.wins[*i].screen.root().as_any().is::<WindowStack>()).or_else(|| self.wins.iter().position(|w| w.screen.root().as_any().is::<WindowStack>()));
                let Some(i) = at else {
                    let _ = reply.send(clarity_app_mcp::Decision::Refuse);
                    return;
                };
                let (title, lines) = (asked.title(&app), asked.lines());
                let answer: Box<dyn FnOnce(crate::dialogs::GrantChoice)> = Box::new(move |choice| {
                    let _ = reply.send(crate::agents::decision(choice));
                });
                self.wins[i].screen.act_root::<WindowStack, _>(|stack, cx| stack.ask_grant(title, lines, answer, cx));
                if let Some(shown) = &self.wins[i].shown {
                    shown.window.focus_window();
                    shown.window.request_redraw();
                }
            }
            crate::agents::AgentWork::TurnedAway(toast) => {
                for win in self.wins.iter_mut() {
                    win.screen.act_root::<WindowStack, _>(|stack, cx| stack.agent_said(toast.clone(), cx));
                    win.screen.invalidate();
                    if let Some(shown) = &win.shown {
                        shown.window.request_redraw();
                    }
                }
            }
            crate::agents::AgentWork::Changed => {
                for win in self.wins.iter_mut() {
                    win.screen.invalidate();
                    if let Some(shown) = &win.shown {
                        shown.window.request_redraw();
                    }
                }
            }
            crate::agents::AgentWork::Call { line, said, change } => {
                let Some(program) = self.program() else { return };
                let ran = crate::agents::run_call(program.as_ref(), &line, &said);
                // A change shows in the open windows at once: each reads its data again, and says what the agent did.
                let toast = change.filter(|_| !ran.refused).map(|(agent, tool)| crate::agents::change_toast(&agent, &tool, &ran));
                for win in self.wins.iter_mut() {
                    if let Some(toast) = &toast {
                        win.screen.act_root::<WindowStack, _>(|stack, cx| stack.agent_changed(toast.clone(), ran.step, cx));
                    }
                    win.screen.invalidate();
                    if let Some(shown) = &win.shown {
                        shown.window.request_redraw();
                    }
                }
            }
        }
    }
}
