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

/// <summary>The shape of the mouse pointer over a place: each host shows it as its system's.</summary>
public enum Cursor
{
    Default,
    /// <summary>Something that runs when clicked: a link, a canvas command.</summary>
    Hand,
    /// <summary>Text the person types or selects in.</summary>
    Text,
    /// <summary>An edge dragged left or right: a sidebar, a splitter, a column.</summary>
    ResizeLeftRight,
    /// <summary>An edge dragged up or down.</summary>
    ResizeUpDown,
    Move,
    Crosshair,
    NotAllowed,
}

/// <summary>Where the pointer is, on the screen and from the element's top left, and the keys held.</summary>
public readonly record struct PointerInput(double ScreenX, double ScreenY, double X, double Y, Modifiers Modifiers = Modifiers.None, int Clicks = 1);

public readonly record struct KeyInput(Key Key, Modifiers Modifiers)
{
    /// <summary>The character the key typed on the person's keyboard, for a key the screen asks the window's character shortcuts with (Key is then Other); none for a key press.</summary>
    public string? Typed { get; init; }

    /// <summary>The key's own name, from the host, for a key Clarity.UI has no Key for (F13, Pause, PrintScreen); Key is then Other.</summary>
    public string? Name { get; init; }

    public bool Shift => Modifiers.HasFlag(Modifiers.Shift);
    public bool Control => Modifiers.HasFlag(Modifiers.Control);
}

/// <summary>
/// Counts clicks for a host that gives only presses: a press of the same
/// button within the double-click time and a few pixels of the last one is
/// its next click (2 for a double click, 3 for a triple), else a first one.
/// </summary>
public sealed class PressCounter
{
    /// <summary>How long after a press the next one still counts, in milliseconds.</summary>
    public const long Time = 500;

    /// <summary>How far from the last press the next one may be, in pixels.</summary>
    public const double Distance = 4;

    double lastX, lastY;
    long lastAt = long.MinValue;
    PointerButton lastButton;
    int count;

    /// <summary>The click count of a press of <paramref name="button"/> at x, y at <paramref name="milliseconds"/>.</summary>
    public int Press(double x, double y, PointerButton button, long milliseconds)
    {
        var next = count > 0 && button == lastButton && milliseconds - lastAt <= Time
            && Math.Abs(x - lastX) <= Distance && Math.Abs(y - lastY) <= Distance;
        count = next ? count + 1 : 1;
        (lastX, lastY, lastAt, lastButton) = (x, y, milliseconds, button);
        return count;
    }
}
