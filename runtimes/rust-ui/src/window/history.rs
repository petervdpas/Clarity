//! Where the person went in a window, for Back and Forward: each place is the
//! chosen row of its lists, tables and trees and the open section, tab or step
//! of its accordions, tabs and steps, noted once per event of the person.

use std::collections::BTreeMap;

/// How many places Back can go to; older ones are forgotten.
const KEPT: usize = 100;

/// A place in a window: what is chosen and opened in it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Place {
    /// The chosen row of each list, table and tree, by part name.
    pub selected: BTreeMap<String, usize>,
    /// The open section, tab or step of each accordion, tabs and steps, by part name.
    pub opened: BTreeMap<String, i64>,
}

/// The places behind and ahead of the one shown.
#[derive(Default)]
pub struct History {
    back: Vec<Place>,
    now: Option<Place>,
    forward: Vec<Place>,
    /// Back or Forward just went: the next place noted is where it went, settled, not a new step.
    went: bool,
}

impl History {
    /// The window is at `place` now, once an event of the person is done: a
    /// new place is a step Back goes to, and what was ahead is gone.
    pub fn note(&mut self, place: Place) {
        if self.went {
            self.went = false;
            self.now = Some(place);
            return;
        }
        if self.now.as_ref() == Some(&place) {
            return;
        }
        if let Some(now) = self.now.replace(place) {
            self.back.push(now);
            if self.back.len() > KEPT {
                self.back.remove(0);
            }
        }
        self.forward.clear();
    }

    /// The place before the one shown, which is now ahead; none at the first.
    pub fn back(&mut self) -> Option<Place> {
        let before = self.back.pop()?;
        if let Some(now) = self.now.replace(before.clone()) {
            self.forward.push(now);
        }
        self.went = true;
        Some(before)
    }

    /// The place ahead, which Back left; none when nothing is ahead.
    pub fn forward(&mut self) -> Option<Place> {
        let ahead = self.forward.pop()?;
        if let Some(now) = self.now.replace(ahead.clone()) {
            self.back.push(now);
        }
        self.went = true;
        Some(ahead)
    }

    pub fn can_go_back(&self) -> bool {
        !self.back.is_empty()
    }

    pub fn can_go_forward(&self) -> bool {
        !self.forward.is_empty()
    }
}
