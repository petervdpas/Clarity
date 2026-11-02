//! A shortcut as Clarity writes it against a key the person pressed, as
//! every stack matches one; symbol keys from the keyboard or the number pad alike.

use clarity_ui::input::{Key, KeyInput, Modifiers};
use clarity_ui::shortcuts::{key_of, matches, parts};

fn key(key: Key, modifiers: Modifiers) -> KeyInput {
    KeyInput::new(key, modifiers)
}

#[test]
fn modifiers_must_be_exactly_those_written() {
    assert!(matches("Ctrl+S", &key(Key::S, Modifiers::CONTROL), false));
    assert!(!matches("Ctrl+S", &key(Key::S, Modifiers::CONTROL | Modifiers::SHIFT), false));
    assert!(matches("Ctrl+Shift+N", &key(Key::N, Modifiers::CONTROL | Modifiers::SHIFT), false));
    assert!(!matches("Ctrl+S", &key(Key::S, Modifiers::NONE), false));
    assert!(matches("ctrl+s", &key(Key::S, Modifiers::CONTROL), false));
    assert!(matches("Alt+F4", &key(Key::F4, Modifiers::ALT), false));
    assert!(matches("Cmd+Q", &key(Key::Q, Modifiers::META), false));
}

#[test]
fn named_keys_and_their_other_names_are_read() {
    assert_eq!(Key::Escape, key_of("Esc"));
    assert_eq!(Key::Enter, key_of("Return"));
    assert_eq!(Key::Delete, key_of("Del"));
    assert_eq!(Key::PageDown, key_of("PgDn"));
    assert_eq!(Key::F12, key_of("F12"));
    assert_eq!(Key::D7, key_of("7"));
    assert_eq!(Key::Other, key_of("F13"));
    assert_eq!(Key::Other, key_of("Gizmo"));
}

#[test]
fn a_plus_is_its_own_key_after_the_modifiers() {
    assert_eq!((vec!["Ctrl"], "+"), parts("Ctrl++"));
    assert_eq!((Vec::<&str>::new(), "+"), parts("+"));
    assert_eq!(Key::Plus, key_of("+"));
    assert!(matches("Ctrl++", &key(Key::Plus, Modifiers::CONTROL), false));
    // Shift makes a + on most keyboards: a symbol does not ask for it unless written.
    assert!(matches("Ctrl++", &key(Key::Plus, Modifiers::CONTROL | Modifiers::SHIFT), false));
    assert!(matches("Ctrl+-", &key(Key::Minus, Modifiers::CONTROL), false));
    assert!(matches("Ctrl+*", &key(Key::Multiply, Modifiers::CONTROL), false));
}

#[test]
fn a_typed_key_is_left_to_an_input_the_person_types_in() {
    assert!(matches("A", &key(Key::A, Modifiers::NONE), false));
    assert!(!matches("A", &key(Key::A, Modifiers::NONE), true));
    assert!(!matches("+", &key(Key::Plus, Modifiers::NONE), true));
    assert!(matches("Ctrl+A", &key(Key::A, Modifiers::CONTROL), true));
    // A key that types nothing still answers in an input.
    assert!(matches("F2", &key(Key::F2, Modifiers::NONE), true));
}
