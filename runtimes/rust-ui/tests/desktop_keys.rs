// The desktop host's input without a window: the system's keys as Clarity.UI's
// keys (symbols from the keyboard or the number pad alike), the keys held,
// typed text, and the wheel in the screen's units, as the C# host has them.

#![cfg(feature = "desktop")]

use clarity_ui::desktop::input::{key_of, modifiers_of, typed_text, wheel_of, LINE};
use clarity_ui::{Key, Modifiers};
use winit::dpi::PhysicalPosition;
use winit::event::MouseScrollDelta;
use winit::keyboard::{Key as Logical, KeyCode, ModifiersState, NamedKey, PhysicalKey, SmolStr};

fn typed(text: &str, code: KeyCode) -> Key {
    key_of(&Logical::Character(SmolStr::new(text)), PhysicalKey::Code(code))
}

fn named(name: NamedKey, code: KeyCode) -> Key {
    key_of(&Logical::Named(name), PhysicalKey::Code(code))
}

#[test]
fn letters_and_digits_are_what_the_keyboard_writes() {
    assert_eq!(typed("a", KeyCode::KeyA), Key::A);
    assert_eq!(typed("Z", KeyCode::KeyZ), Key::Z);
    // A French keyboard writes a on the key where q lies: the shortcut is the letter written.
    assert_eq!(typed("a", KeyCode::KeyQ), Key::A);
    assert_eq!(typed("7", KeyCode::Digit7), Key::D7);
    // A key that writes no Latin letter is the key where it lies.
    assert_eq!(typed("\u{444}", KeyCode::KeyA), Key::A);
    // A digit key with Shift writes a symbol it has no key of: it stays the digit.
    assert_eq!(typed("!", KeyCode::Digit1), Key::D1);
    assert_eq!(typed("&", KeyCode::Digit1), Key::D1);
}

#[test]
fn symbols_come_from_the_keyboard_or_the_number_pad_alike() {
    // Shift with = writes +: that is Plus, as the pad's + is.
    assert_eq!(typed("+", KeyCode::Equal), Key::Plus);
    assert_eq!(typed("=", KeyCode::Equal), Key::Equal);
    assert_eq!(typed("+", KeyCode::NumpadAdd), Key::Plus);
    assert_eq!(typed("-", KeyCode::Minus), Key::Minus);
    assert_eq!(typed("-", KeyCode::NumpadSubtract), Key::Minus);
    assert_eq!(typed("*", KeyCode::NumpadMultiply), Key::Multiply);
    assert_eq!(typed("/", KeyCode::NumpadDivide), Key::Slash);
    assert_eq!(typed(".", KeyCode::NumpadDecimal), Key::Period);
    assert_eq!(typed(",", KeyCode::Comma), Key::Comma);
    assert_eq!(typed(";", KeyCode::Semicolon), Key::Semicolon);
    assert_eq!(typed("'", KeyCode::Quote), Key::Quote);
    assert_eq!(typed("\\", KeyCode::Backslash), Key::Backslash);
    assert_eq!(typed("[", KeyCode::BracketLeft), Key::BracketLeft);
    assert_eq!(typed("]", KeyCode::BracketRight), Key::BracketRight);
    assert_eq!(typed("`", KeyCode::Backquote), Key::Backquote);
    // The pad's digits are the digits.
    assert_eq!(typed("4", KeyCode::Numpad4), Key::D4);
    // A symbol key with Shift writes what has no key of its own: the key where it lies.
    assert_eq!(typed(":", KeyCode::Semicolon), Key::Semicolon);
}

