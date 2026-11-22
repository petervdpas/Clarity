//! How a program starts on the desktop: the splash and the sign-in windows
//! before its main window.

use super::*;

impl Host {
    /// While the splash shows: `on start` runs on a thread of its own once the
    /// splash is on the screen, what loads is asked for every little while,
    /// and once `on start` answered and its time is up the splash closes and
    /// the main window opens: the splash finishes before the rest shows.
    /// When to look again; none once there is nothing to wait for.
    pub(super) fn follow_start(&mut self) -> Option<Instant> {
        let starting = self.starting.as_mut()?;
        let opened = starting.opened?;
        let at = self.wins.iter().position(|w| w.splash)?;
        if !starting.asked {
            starting.asked = true;
            let program = starting.running.program.clone();
            let proxy = self.proxy.clone();
            std::thread::spawn(move || {
                // What fails in `on start` is said in the main window, as when it runs on the screen's thread.
                let reply = catch_unwind(AssertUnwindSafe(|| program.start_app())).unwrap_or_else(|payload| Err(Text::plain(panic_text(payload.as_ref()))));
                let _ = proxy.send_event(Wake::Started(reply));
            });
        }
        let now = Instant::now();
        let answered = starting.reply.is_some();
        if crate::splash::closes(answered, now - opened, starting.at_least) {
            let key = self.wins[at].key;
            let Some(starting) = self.starting.take() else { return None };
            if starting.signs_in {
                // It keeps the answer until someone signs in on it, then asks for the main window (`Apart::Main`).
                if let Some(reply) = starting.reply {
                    let win = &mut self.wins[at];
                    win.screen.act_root::<WindowStack, _>(move |stack, cx| stack.splash_started(reply, cx));
                    if let Some(shown) = &win.shown {
                        shown.window.request_redraw();
                    }
                }
                return None;
            }
            if let Some(reply) = starting.reply {
                // The main window is added before the splash goes, so the program never has no window of its own.
                self.open_main(starting.running, reply);
            }
            self.close(key);
            return None;
        }
        if now >= starting.next_status {
            starting.next_status = now + crate::splash::STATUS_EVERY;
            if let Some(text) = starting.running.program.splash_status() {
                let win = &mut self.wins[at];
                win.screen.act_root::<WindowStack, _>(|stack, _| stack.show_status(text));
                win.screen.invalidate();
                if let Some(shown) = &win.shown {
                    shown.window.request_redraw();
                }
            }
        }
        Some(now + crate::splash::next_look(now - opened, starting.at_least, answered))
    }
}
