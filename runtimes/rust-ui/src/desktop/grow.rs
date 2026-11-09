//! How a desktop window's size follows what it shows (DESIGN 7.9): a window
//! that names its size and opens in place grows the desktop window to it, as
//! far as the screen has room, and never makes it smaller than the person
//! left it. A sidebar that opens grows it by the room it takes, so the
//! window's own parts keep theirs. Sizes are in the screen's units.

use crate::element::Element;
use crate::window::{WindowStack, WindowView};

use super::STANDARD_SIZE;

/// What a grown window leaves of the screen's height, for the system's own bars.
pub const SCREEN_MARGIN: f64 = 64.0;

/// The size of a desktop window at `now` when a window that asks for
/// `wanted` opens in it: as large as it asks, never smaller than it is, and
/// no larger than `room` (the screen's) where that is known.
pub fn grown(now: (f64, f64), wanted: Option<(f64, f64)>, room: Option<(f64, f64)>) -> (f64, f64) {
    let Some(want) = wanted else { return now };
    let width = now.0.max(want.0);
    let height = now.1.max(want.1);
    match room {
        Some(room) => (now.0.max(width.min(room.0)), now.1.max(height.min(room.1))),
        None => (width, height),
    }
}

/// The size of a desktop window `now` when its sidebars went from `was`
/// wide to `width`: a sidebar that opens grows it by the room it takes, as
/// far as the screen has `room`; one that folds leaves it as it is.
pub fn sidebar_grown(now: (f64, f64), was: f64, width: f64, room: Option<(f64, f64)>) -> (f64, f64) {
    if width <= was {
        return now;
    }
    grown(now, Some((now.0 + width - was, now.1)), room)
}

/// A window view's sidebars as the desktop window last saw them: which
/// side's is open (shown and not folded) and how wide they are together.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sidebars {
    pub open: [bool; 2],
    pub width: f64,
}

/// A window view's sidebars now.
pub fn sidebars_of(view: &WindowView) -> Sidebars {
    let open = |left: bool| view.frame().sidebar_shows(left) && view.sidebar(left).is_some_and(|s| !s.collapsed());
    Sidebars { open: [open(true), open(false)], width: sidebars_width(view) }
}

/// The size of a desktop window `now` when its sidebars went from `was` to
/// `is`: it grows only when a sidebar opened, by the room that took. One
/// dragged wider takes its room from the window's own parts.
pub fn sidebars_grown(now: (f64, f64), was: &Sidebars, is: &Sidebars, room: Option<(f64, f64)>) -> (f64, f64) {
    let opened = (0..2).any(|i| is.open[i] && !was.open[i]);
    if !opened {
        return now;
    }
    sidebar_grown(now, was.width, is.width, room)
}

/// How wide a window view's sidebars are now: each shown one as wide as it wants (its strip while folded).
pub fn sidebars_width(view: &WindowView) -> f64 {
    [true, false].into_iter().filter(|left| view.frame().sidebar_shows(*left)).filter_map(|left| view.sidebar(left)).map(|s| s.wanted()).sum()
}

/// The room a screen of `pixels` gives a window, in the screen's units at `per_unit`, less the system's own bars.
pub fn room_of(pixels: (u32, u32), per_unit: f64) -> Option<(f64, f64)> {
    let per_unit = if per_unit > 0.0 { per_unit } else { 1.0 };
    (pixels.0 > 0 && pixels.1 > 0).then(|| (pixels.0 as f64 / per_unit, pixels.1 as f64 / per_unit - SCREEN_MARGIN))
}

/// The size a desktop window opens at for a program's stack: a dialog in a
/// window of its own as large as its content needs, else the size its window
/// names, else the standard 1100 by 760.
pub fn open_size(stack: &WindowStack) -> (f64, f64) {
    stack.wanted_size().unwrap_or(STANDARD_SIZE)
}

/// The size the window on top of a stack names; none when it names none.
pub fn top_size(stack: &WindowStack) -> Option<(f64, f64)> {
    let top = stack.current()?.window();
    stack.running().facts.window(&top).and_then(|w| w.size)
}

/// The part a dialog in a window of its own starts with the focus on: its first that takes the focus and shows.
pub fn first_focus(root: &dyn Element) -> Option<crate::element::ElementId> {
    crate::element::focus_takers(root).into_iter().next()
}
