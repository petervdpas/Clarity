//! A shortcut as Clarity writes it (`Ctrl+S`, `F2`, `Ctrl+Shift+N`) against
//! a key the person pressed, as every stack matches one: the modifiers
//! exactly, letters in either case, Shift for a symbol only when written (it
//! makes + and *), and a typed key without Ctrl, Alt or Meta left to an
//! input the person types in. A symbol is the same key from the keyboard or
//! the number pad.

use crate::input::{Key, KeyInput};

/// The key of each symbol a shortcut writes.
pub const SYMBOLS: [(char, Key); 13] = [
    ('+', Key::Plus),
    ('-', Key::Minus),
    ('*', Key::Multiply),
    ('/', Key::Slash),
    ('.', Key::Period),
    (',', Key::Comma),
    ('=', Key::Equal),
    (';', Key::Semicolon),
    ('\'', Key::Quote),
    ('\\', Key::Backslash),
    ('[', Key::BracketLeft),
    (']', Key::BracketRight),
    ('`', Key::Backquote),
];

/// Names of keys in a shortcut that differ from Clarity.UI's own.
const NAMED: [(&str, Key); 6] = [
    ("Esc", Key::Escape),
    ("Return", Key::Enter),
    ("Del", Key::Delete),
    ("Ins", Key::Insert),
    ("PgUp", Key::PageUp),
    ("PgDn", Key::PageDown),
];

/// The keys a shortcut's last word may name by Clarity.UI's own name.
const WORDS: [(&str, Key); 16] = [
    ("Tab", Key::Tab),
    ("Enter", Key::Enter),
    ("Escape", Key::Escape),
    ("Space", Key::Space),
    ("Backspace", Key::Backspace),
    ("Delete", Key::Delete),
    ("Insert", Key::Insert),
    ("Left", Key::Left),
    ("Right", Key::Right),
    ("Up", Key::Up),
    ("Down", Key::Down),
    ("Home", Key::Home),
    ("End", Key::End),
    ("PageUp", Key::PageUp),
    ("PageDown", Key::PageDown),
    ("Shift", Key::Shift),
];

/// The key of a symbol; none for a character that is no symbol.
pub fn symbol(c: char) -> Option<Key> {
    SYMBOLS.iter().find(|(s, _)| *s == c).map(|(_, k)| *k)
}

/// The modifiers of a shortcut and its key: `Ctrl++` is Ctrl and `+`.
pub fn parts(shortcut: &str) -> (Vec<&str>, &str) {
    if shortcut == "+" {
        return (Vec::new(), "+");
    }
    if shortcut.len() > 2 && shortcut.ends_with("++") {
        return (shortcut[..shortcut.len() - 2].split('+').collect(), "+");
    }
    let mut keys: Vec<&str> = shortcut.split('+').collect();
    let last = keys.pop().unwrap_or("");
    (keys, last)
}

/// The key of a shortcut's last word; `Other` for one Clarity.UI has no key for.
pub fn key_of(word: &str) -> Key {
    let mut chars = word.chars();
    if let (Some(c), None) = (chars.next(), chars.next()) {
        if c.is_ascii_alphabetic() {
            return Key::LETTERS[(c.to_ascii_uppercase() as u8 - b'A') as usize];
        }
        if c.is_ascii_digit() {
            return Key::DIGITS[(c as u8 - b'0') as usize];
        }
        if let Some(key) = symbol(c) {
            return key;
        }
    }
    let same = |name: &&str| name.eq_ignore_ascii_case(word);
    if let Some((_, key)) = NAMED.iter().find(|(n, _)| same(n)) {
        return *key;
    }
    if let Some((_, key)) = WORDS.iter().find(|(n, _)| same(n)) {
        return *key;
    }
    if word.len() > 1 && (word.starts_with('F') || word.starts_with('f')) {
        if let Ok(n) = word[1..].parse::<usize>() {
            if (1..=12).contains(&n) {
                return Key::FUNCTIONS[n - 1];
            }
        }
    }
    if word.eq_ignore_ascii_case("Control") {
        return Key::Control;
    }
    if word.eq_ignore_ascii_case("Alt") {
        return Key::Alt;
    }
    Key::Other
}

/// Whether `shortcut` is the key pressed; `typing` says the focus is in an input the person types in.
pub fn matches(shortcut: &str, key: &KeyInput, typing: bool) -> bool {
    let (modifiers, word) = parts(shortcut.trim());
    // A character typed: a symbol shortcut without Ctrl, Alt or Meta answers what the keyboard typed, wherever its key is.
    if let Some(character) = key.typed {
        let mut chars = word.chars();
        let one = match (chars.next(), chars.next()) {
            (Some(c), None) => Some(c),
            _ => None,
        };
        return !typing
            && one.is_some_and(|c| !c.is_ascii_alphanumeric() && c == character)
            && modifiers.iter().all(|m| m.eq_ignore_ascii_case("Shift"));
    }
    let wanted = key_of(word);
    // A key Clarity.UI has no Key for answers by the name its host gives it (F13, Pause).
    if wanted == Key::Other && !(key.key == Key::Other && key.name.is_some_and(|n| n.eq_ignore_ascii_case(word))) {
        return false;
    }
    if wanted != key.key {
        return false;
    }
    let held = |name: &str| modifiers.iter().any(|m| m.eq_ignore_ascii_case(name));
    let control = held("Ctrl") || held("Control");
    if key.control() != control || key.alt() != held("Alt") || key.meta() != (held("Meta") || held("Cmd")) {
        return false;
    }
    if !(wanted.is_symbol() && !held("Shift")) && key.shift() != held("Shift") {
        return false;
    }
    // A letter, digit or symbol without Ctrl, Alt or Meta is what the person types in an input.
    let typed = wanted.is_typed();
    !(typed && typing && !control && !held("Alt") && !held("Meta"))
}
