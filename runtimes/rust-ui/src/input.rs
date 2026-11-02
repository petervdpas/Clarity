//! What the person does, as each host gives it: the desktop's window, a
//! picture in a test. The same keys and buttons on every system.

use std::ops::{BitOr, BitOrAssign};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PointerButton {
    Left,
    Middle,
    Right,
}

/// The keys held with a key or a press: Shift, Control, Alt and Meta.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Modifiers(u8);

impl Modifiers {
    pub const NONE: Modifiers = Modifiers(0);
    pub const SHIFT: Modifiers = Modifiers(1);
    pub const CONTROL: Modifiers = Modifiers(2);
    pub const ALT: Modifiers = Modifiers(4);
    /// The Command key on macOS, the Windows key elsewhere.
    pub const META: Modifiers = Modifiers(8);

    /// Whether every key of `other` is held.
    pub fn has(self, other: Modifiers) -> bool {
        self.0 & other.0 == other.0
    }

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }
}

impl BitOr for Modifiers {
    type Output = Modifiers;

    fn bitor(self, other: Modifiers) -> Modifiers {
        Modifiers(self.0 | other.0)
    }
}

impl BitOrAssign for Modifiers {
    fn bitor_assign(&mut self, other: Modifiers) {
        self.0 |= other.0;
    }
}

/// A key of the keyboard, the same on every system. The symbols come from
/// the keyboard or the number pad alike: the pad's + is `Plus` too.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Key {
    Other,
    Tab, Enter, Escape, Space, Backspace, Delete, Insert,
    Left, Right, Up, Down, Home, End, PageUp, PageDown,
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
    // The modifier keys themselves, going down and up; Shift is either Shift key.
    Shift, Control, Alt,
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
    D0, D1, D2, D3, D4, D5, D6, D7, D8, D9,
    Plus, Minus, Multiply, Slash, Period, Comma, Equal, Semicolon, Quote, Backslash, BracketLeft, BracketRight, Backquote,
}

impl Key {
    /// The letters in order, A first.
    pub const LETTERS: [Key; 26] = [
        Key::A, Key::B, Key::C, Key::D, Key::E, Key::F, Key::G, Key::H, Key::I, Key::J, Key::K, Key::L, Key::M,
        Key::N, Key::O, Key::P, Key::Q, Key::R, Key::S, Key::T, Key::U, Key::V, Key::W, Key::X, Key::Y, Key::Z,
    ];

    /// The digits of the main keyboard in order, 0 first.
    pub const DIGITS: [Key; 10] = [Key::D0, Key::D1, Key::D2, Key::D3, Key::D4, Key::D5, Key::D6, Key::D7, Key::D8, Key::D9];

    /// The function keys in order, F1 first.
    pub const FUNCTIONS: [Key; 12] = [Key::F1, Key::F2, Key::F3, Key::F4, Key::F5, Key::F6, Key::F7, Key::F8, Key::F9, Key::F10, Key::F11, Key::F12];

    /// Whether it is a key the person types with: a letter, a digit or a symbol.
    pub fn is_typed(self) -> bool {
        self >= Key::A
    }

    /// Whether it is a symbol: Shift makes some (+, *), so it only counts in a shortcut when written.
    pub fn is_symbol(self) -> bool {
        self >= Key::Plus
    }
}

/// Where the pointer is, on the screen and from the element's top left, the keys held and how many clicks it is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointerInput {
    pub screen_x: f64,
    pub screen_y: f64,
    pub x: f64,
    pub y: f64,
    pub modifiers: Modifiers,
    pub clicks: u32,
}

/// The shape of the mouse pointer over a place: each host shows it as its system's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Cursor {
    #[default]
    Default,
    /// Something that runs when clicked: a link, a canvas command.
    Hand,
    /// Text the person types or selects in.
    Text,
    /// An edge dragged left or right: a sidebar, a splitter, a column.
    ResizeLeftRight,
    /// An edge dragged up or down.
    ResizeUpDown,
    Move,
    Crosshair,
    NotAllowed,
}

/// A key going down or up, with the keys held.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyInput {
    pub key: Key,
    pub modifiers: Modifiers,
    /// The character the key typed on the person's keyboard, for a key the
    /// screen asks the window's character shortcuts with (`key` is then
    /// `Other`); none for a key press.
    pub typed: Option<char>,
    /// The key's own name, from the host, for a key Clarity.UI has no `Key`
    /// for (F13, Pause, PrintScreen); `key` is then `Other`.
    pub name: Option<&'static str>,
}

impl KeyInput {
    pub fn new(key: Key, modifiers: Modifiers) -> KeyInput {
        KeyInput { key, modifiers, typed: None, name: None }
    }

    /// The key alone, nothing held.
    pub fn plain(key: Key) -> KeyInput {
        KeyInput { key, modifiers: Modifiers::NONE, typed: None, name: None }
    }

    /// A character typed after a key nothing took, as the window's character shortcuts hear it (`*` from Shift+8).
    pub fn typed(character: char) -> KeyInput {
        KeyInput { key: Key::Other, modifiers: Modifiers::NONE, typed: Some(character), name: None }
    }

    /// A key Clarity.UI has no `Key` for, by the name its host gives it (`F13`), with the keys held.
    pub fn named(name: &'static str, modifiers: Modifiers) -> KeyInput {
        KeyInput { key: Key::Other, modifiers, typed: None, name: Some(name) }
    }

    pub fn shift(&self) -> bool {
        self.modifiers.has(Modifiers::SHIFT)
    }

    pub fn control(&self) -> bool {
        self.modifiers.has(Modifiers::CONTROL)
    }

    pub fn alt(&self) -> bool {
        self.modifiers.has(Modifiers::ALT)
    }

    pub fn meta(&self) -> bool {
        self.modifiers.has(Modifiers::META)
    }
}

/// Counts clicks for a host that gives only presses: a press of the same
/// button within the double-click time and a few pixels of the last one is
/// its next click (2 for a double click, 3 for a triple), else a first one.
#[derive(Debug, Default)]
pub struct PressCounter {
    last: Option<(f64, f64, i64, PointerButton)>,
    count: u32,
}

impl PressCounter {
    /// How long after a press the next one still counts, in milliseconds.
    pub const TIME: i64 = 500;

    /// How far from the last press the next one may be, in pixels.
    pub const DISTANCE: f64 = 4.0;

    pub fn new() -> PressCounter {
        PressCounter::default()
    }

    /// The click count of a press of `button` at `x`, `y` at `milliseconds`.
    pub fn press(&mut self, x: f64, y: f64, button: PointerButton, milliseconds: i64) -> u32 {
        let next = match self.last {
            Some((lx, ly, at, b)) => {
                self.count > 0 && b == button && milliseconds - at <= Self::TIME && (x - lx).abs() <= Self::DISTANCE && (y - ly).abs() <= Self::DISTANCE
            }
            None => false,
        };
        self.count = if next { self.count + 1 } else { 1 };
        self.last = Some((x, y, milliseconds, button));
        self.count
    }
}
