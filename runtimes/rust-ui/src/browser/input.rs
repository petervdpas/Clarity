//! What the page gives, as Clarity.UI takes it: a keyboard event's `key`
//! and `code` as Clarity.UI's key, the keys held, a pointer's button, the
//! wheel in the screen's units, and the pointer's shape as CSS names it.
//! Plain functions, so they are tested without a browser.

use crate::input::{Cursor, Key, Modifiers, PointerButton};
pub use crate::input::WHEEL_NOTCH;

/// The keys held as the page sends them: Shift 1, Control 2, Alt 4, Meta 8.
pub fn modifiers_of(bits: u32) -> Modifiers {
    let mut held = Modifiers::NONE;
    for (bit, modifier) in [(1, Modifiers::SHIFT), (2, Modifiers::CONTROL), (4, Modifiers::ALT), (8, Modifiers::META)] {
        if bits & bit != 0 {
            held = held | modifier;
        }
    }
    held
}

/// A pointer event's `button`: 0 the main one, 1 the wheel, 2 the context one; none for the others.
pub fn button_of(button: i32) -> Option<PointerButton> {
    match button {
        0 => Some(PointerButton::Left),
        1 => Some(PointerButton::Middle),
        2 => Some(PointerButton::Right),
        _ => None,
    }
}

/// A wheel event's turn in the screen's units, as `Screen::wheel` takes it:
/// `mode` is its `deltaMode`, handled as the desktop handles its wheel:
/// 0 pixels, which are CSS pixels and so the screen's units already (the
/// desktop divides the system's pixels by its scale to get them); 1 lines,
/// a notch each (`WHEEL_NOTCH`, the desktop's); 2 pages, the canvas's
/// `page` width and height each. A positive `dy` shows what is further
/// down, as in the page (the desktop turns the system's sign to this).
pub fn wheel_of(dx: f64, dy: f64, mode: u32, page: (f64, f64)) -> (f64, f64) {
    match mode {
        1 => (dx * WHEEL_NOTCH, dy * WHEEL_NOTCH),
        2 => (dx * page.0, dy * page.1),
        _ => (dx, dy),
    }
}

/// Clarity.UI's key of a keyboard event: what the key writes (`key`) comes
/// first, as on the desktop, so a shortcut is the letter or symbol the
/// person sees on it; a key that writes nothing Clarity.UI has a key for is
/// the key where it lies (`code`).
pub fn key_of(key: &str, code: &str) -> Key {
    let written = named_key(key).unwrap_or_else(|| written_key(key));
    if written != Key::Other { written } else { code_key(code) }
}

/// The key of a named key value (`Enter`, `ArrowLeft`, `F5`).
fn named_key(key: &str) -> Option<Key> {
    let named = match key {
        "Tab" => Key::Tab,
        "Enter" => Key::Enter,
        "Escape" | "Esc" => Key::Escape,
        "Backspace" => Key::Backspace,
        "Delete" | "Del" => Key::Delete,
        "Insert" => Key::Insert,
        "ArrowLeft" | "Left" => Key::Left,
        "ArrowRight" | "Right" => Key::Right,
        "ArrowUp" | "Up" => Key::Up,
        "ArrowDown" | "Down" => Key::Down,
        "Home" => Key::Home,
        "End" => Key::End,
        "PageUp" => Key::PageUp,
        "PageDown" => Key::PageDown,
        "Shift" => Key::Shift,
        "Control" => Key::Control,
        "Alt" | "AltGraph" => Key::Alt,
        _ => {
            let n = key.strip_prefix('F').and_then(|n| n.parse::<usize>().ok()).filter(|n| (1..=12).contains(n))?;
            Key::FUNCTIONS[n - 1]
        }
    };
    Some(named)
}

/// The key of what a key writes: a Latin letter, a digit or a symbol; else none.
fn written_key(text: &str) -> Key {
    let mut chars = text.chars();
    let (Some(c), None) = (chars.next(), chars.next()) else { return Key::Other };
    match c.to_ascii_lowercase() {
        c @ 'a'..='z' => Key::LETTERS[(c as u8 - b'a') as usize],
        c @ '0'..='9' => Key::DIGITS[(c as u8 - b'0') as usize],
        ' ' => Key::Space,
        '+' => Key::Plus,
        '-' => Key::Minus,
        '*' => Key::Multiply,
        '/' => Key::Slash,
        '.' => Key::Period,
        ',' => Key::Comma,
        '=' => Key::Equal,
        ';' => Key::Semicolon,
        '\'' => Key::Quote,
        '\\' => Key::Backslash,
        '[' => Key::BracketLeft,
        ']' => Key::BracketRight,
        '`' => Key::Backquote,
        _ => Key::Other,
    }
}

/// The key where a key lies on a US keyboard (`KeyA`, `Digit1`, `Numpad1`), the pad's as the main keyboard's.
fn code_key(code: &str) -> Key {
    if let Some(letter) = code.strip_prefix("Key") {
        let key = written_key(letter);
        if (Key::A..=Key::Z).contains(&key) {
            return key;
        }
    }
    if let Some(digit) = code.strip_prefix("Digit").or_else(|| code.strip_prefix("Numpad")) {
        let key = written_key(digit);
        if (Key::D0..=Key::D9).contains(&key) {
            return key;
        }
    }
    match code {
        "NumpadAdd" => Key::Plus,
        "Minus" | "NumpadSubtract" => Key::Minus,
        "NumpadMultiply" => Key::Multiply,
        "Slash" | "NumpadDivide" => Key::Slash,
        "Period" | "NumpadDecimal" => Key::Period,
        "Comma" | "NumpadComma" => Key::Comma,
        "Equal" | "NumpadEqual" => Key::Equal,
        "Semicolon" => Key::Semicolon,
        "Quote" => Key::Quote,
        "Backslash" => Key::Backslash,
        "BracketLeft" => Key::BracketLeft,
        "BracketRight" => Key::BracketRight,
        "Backquote" => Key::Backquote,
        "Space" => Key::Space,
        "NumpadEnter" => Key::Enter,
        "ShiftLeft" | "ShiftRight" => Key::Shift,
        "ControlLeft" | "ControlRight" => Key::Control,
        "AltLeft" | "AltRight" => Key::Alt,
        _ => named_key(code).unwrap_or(Key::Other),
    }
}

/// The text a key typed, without control characters; none when it typed
/// nothing, or when Control, Alt or Meta is held (then it is a shortcut).
pub fn typed_text(text: &str, held: Modifiers) -> Option<String> {
    if held.has(Modifiers::CONTROL) || held.has(Modifiers::ALT) || held.has(Modifiers::META) {
        return None;
    }
    let typed: String = text.chars().filter(|c| !c.is_control()).collect();
    (!typed.is_empty()).then_some(typed)
}

/// The CSS `cursor` of a pointer shape.
pub fn cursor_css(cursor: Cursor) -> &'static str {
    match cursor {
        Cursor::Default => "default",
        Cursor::Hand => "pointer",
        Cursor::Text => "text",
        Cursor::ResizeLeftRight => "ew-resize",
        Cursor::ResizeUpDown => "ns-resize",
        Cursor::Move => "move",
        Cursor::Crosshair => "crosshair",
        Cursor::NotAllowed => "not-allowed",
    }
}
