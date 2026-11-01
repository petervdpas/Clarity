// Avalonia's keys as the keys of a number range (Clarity.Ui's NumberRange).

namespace Clarity.Avalonia;

public static class RangeKeys
{
    /// <summary>The range key a key press is, when it is one.</summary>
    public static RangeKey? Of(global::Avalonia.Input.Key key) => key switch
    {
        global::Avalonia.Input.Key.Up or global::Avalonia.Input.Key.Right => RangeKey.Up,
        global::Avalonia.Input.Key.Down or global::Avalonia.Input.Key.Left => RangeKey.Down,
        global::Avalonia.Input.Key.PageUp => RangeKey.PageUp,
        global::Avalonia.Input.Key.PageDown => RangeKey.PageDown,
        global::Avalonia.Input.Key.Home => RangeKey.Home,
        global::Avalonia.Input.Key.End => RangeKey.End,
        _ => null,
    };
}
