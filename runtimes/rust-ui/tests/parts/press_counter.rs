//! A host that gives only presses (the desktop's window) counts the clicks
//! itself: a press soon after and near the last one of the same button is the
//! next click of a double or triple click.

use clarity_ui::input::{PointerButton, PressCounter};

#[test]
fn presses_close_in_time_and_place_count_up() {
    let mut counter = PressCounter::new();
    assert_eq!(counter.press(10.0, 10.0, PointerButton::Left, 1000), 1);
    assert_eq!(counter.press(11.0, 12.0, PointerButton::Left, 1200), 2);
    assert_eq!(counter.press(11.0, 12.0, PointerButton::Left, 1400), 3);
}

#[test]
fn a_late_far_or_other_press_starts_again() {
    let mut counter = PressCounter::new();
    assert_eq!(counter.press(10.0, 10.0, PointerButton::Left, 1000), 1);
    assert_eq!(counter.press(10.0, 10.0, PointerButton::Left, 1700), 1);
    assert_eq!(counter.press(30.0, 10.0, PointerButton::Left, 1800), 1);
    assert_eq!(counter.press(30.0, 10.0, PointerButton::Right, 1900), 1);
}
