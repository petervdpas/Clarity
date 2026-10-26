// The keys an engine beside the screen gets, by Clarity's names, as the web
// renderer names them too (conformance/screen-engines.json, keys): a letter
// as itself in lower case, a digit, and the named keys. A key goes down
// once however long it repeats.

using Avalonia.Input;

namespace Clarity.Avalonia;

public static class ScreenKeys
{
    /// <summary>Clarity's name of <paramref name="key"/>; null for one an engine does not get.</summary>
    public static string? NameOf(Key key) => key switch
    {
        >= Key.A and <= Key.Z => ((char)('a' + (key - Key.A))).ToString(),
        >= Key.D0 and <= Key.D9 => ((char)('0' + (key - Key.D0))).ToString(),
        >= Key.NumPad0 and <= Key.NumPad9 => ((char)('0' + (key - Key.NumPad0))).ToString(),
        >= Key.F1 and <= Key.F12 => "F" + (key - Key.F1 + 1),
        Key.Space => "Space",
        Key.Enter => "Enter",
        Key.Escape => "Escape",
        Key.Back => "Backspace",
        Key.Delete => "Delete",
        Key.Left => "ArrowLeft",
        Key.Right => "ArrowRight",
        Key.Up => "ArrowUp",
        Key.Down => "ArrowDown",
        Key.Home => "Home",
        Key.End => "End",
        Key.PageUp => "PageUp",
        Key.PageDown => "PageDown",
        Key.LeftShift or Key.RightShift => "Shift",
        Key.LeftCtrl or Key.RightCtrl => "Control",
        Key.LeftAlt or Key.RightAlt => "Alt",
        _ => null,
    };
}

/// <summary>The keys that are down, so each goes down once however long it repeats.</summary>
public sealed class HeldKeys
{
    readonly List<string> held = [];

    /// <summary>Whether <paramref name="name"/> went down now, rather than repeating.</summary>
    public bool Down(string name)
    {
        if (held.Contains(name))
            return false;
        held.Add(name);
        return true;
    }

    /// <summary>Whether <paramref name="name"/> was down.</summary>
    public bool Up(string name) => held.Remove(name);

    /// <summary>Lets go of every key: the canvas lost the keyboard.</summary>
    public List<string> All()
    {
        var all = held.ToList();
        held.Clear();
        return all;
    }
}
