//! What the system's window gives, as Clarity.UI takes it: winit's keys as
//! Clarity.UI's keys, the keys held, the text typed and the wheel in the
//! screen's units. Plain functions, so they are tested without a window.

use winit::event::MouseScrollDelta;
use winit::keyboard::{Key as Logical, KeyCode, ModifiersState, NamedKey, PhysicalKey};

use crate::input::{Key, Modifiers};

/// How far one notch of the wheel scrolls, in the screen's units.
pub const LINE: f64 = 40.0;

/// Clarity.UI's key of a key of the keyboard. What the key writes comes
/// first, so a shortcut is the letter or symbol the person sees on it (the
/// pad's + is `Plus`, as Shift with = is); a key that writes nothing Clarity.UI
/// has a key for is the key where it lies.
pub fn key_of(logical: &Logical, physical: PhysicalKey) -> Key {
    let written = match logical {
        Logical::Named(name) => named_key(*name),
        Logical::Character(text) => written_key(text),
        _ => Key::Other,
    };
    if written != Key::Other {
        return written;
    }
    match physical {
        PhysicalKey::Code(code) => code_key(code),
        PhysicalKey::Unidentified(_) => Key::Other,
    }
}

/// The key a named key is.
fn named_key(name: NamedKey) -> Key {
    match name {
        NamedKey::Tab => Key::Tab,
        NamedKey::Enter => Key::Enter,
        NamedKey::Escape => Key::Escape,
        NamedKey::Space => Key::Space,
        NamedKey::Backspace => Key::Backspace,
        NamedKey::Delete => Key::Delete,
        NamedKey::Insert => Key::Insert,
        NamedKey::ArrowLeft => Key::Left,
        NamedKey::ArrowRight => Key::Right,
        NamedKey::ArrowUp => Key::Up,
        NamedKey::ArrowDown => Key::Down,
        NamedKey::Home => Key::Home,
        NamedKey::End => Key::End,
        NamedKey::PageUp => Key::PageUp,
        NamedKey::PageDown => Key::PageDown,
        NamedKey::F1 => Key::F1,
        NamedKey::F2 => Key::F2,
        NamedKey::F3 => Key::F3,
        NamedKey::F4 => Key::F4,
        NamedKey::F5 => Key::F5,
        NamedKey::F6 => Key::F6,
        NamedKey::F7 => Key::F7,
        NamedKey::F8 => Key::F8,
        NamedKey::F9 => Key::F9,
        NamedKey::F10 => Key::F10,
        NamedKey::F11 => Key::F11,
        NamedKey::F12 => Key::F12,
        NamedKey::Shift => Key::Shift,
        NamedKey::Control => Key::Control,
        NamedKey::Alt => Key::Alt,
        _ => Key::Other,
    }
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

/// The key where a key lies on a US keyboard, the number pad's as the main keyboard's.
fn code_key(code: KeyCode) -> Key {
    use KeyCode as C;
    match code {
        C::KeyA => Key::A,
        C::KeyB => Key::B,
        C::KeyC => Key::C,
        C::KeyD => Key::D,
        C::KeyE => Key::E,
        C::KeyF => Key::F,
        C::KeyG => Key::G,
        C::KeyH => Key::H,
        C::KeyI => Key::I,
        C::KeyJ => Key::J,
        C::KeyK => Key::K,
        C::KeyL => Key::L,
        C::KeyM => Key::M,
        C::KeyN => Key::N,
        C::KeyO => Key::O,
        C::KeyP => Key::P,
        C::KeyQ => Key::Q,
        C::KeyR => Key::R,
        C::KeyS => Key::S,
        C::KeyT => Key::T,
        C::KeyU => Key::U,
        C::KeyV => Key::V,
        C::KeyW => Key::W,
        C::KeyX => Key::X,
        C::KeyY => Key::Y,
        C::KeyZ => Key::Z,
        C::Digit0 | C::Numpad0 => Key::D0,
        C::Digit1 | C::Numpad1 => Key::D1,
        C::Digit2 | C::Numpad2 => Key::D2,
        C::Digit3 | C::Numpad3 => Key::D3,
        C::Digit4 | C::Numpad4 => Key::D4,
        C::Digit5 | C::Numpad5 => Key::D5,
        C::Digit6 | C::Numpad6 => Key::D6,
        C::Digit7 | C::Numpad7 => Key::D7,
        C::Digit8 | C::Numpad8 => Key::D8,
        C::Digit9 | C::Numpad9 => Key::D9,
        C::NumpadAdd => Key::Plus,
        C::Minus | C::NumpadSubtract => Key::Minus,
        C::NumpadMultiply => Key::Multiply,
        C::Slash | C::NumpadDivide => Key::Slash,
        C::Period | C::NumpadDecimal => Key::Period,
        C::Comma | C::NumpadComma => Key::Comma,
        C::Equal | C::NumpadEqual => Key::Equal,
        C::Semicolon => Key::Semicolon,
        C::Quote => Key::Quote,
        C::Backslash => Key::Backslash,
        C::BracketLeft => Key::BracketLeft,
        C::BracketRight => Key::BracketRight,
        C::Backquote => Key::Backquote,
        C::Tab => Key::Tab,
        C::Enter | C::NumpadEnter => Key::Enter,
        C::Escape => Key::Escape,
        C::Space => Key::Space,
        C::Backspace | C::NumpadBackspace => Key::Backspace,
        C::Delete => Key::Delete,
        C::Insert => Key::Insert,
        C::ArrowLeft => Key::Left,
        C::ArrowRight => Key::Right,
        C::ArrowUp => Key::Up,
        C::ArrowDown => Key::Down,
        C::Home => Key::Home,
        C::End => Key::End,
        C::PageUp => Key::PageUp,
        C::PageDown => Key::PageDown,
        C::F1 => Key::F1,
        C::F2 => Key::F2,
        C::F3 => Key::F3,
        C::F4 => Key::F4,
        C::F5 => Key::F5,
        C::F6 => Key::F6,
        C::F7 => Key::F7,
        C::F8 => Key::F8,
        C::F9 => Key::F9,
        C::F10 => Key::F10,
        C::F11 => Key::F11,
        C::F12 => Key::F12,
        C::ShiftLeft | C::ShiftRight => Key::Shift,
        C::ControlLeft | C::ControlRight => Key::Control,
        C::AltLeft | C::AltRight => Key::Alt,
        _ => Key::Other,
    }
}

/// The keys held, as Clarity.UI has them; the Super key (Command, Windows) is Meta.
pub fn modifiers_of(state: ModifiersState) -> Modifiers {
    let mut held = Modifiers::NONE;
    if state.shift_key() {
        held |= Modifiers::SHIFT;
    }
    if state.control_key() {
        held |= Modifiers::CONTROL;
    }
    if state.alt_key() {
        held |= Modifiers::ALT;
    }
    if state.super_key() {
        held |= Modifiers::META;
    }
    held
}

/// The text a key typed, without control characters; none when it typed
/// nothing, or when Ctrl, Alt or Meta is held (then it is a shortcut).
pub fn typed_text(text: Option<&str>, held: Modifiers) -> Option<String> {
    if held.has(Modifiers::CONTROL) || held.has(Modifiers::ALT) || held.has(Modifiers::META) {
        return None;
    }
    let typed: String = text?.chars().filter(|c| !c.is_control()).collect();
    (!typed.is_empty()).then_some(typed)
}

/// A turn of the wheel in the screen's units, as `Screen::wheel` takes it:
/// a positive `dy` shows what is further down, a positive `dx` what is
/// further right. A notch is `LINE`; a touchpad's pixels are the system's,
/// divided by the window's `scale`.
pub fn wheel_of(delta: MouseScrollDelta, scale: f64) -> (f64, f64) {
    match delta {
        MouseScrollDelta::LineDelta(x, y) => (-(x as f64) * LINE, -(y as f64) * LINE),
        MouseScrollDelta::PixelDelta(at) => {
            let scale = if scale > 0.0 { scale } else { 1.0 };
            (-at.x / scale, -at.y / scale)
        }
    }
}

/// Where the pointer is over a desktop window. winit on X11 tells a move
/// only when it differs from the last move it saw, and the place it gives on
/// entering the window or on its focus coming back does not count as one.
/// So after a dialog of its own closed over the main window, a move straight
/// back to where the pointer last moved there (a touch, a tablet, a warp) is
/// not told, and a press would go to where the dialog's button was. Kept
/// here: the place last told, the last move, and whether the pointer moved
/// (the system's raw motion) since a place was only given; then the pointer
/// is where it last moved. Where the window system tells every move, the
/// place told is where it is.
#[derive(Debug, Clone, Default)]
pub struct PointerPlace {
    /// Whether the window system leaves such a move untold (X11).
    untold: bool,
    at: (f64, f64),
    /// Where the window system last saw it move to.
    moved: Option<(f64, f64)>,
    /// The next place told is given for entering or focus, not a move.
    giving: bool,
    /// The place it is at was given for entering or focus.
    given: bool,
    /// The pointer moved since the place was given, and no move was told.
    moved_untold: bool,
}

impl PointerPlace {
    /// A place kept for a window system that leaves a move back untold (`untold`, X11) or tells every move.
    pub fn new(untold: bool) -> PointerPlace {
        PointerPlace { untold, ..PointerPlace::default() }
    }

    /// Where the pointer is, in the screen's units.
    pub fn at(&self) -> (f64, f64) {
        self.at
    }

    /// The pointer entered the window: the next place told is given, not a move.
    pub fn entered(&mut self) {
        self.giving = self.untold;
    }

    /// The window got the focus back: the next place told is given, as on entering.
    pub fn focused(&mut self) {
        self.entered();
    }

    /// The window system told a place.
    pub fn told(&mut self, x: f64, y: f64) {
        self.at = (x, y);
        if self.giving {
            self.giving = false;
            self.given = true;
        } else {
            self.moved = Some((x, y));
            self.given = false;
        }
        self.moved_untold = false;
    }

    /// The pointer moved (the system's raw motion), told or not.
    pub fn raw_motion(&mut self) {
        if self.given {
            self.moved_untold = true;
        }
    }

    /// Where the pointer is now, when a move was left untold: where it last
    /// moved to. None when the place told stands.
    pub fn settle(&mut self) -> Option<(f64, f64)> {
        if !(self.given && self.moved_untold) {
            return None;
        }
        let moved = self.moved?;
        self.at = moved;
        self.given = false;
        self.moved_untold = false;
        Some(moved)
    }
}
