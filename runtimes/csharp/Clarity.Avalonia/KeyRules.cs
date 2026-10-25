// Keys set by the person (DESIGN 5.28), as every renderer sets them
// (conformance/shortcut-keys.json): a command's keys are the person's or the
// design's; a key taken in a place they share swaps; Reset swaps back; the
// keys the system and the inputs need are refused. Keys are written as
// Clarity writes them: `Ctrl+Shift+N`.

using Avalonia.Input;

namespace Clarity.Avalonia;

public static class KeyRules
{
    /// <summary>A command: its id, the places it is active in ("" is the app's frame) and the keys its design gives it.</summary>
    public sealed record Command(string Id, IReadOnlyList<string> Layers, IReadOnlyList<string> Keys);

    /// <summary>A command's keys: the person's, or the design's.</summary>
    public static IReadOnlyList<string> KeysOf(Command command, IReadOnlyDictionary<string, IReadOnlyList<string>> choices) =>
        choices.TryGetValue(command.Id, out var keys) ? keys : command.Keys;

    static bool Share(Command a, Command b) => a.Layers.Any(b.Layers.Contains);

    /// <summary>
    /// Gives <paramref name="id"/> the key: a command that has it in a place they share gets the keys
    /// this one had (<c>Swapped</c>); what is kept is only what differs from the design.
    /// </summary>
    public static (Dictionary<string, IReadOnlyList<string>> Choices, string? Swapped) Assign(
        IReadOnlyList<Command> commands, IReadOnlyDictionary<string, IReadOnlyList<string>> choices, string id, string key)
    {
        var command = commands.FirstOrDefault(c => c.Id == id);
        if (command is null)
            return (new(choices), null);
        var keys = commands.ToDictionary(c => c.Id, c => KeysOf(c, choices).ToList());
        var old = keys[id].ToList();
        var other = commands.FirstOrDefault(c => c.Id != id && Share(c, command) && keys[c.Id].Contains(key));
        if (other is not null)
            keys[other.Id] = [.. keys[other.Id].Where(k => k != key), .. old.Where(k => !keys[other.Id].Contains(k))];
        keys[id] = [key];
        var next = new Dictionary<string, IReadOnlyList<string>>();
        foreach (var c in commands)
        {
            if (!keys[c.Id].SequenceEqual(c.Keys))
                next[c.Id] = keys[c.Id];
        }
        return (next, other?.Id);
    }

    /// <summary>Puts a command's keys back to the design's: a command that took one of them in a swap gets its own back.</summary>
    public static Dictionary<string, IReadOnlyList<string>> Reset(
        IReadOnlyList<Command> commands, IReadOnlyDictionary<string, IReadOnlyList<string>> choices, string id)
    {
        var command = commands.FirstOrDefault(c => c.Id == id);
        var next = new Dictionary<string, IReadOnlyList<string>>(choices);
        if (command is null)
            return next;
        foreach (var key in command.Keys)
            next = Assign(commands, next, id, key).Choices;
        next.Remove(id);
        return next;
    }

    /// <summary>The commands of other places that have the key too: allowed, and the inner one goes first where both are active.</summary>
    public static IReadOnlyList<string> AlsoHas(
        IReadOnlyList<Command> commands, IReadOnlyDictionary<string, IReadOnlyList<string>> choices, string id, string key)
    {
        var command = commands.FirstOrDefault(c => c.Id == id);
        if (command is null)
            return [];
        return [.. commands.Where(c => c.Id != id && !Share(c, command) && KeysOf(c, choices).Contains(key)).Select(c => c.Id)];
    }

    /// <summary>The modifiers of a key and the key itself: <c>Ctrl++</c> is Ctrl and <c>+</c>.</summary>
    public static (IReadOnlyList<string> Modifiers, string Key) Parts(string key)
    {
        if (key.EndsWith("++", StringComparison.Ordinal))
            return ([.. key[..^2].Split('+', StringSplitOptions.RemoveEmptyEntries)], "+");
        var all = key.Split('+');
        return (all[..^1], all[^1]);
    }

    /// <summary>Why a key cannot be a command's: typing, tab, enter or escape; null when it can.</summary>
    public static string? Refused(string key)
    {
        var (held, last) = Parts(key);
        switch (last)
        {
            case "Tab":
                return "tab";
            case "Enter":
                return "enter";
            case "Escape":
                return "escape";
        }
        var plain = !held.Any(m => m is "Ctrl" or "Alt" or "Meta");
        return plain && (last.Length == 1 || last == "Space") ? "typing" : null;
    }

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
        var (held, last) = Parts(written);
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
