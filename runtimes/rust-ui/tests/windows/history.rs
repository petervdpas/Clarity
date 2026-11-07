//! Back and Forward (window/history.rs): the places the person was in a
//! window, noted once per event, gone back to and forward again.

use clarity_ui::window::history::{History, Place};

fn at(row: usize) -> Place {
    let mut place = Place::default();
    place.selected.insert("chapters".into(), row);
    place
}

#[test]
fn back_and_forward_go_through_the_places_noted() {
    let mut h = History::default();
    h.note(at(0));
    h.note(at(0));
    assert!(!h.can_go_back(), "the same place twice is one step");
    h.note(at(1));
    h.note(at(2));
    assert_eq!(Some(at(1)), h.back());
    // Where Back went settles, not a new step: Forward still goes to 2.
    h.note(at(1));
    assert_eq!(Some(at(0)), h.back());
    h.note(at(0));
    assert_eq!(None, h.back());
    assert_eq!(Some(at(1)), h.forward());
    h.note(at(1));
    // A new place after going Back forgets what was ahead.
    h.note(at(5));
    assert!(!h.can_go_forward());
    assert_eq!(Some(at(1)), h.back());
}
