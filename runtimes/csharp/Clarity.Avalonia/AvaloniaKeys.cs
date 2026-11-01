// Avalonia's keys as Clarity writes keys (`Ctrl+Shift+N`), both ways: what
// the person pressed, and a command's key as Avalonia's gesture. The rules
// for setting keys are Clarity.Ui's KeyRules.

using Avalonia.Input;

namespace Clarity.Avalonia;

public static class AvaloniaKeys
{
    /// <summary>Avalonia's names of the keys Clarity names otherwise, both ways.</summary>
    static readonly (Key Key, string Written)[] Named =
    [
        (Key.Back, "Backspace"), (Key.OemPlus, "+"), (Key.Add, "+"), (Key.OemMinus, "-"), (Key.Subtract, "-"),
        (Key.OemPeriod, "."), (Key.Decimal, "."), (Key.OemComma, ","), (Key.Multiply, "*"), (Key.Divide, "/"),
        (Key.Return, "Enter"), (Key.Escape, "Escape"), (Key.Space, "Space"), (Key.Tab, "Tab"),
        (Key.Left, "Left"), (Key.Right, "Right"), (Key.Up, "Up"), (Key.Down, "Down"),
        (Key.Delete, "Delete"), (Key.Insert, "Insert"), (Key.Home, "Home"), (Key.End, "End"),
        (Key.PageUp, "PageUp"), (Key.PageDown, "PageDown"),
    ];

    static readonly Key[] ModifierKeys = [Key.LeftCtrl, Key.RightCtrl, Key.LeftShift, Key.RightShift, Key.LeftAlt, Key.RightAlt, Key.LWin, Key.RWin];

    /// <summary>A key pressed as Clarity writes keys (<c>Ctrl+Shift+N</c>); null for a modifier on its own.</summary>
    public static string? Written(Key key, KeyModifiers modifiers)
    {
        if (ModifierKeys.Contains(key) || key == Key.None)
            return null;
        var name = key switch
        {
            >= Key.D0 and <= Key.D9 => ((char)('0' + (key - Key.D0))).ToString(),
            >= Key.NumPad0 and <= Key.NumPad9 => ((char)('0' + (key - Key.NumPad0))).ToString(),
            >= Key.A and <= Key.Z => ((char)('A' + (key - Key.A))).ToString(),
            _ => Named.FirstOrDefault(n => n.Key == key).Written ?? key.ToString(),
        };
        var held = new List<string>();
        if (modifiers.HasFlag(KeyModifiers.Control)) held.Add("Ctrl");
        if (modifiers.HasFlag(KeyModifiers.Alt)) held.Add("Alt");
        if (modifiers.HasFlag(KeyModifiers.Shift)) held.Add("Shift");
        if (modifiers.HasFlag(KeyModifiers.Meta)) held.Add("Meta");
        held.Add(name);
        return string.Join("+", held);
    }

    /// <summary>A key as Clarity writes it, as Avalonia's gesture: <c>Ctrl+1</c> is <c>Ctrl+D1</c>.</summary>
    public static KeyGesture? Gesture(string written)
    {
        var (held, last) = KeyRules.Parts(written);
        var key = last switch
        {
            [var c] when char.IsAsciiDigit(c) => "D" + c,
            "+" or "=" => "OemPlus",
            _ => Named.FirstOrDefault(n => n.Written == last && n.Key is not Key.Add and not Key.Subtract and not Key.Decimal).Key is var k && k != Key.None ? k.ToString() : last,
        };
        try
        {
            return KeyGesture.Parse(string.Join("+", [.. held, key]));
        }
        catch (ArgumentException)
        {
            return null;
        }
    }
}
