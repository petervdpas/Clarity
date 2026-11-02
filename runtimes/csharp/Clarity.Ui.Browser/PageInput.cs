// What the person does on a browser page, as the screen takes it (DESIGN 7.9).
// The page gives presses only, so a double or triple click is counted here,
// as the desktop counts it. Text comes from the page's hidden text field, not
// from the keys: what AltGr, Option, a dead key, an input method or a phone's
// keyboard makes arrives as the text it is. Copying and pasting go through
// the browser's clipboard: a copy says its text to the page, and a paste
// brings the page's text in before the screen hears Ctrl+V. A key the
// browser has a use for (reloading, its tools) stays the browser's.

using Clarity.Ui;

namespace Clarity.Ui.Browser;

/// <summary>What the page does with a key after the screen heard it.</summary>
public enum PageKeyWay
{
    /// <summary>The page keeps the browser from doing anything with it (Tab, arrows, a shortcut the screen takes).</summary>
    Prevent,

    /// <summary>The browser does what it does: types the text into the hidden field, copies, pastes, reloads.</summary>
    Browser,
}

/// <summary>The browser's clipboard as the screen sees it: what was last copied or pasted on the page, and each copy said to the page.</summary>
public sealed class PageClipboard(Action<string> copy) : IClipboard
{
    string? text;

    public string? GetText() => text;

    public void SetText(string value)
    {
        text = value;
        copy(value);
    }

    /// <summary>The browser pasted <paramref name="value"/>: it is what the screen pastes next.</summary>
    public void Pasted(string value) => text = value;
}

/// <summary>The page's pointer shapes: CSS's cursor of each.</summary>
public static class PageCursor
{
    public static string Css(Cursor cursor) => cursor switch
    {
        Cursor.Hand => "pointer",
        Cursor.Text => "text",
        Cursor.ResizeLeftRight => "ew-resize",
        Cursor.ResizeUpDown => "ns-resize",
        Cursor.Move => "move",
        Cursor.Crosshair => "crosshair",
        Cursor.NotAllowed => "not-allowed",
        _ => "default",
    };
}

/// <summary>The person's pointer, keys and text on a page, given to its screen.</summary>
public sealed class PageInput
{
    readonly Screen screen;
    readonly PressCounter presses = new();
    Modifiers pasteHeld = Modifiers.Control;

    /// <summary>Takes what the person does on the page to <paramref name="screen"/>; what it copies goes to <paramref name="copy"/> (the browser's clipboard).</summary>
    public PageInput(Screen screen, Action<string> copy)
    {
        this.screen = screen;
        Clipboard = new PageClipboard(copy);
        screen.Clipboard = Clipboard;
    }

    public PageClipboard Clipboard { get; }

    /// <summary>A press at x, y at <paramref name="milliseconds"/>: its click count is counted as the desktop counts it.</summary>
    public void Pressed(double x, double y, PointerButton button, long milliseconds) =>
        screen.PointerPressed(x, y, button, presses.Press(x, y, button, milliseconds));

    /// <summary>
    /// A key went down (a browser's KeyboardEvent): the screen hears it,
    /// unless the browser's text field makes it into text first (a dead key,
    /// an input method) or it is a paste, which the screen hears once the
    /// pasted text is in. <paramref name="altGraph"/> is AltGr held (which
    /// browsers on Windows also say as Ctrl and Alt); <paramref name="mac"/>
    /// says Option makes characters, as it does on a Mac.
    /// </summary>
    public PageKeyWay KeyDown(string key, Modifiers held, bool altGraph, bool mac)
    {
        // A key an input method or a dead key works on: its text comes as text.
        if (key is "Dead" or "Process" or "Unidentified" or "Compose")
            return PageKeyWay.Browser;
        if (altGraph)
            held &= ~(Modifiers.Control | Modifiers.Alt);
        var command = held.HasFlag(Modifiers.Control) || held.HasFlag(Modifiers.Meta);
        var pressed = new KeyInput(WebKeys.Of(key), held) { Name = WebKeys.NameOf(key) };
        if (command && key is "v" or "V")
        {
            // The browser's paste event brings the text; the screen hears the keys then.
            pasteHeld = held;
            return PageKeyWay.Browser;
        }
        screen.KeyPressed(pressed);
        // Copying and cutting: the browser's copy event takes what the screen just copied.
        if (command && key is "c" or "C" or "x" or "X")
            return PageKeyWay.Browser;
        if (BrowserKey(key, held))
            return PageKeyWay.Browser;
        // A character typed goes into the page's text field, which says it as text.
        return Types(key, held, mac) ? PageKeyWay.Browser : PageKeyWay.Prevent;
    }

    // -- Touch ---------------------------------------------------------------------

    /// <summary>How far a finger moves before it pans, in the page's pixels.</summary>
    public const double Slop = 8;

    /// <summary>How long a finger rests before a move drags what it is on (a slider, a selection) instead of panning, in milliseconds.</summary>
    public const long Hold = 400;

    (double X, double Y, long At, double LastX, double LastY, bool Panning, bool Pressed)? touch;

    /// <summary>A finger went down: what it does is known once it moves or lifts.</summary>
    public void TouchDown(double x, double y, long milliseconds) => touch = (x, y, milliseconds, x, y, false, false);