#[test]
fn named_keys_and_the_pad_without_num_lock() {
    assert_eq!(named(NamedKey::Enter, KeyCode::Enter), Key::Enter);
    assert_eq!(named(NamedKey::Enter, KeyCode::NumpadEnter), Key::Enter);
    assert_eq!(named(NamedKey::Tab, KeyCode::Tab), Key::Tab);
    assert_eq!(named(NamedKey::Escape, KeyCode::Escape), Key::Escape);
    assert_eq!(named(NamedKey::Space, KeyCode::Space), Key::Space);
    assert_eq!(typed(" ", KeyCode::Space), Key::Space);
    assert_eq!(named(NamedKey::Backspace, KeyCode::Backspace), Key::Backspace);
    assert_eq!(named(NamedKey::Delete, KeyCode::Delete), Key::Delete);
    assert_eq!(named(NamedKey::Insert, KeyCode::Insert), Key::Insert);
    assert_eq!(named(NamedKey::ArrowLeft, KeyCode::Numpad4), Key::Left);
    assert_eq!(named(NamedKey::ArrowRight, KeyCode::ArrowRight), Key::Right);
    assert_eq!(named(NamedKey::ArrowUp, KeyCode::ArrowUp), Key::Up);
    assert_eq!(named(NamedKey::ArrowDown, KeyCode::ArrowDown), Key::Down);
    assert_eq!(named(NamedKey::Home, KeyCode::Home), Key::Home);
    assert_eq!(named(NamedKey::End, KeyCode::End), Key::End);
    assert_eq!(named(NamedKey::PageUp, KeyCode::PageUp), Key::PageUp);
    assert_eq!(named(NamedKey::PageDown, KeyCode::PageDown), Key::PageDown);
    assert_eq!(named(NamedKey::F1, KeyCode::F1), Key::F1);
    assert_eq!(named(NamedKey::F12, KeyCode::F12), Key::F12);
    assert_eq!(named(NamedKey::Shift, KeyCode::ShiftRight), Key::Shift);
    assert_eq!(named(NamedKey::Control, KeyCode::ControlLeft), Key::Control);
    assert_eq!(named(NamedKey::Alt, KeyCode::AltLeft), Key::Alt);
    assert_eq!(named(NamedKey::Super, KeyCode::SuperLeft), Key::Other);
    // What the system names nothing is the key where it lies.
    assert_eq!(key_of(&Logical::Dead(None), PhysicalKey::Code(KeyCode::Quote)), Key::Quote);
    assert_eq!(key_of(&Logical::Dead(None), PhysicalKey::Code(KeyCode::F5)), Key::F5);
    assert_eq!(key_of(&Logical::Dead(None), PhysicalKey::Code(KeyCode::NumpadAdd)), Key::Plus);
    assert_eq!(key_of(&Logical::Dead(None), PhysicalKey::Code(KeyCode::CapsLock)), Key::Other);
}

#[test]
fn the_keys_held() {
    assert_eq!(modifiers_of(ModifiersState::empty()), Modifiers::NONE);
    let all = modifiers_of(ModifiersState::SHIFT | ModifiersState::CONTROL | ModifiersState::ALT | ModifiersState::SUPER);
    assert_eq!(all, Modifiers::SHIFT | Modifiers::CONTROL | Modifiers::ALT | Modifiers::META);
    assert_eq!(modifiers_of(ModifiersState::CONTROL), Modifiers::CONTROL);
}

#[test]
fn typed_text_leaves_out_control_characters_and_shortcuts() {
    assert_eq!(typed_text(Some("a"), Modifiers::NONE).as_deref(), Some("a"));
    assert_eq!(typed_text(Some("A"), Modifiers::SHIFT).as_deref(), Some("A"));
    assert_eq!(typed_text(Some("\u{e9}"), Modifiers::NONE).as_deref(), Some("\u{e9}"));
    assert_eq!(typed_text(None, Modifiers::NONE), None);
    // Enter, Tab, Backspace and Escape write control characters: they are keys, not text.
    for control in ["\r", "\t", "\u{8}", "\u{1b}", "\u{7f}"] {
        assert_eq!(typed_text(Some(control), Modifiers::NONE), None);
    }
    // With Ctrl, Alt or Meta held a key is a shortcut, not text.
    assert_eq!(typed_text(Some("s"), Modifiers::CONTROL), None);
    assert_eq!(typed_text(Some("s"), Modifiers::ALT), None);
    assert_eq!(typed_text(Some("s"), Modifiers::META), None);
    // A control character inside what was typed goes, the rest stays.
    assert_eq!(typed_text(Some("a\u{1}b"), Modifiers::NONE).as_deref(), Some("ab"));
}

#[test]
fn a_notch_of_the_wheel_is_forty_units_and_down_shows_further_down() {
    assert_eq!(LINE, 40.0);
    // The wheel turned toward the person (down): what is further down shows.
    assert_eq!(wheel_of(MouseScrollDelta::LineDelta(0.0, -1.0), 1.0), (0.0, 40.0));
    assert_eq!(wheel_of(MouseScrollDelta::LineDelta(0.0, 2.0), 2.0), (0.0, -80.0));
    assert_eq!(wheel_of(MouseScrollDelta::LineDelta(-1.0, 0.0), 1.0), (40.0, 0.0));
    // A touchpad gives pixels of the system's: at scale 2 they are half as many of the screen's units.
    assert_eq!(wheel_of(MouseScrollDelta::PixelDelta(PhysicalPosition::new(10.0, -30.0)), 2.0), (-5.0, 15.0));
    assert_eq!(wheel_of(MouseScrollDelta::PixelDelta(PhysicalPosition::new(0.0, 12.0)), 0.0), (0.0, -12.0));
}
