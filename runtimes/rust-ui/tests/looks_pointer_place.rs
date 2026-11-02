// Where the pointer is over a desktop window when X11 leaves a move untold.
// winit on X11 tells a move only when it differs from the last move it saw;
// the place it gives on entering a window or on its focus coming back does
// not count as one. So after a dialog of its own closes over the place the
// pointer last moved to in the main window, a move straight back there (a
// touch, a tablet, a warp) is not told, and the press went to where the
// dialog's button was. The place a press goes to is then where the pointer
// last moved, once the system says the pointer moved.

#![cfg(feature = "desktop")]

use clarity_ui::desktop::input::PointerPlace;

#[test]
fn a_move_is_where_the_pointer_is() {
    let mut place = PointerPlace::new(true);
    place.told(60.0, 74.0);
    place.raw_motion();
    assert_eq!(None, place.settle());
    assert_eq!((60.0, 74.0), place.at());
}

#[test]
fn a_move_back_to_where_it_last_moved_after_a_dialog_closed_goes_there() {
    let mut place = PointerPlace::new(true);
    // The press that opened the dialog, on the Note button.
    place.told(60.0, 74.0);
    // The dialog closed under the pointer: entering the main window gives where its Fine button was.
    place.entered();
    place.told(180.0, 231.0);
    assert_eq!((180.0, 231.0), place.at());
    // The pointer moves back onto Note; X11 tells no move, only that it moved.
    place.raw_motion();
    assert_eq!(Some((60.0, 74.0)), place.settle());
    assert_eq!((60.0, 74.0), place.at());
}

#[test]
fn without_a_move_the_place_given_on_entering_stays() {
    let mut place = PointerPlace::new(true);
    place.told(60.0, 74.0);
    place.entered();
    place.told(180.0, 231.0);
    assert_eq!(None, place.settle());
    assert_eq!((180.0, 231.0), place.at());
    // A move told afterwards is where it is, and is the new last move.
    place.raw_motion();
    place.told(300.0, 200.0);
    assert_eq!(None, place.settle());
    assert_eq!((300.0, 200.0), place.at());
}

#[test]
fn focus_coming_back_gives_a_place_as_entering_does() {
    let mut place = PointerPlace::new(true);
    place.told(60.0, 74.0);
    place.focused();
    place.told(180.0, 231.0);
    place.raw_motion();
    assert_eq!(Some((60.0, 74.0)), place.settle());
}

#[test]
fn where_the_window_system_tells_every_move_nothing_is_guessed() {
    let mut place = PointerPlace::new(false);
    place.told(60.0, 74.0);
    place.entered();
    place.told(180.0, 231.0);
    place.raw_motion();
    assert_eq!(None, place.settle());
    assert_eq!((180.0, 231.0), place.at());
}
