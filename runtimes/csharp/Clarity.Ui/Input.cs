// What the person does, as each host gives it: the desktop's window, a
// browser page's canvas. The same keys and buttons on every system.

namespace Clarity.Ui;

public enum PointerButton
{
    Left,
    Middle,
    Right,
}

[Flags]
public enum Modifiers
{
    None = 0,
    Shift = 1,
    Control = 2,
    Alt = 4,
    /// <summary>The Command key on macOS, the Windows key elsewhere.</summary>
    Meta = 8,
}

public enum Key
{
    Other,
    Tab, Enter, Escape, Space, Backspace, Delete, Insert,
    Left, Right, Up, Down, Home, End, PageUp, PageDown,
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
    // The modifier keys themselves, going down and up (an engine's held keys); Shift is either Shift key.
    Shift, Control, Alt,
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
    D0, D1, D2, D3, D4, D5, D6, D7, D8, D9,
    // The symbols, from the keyboard or the number pad alike: the pad's + is Plus too.
    Plus, Minus, Multiply, Slash, Period, Comma, Equal, Semicolon, Quote, Backslash, BracketLeft, BracketRight, Backquote,
}

/// <summary>Where the pointer is, on the screen and from the element's top left, and the keys held.</summary>
public readonly record struct PointerInput(double ScreenX, double ScreenY, double X, double Y, Modifiers Modifiers = Modifiers.None, int Clicks = 1);

public readonly record struct KeyInput(Key Key, Modifiers Modifiers)
{
    public bool Shift => Modifiers.HasFlag(Modifiers.Shift);
    public bool Control => Modifiers.HasFlag(Modifiers.Control);
}
