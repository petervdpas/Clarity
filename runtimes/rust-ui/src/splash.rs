//! An application's splash screen (docs/APP-ICON-SPLASH-PLAN.md, step 3):
//! the page the app names shows first while the app's `on start` runs (on
//! the desktop in a borderless window in the middle of the screen, in the
//! browser over the whole page); it says what loads as code sets
//! `splash.status`, and finishes once `on start` answered and the time the
//! app gives is up; then the main window shows. These are the rules both hosts follow.

use std::time::Duration;

/// How often the screen asks what loads while the splash shows.
pub const STATUS_EVERY: Duration = Duration::from_millis(150);

/// Where a window of `size` lies in the middle of a screen at `at` of `room`, all in the screen's pixels.
pub fn centred(at: (i32, i32), room: (u32, u32), size: (u32, u32)) -> (i32, i32) {
    let middle = |at: i32, room: u32, size: u32| at + (room as i32 - size as i32).max(0) / 2;
    (middle(at.0, room.0, size.0), middle(at.1, room.1, size.1))
}

/// Whether the splash finishes: `on start` answered and the splash showed at least as long as asked.
pub fn closes(answered: bool, shown_for: Duration, at_least: Duration) -> bool {
    answered && shown_for >= at_least
}

/// When the splash is next to be looked at: a status asked for, or the time it may close.
pub fn next_look(shown_for: Duration, at_least: Duration, answered: bool) -> Duration {
    if answered { at_least.saturating_sub(shown_for).min(STATUS_EVERY) } else { STATUS_EVERY }
}