    /// <summary>
    /// A finger moved: past a few pixels at once it pans (the wheel the
    /// screen knows, as a scroll bar or a touchpad turns it); after resting
    /// a moment it drags what it is on, as a held mouse button does.
    /// </summary>
    public void TouchMoved(double x, double y, long milliseconds)
    {
        if (touch is not { } t)
            return;
        if (t.Pressed)
        {
            screen.PointerMoved(x, y);
            touch = t with { LastX = x, LastY = y };
            return;
        }
        if (!t.Panning && Math.Abs(x - t.X) <= Slop && Math.Abs(y - t.Y) <= Slop)
            return;
        if (!t.Panning && milliseconds - t.At >= Hold)
        {
            Pressed(t.X, t.Y, PointerButton.Left, t.At);
            screen.PointerMoved(x, y);
            touch = t with { LastX = x, LastY = y, Pressed = true };
            return;
        }
        // The content follows the finger: a finger going up shows what is further down.
        screen.Wheel(t.X, t.Y, t.LastX - x, t.LastY - y);
        touch = t with { LastX = x, LastY = y, Panning = true };
    }

    /// <summary>The browser took the finger over (a gesture of its own): it does nothing more; a drag it was making is dropped, not committed.</summary>
    public void TouchCancelled()
    {
        if (touch is { Pressed: true })
            screen.PointerCancelled();
        touch = null;
    }

    /// <summary>The browser took the pointer away mid-press (a pen or mouse's pointercancel, the page lost the focus): a drag is dropped, as on the desktop.</summary>
    public void PointerCancelled()
    {
        touch = null;
        screen.PointerCancelled();
    }

    // -- The wheel -------------------------------------------------------------------

    /// <summary>A browser's notch of a mouse wheel in the page's pixels (Chrome, Edge and Safari give 100 at deltaMode 0).</summary>
    public const double PageNotch = 100;

    /// <summary>A browser's notch in lines (Firefox gives 3 at deltaMode 1).</summary>
    public const double LinesNotch = 3;

    /// <summary>
    /// A wheel event's deltas in the screen's units: one notch of a mouse
    /// wheel is 50, as on the desktop (DesktopWindow.WheelNotch), whether the
    /// browser said it in pixels (<paramref name="mode"/> 0, 100 a notch) or
    /// lines (1, 3 a notch); a page (2) is <paramref name="pageHeight"/>. A
    /// touchpad's small pixel turns scale the same, so Ctrl+wheel zooms 1.15
    /// a notch in every browser, not 1.15 squared.
    /// </summary>
    public static (double Dx, double Dy) WheelUnits(double dx, double dy, int mode, double pageHeight)
    {
        var unit = mode switch
        {
            1 => CanvasPart.Notch / LinesNotch,
            2 => pageHeight,
            _ => CanvasPart.Notch / PageNotch,
        };
        return (dx * unit, dy * unit);
    }

    /// <summary>The wheel turned (a browser's wheel event, its deltas in <paramref name="mode"/>), with the keys held.</summary>
    public void Wheel(double x, double y, double dx, double dy, int mode, double pageHeight, Modifiers held)
    {
        var (ux, uy) = WheelUnits(dx, dy, mode, pageHeight);
        screen.Wheel(x, y, ux, uy, held);
    }

    /// <summary>A finger lifted: a tap is a click where it went down; a pan or a drag ends.</summary>
    public void TouchUp(double x, double y, long milliseconds)
    {
        if (touch is not { } t)
            return;
        touch = null;
        if (t.Pressed)
            screen.PointerReleased(x, y, PointerButton.Left);
        else if (!t.Panning)
        {
            Pressed(t.X, t.Y, PointerButton.Left, milliseconds);
            screen.PointerReleased(t.X, t.Y, PointerButton.Left);
        }
    }

    /// <summary>A key let go.</summary>
    public void KeyUp(string key, Modifiers held, bool altGraph)
    {
        if (altGraph)
            held &= ~(Modifiers.Control | Modifiers.Alt);
        screen.KeyReleased(new KeyInput(WebKeys.Of(key), held) { Name = WebKeys.NameOf(key) });
    }

    /// <summary>The browser pasted <paramref name="text"/> (Ctrl+V or Cmd+V): the screen pastes it.</summary>
    public void Pasted(string text)
    {
        Clipboard.Pasted(text);
        screen.KeyPressed(new KeyInput(Key.V, pasteHeld));
    }

    /// <summary>Text the page's field took (a key, AltGr, a dead key with its letter, an input method's words): the screen takes it, without control characters.</summary>
    public void Typed(string text)
    {
        var clean = new string([.. text.Where(c => !char.IsControl(c))]);
        if (clean.Length > 0)
            screen.TextTyped(clean);
    }

    /// <summary>Whether the part with the focus takes typed text: a phone then shows its keyboard.</summary>
    public bool TakesText => screen.Focused is TextEditor { Enabled: var on, ReadOnly: var read } && on() && !read();

    /// <summary>Where the part with the focus is, for an input method's window to show beside it.</summary>
    public Box? FocusedPlace => screen.Focused?.Bounds;

    /// <summary>Whether <paramref name="key"/> with <paramref name="held"/> makes a character in the page's text field.</summary>
    public static bool Types(string key, Modifiers held, bool mac)
    {
        if (held.HasFlag(Modifiers.Control) || held.HasFlag(Modifiers.Meta))
            return false;
        // Alt with a letter is a shortcut on Windows and Linux; Option makes a character on a Mac.
        if (held.HasFlag(Modifiers.Alt) && !mac)
            return false;
        return key.EnumerateRunes().Count() == 1 && !char.IsControl(key, 0);
    }

    /// <summary>The browser's own keys, which reach it: reloading, full screen and its tools.</summary>
    public static bool BrowserKey(string key, Modifiers held)
    {
        var command = held.HasFlag(Modifiers.Control) || held.HasFlag(Modifiers.Meta);
        return key switch
        {
            "F5" or "F11" or "F12" => true,
            "r" or "R" => command,
            "i" or "I" or "j" or "J" => command && held.HasFlag(Modifiers.Shift),
            _ => false,
        };
    }
}
