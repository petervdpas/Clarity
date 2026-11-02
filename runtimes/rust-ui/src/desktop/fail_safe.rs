//! The desktop's fail safe (DESIGN 7.9): whatever goes wrong on the screen's
//! thread (what the person did, drawing, laying out on a resize, a sidebar
//! growing the window, time going on) is said as Clarity's error message in
//! the window, and the program goes on. A failure that comes again every
//! frame (a drawing that keeps failing) is said once, not sixty times a
//! second. A Rust failure is a panic: it is caught where it unwinds to the
//! host's loop. Running out of memory aborts the program in Rust; it is no
//! failure to go on from, as in C#.

use std::panic::{catch_unwind, AssertUnwindSafe};

/// How long the same failure is not said again, in milliseconds.
pub const QUIET: i64 = 5000;

/// Says each failure on the host's loop once while it repeats.
pub struct FailSafe {
    say: Box<dyn FnMut(&str)>,
    last_said: Option<String>,
    last_at: i64,
}

impl FailSafe {
    /// A fail safe that says a failure with `say`.
    pub fn new(say: impl FnMut(&str) + 'static) -> FailSafe {
        FailSafe { say: Box::new(say), last_said: None, last_at: i64::MIN }
    }

    /// Runs `action` at `now` (milliseconds); a failure is said (once while
    /// it repeats) and the program goes on. Its answer, or none when it failed.
    pub fn run<R>(&mut self, now: i64, action: impl FnOnce() -> R) -> Option<R> {
        match catch_unwind(AssertUnwindSafe(action)) {
            Ok(answer) => Some(answer),
            Err(payload) => {
                self.failed(&panic_text(payload.as_ref()), now);
                None
            }
        }
    }

    /// A failure `what` at `now`: said unless it is the one said last within its quiet time.
    pub fn failed(&mut self, what: &str, now: i64) {
        if self.last_said.as_deref() == Some(what) && now.saturating_sub(self.last_at) < QUIET {
            return;
        }
        self.last_said = Some(what.to_string());
        self.last_at = now;
        (self.say)(what);
    }
}

/// What a panic said: its message, or a plain word when it gave none.
pub fn panic_text(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(text) = payload.downcast_ref::<&str>() {
        text.to_string()
    } else if let Some(text) = payload.downcast_ref::<String>() {
        text.clone()
    } else {
        crate::locale::translate("desktopFailed", "something went wrong")
    }
}
