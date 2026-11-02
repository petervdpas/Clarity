// A shortcut as Clarity writes it (`Ctrl+S`, `F2`, `Ctrl+Shift+N`) against a
// key the person pressed, as the shared screen matches one
// (runtimes/web/src/screen/shortcut.ts): the modifiers exactly, letters in
// either case, Shift for a symbol only when written (it makes + and *),
// and a typed key without Ctrl, Alt or Meta left to an input the person
// types in. A symbol is the same key from the keyboard or the number pad.

namespace Clarity.Ui;

public static class Shortcuts
{
    /// <summary>Names of keys in a shortcut that differ from Clarity.UI's own.</summary>
    static readonly Dictionary<string, Key> Named = new(StringComparer.OrdinalIgnoreCase)
    {
        ["Esc"] = Key.Escape,
        ["Return"] = Key.Enter,
        ["Del"] = Key.Delete,
        ["Ins"] = Key.Insert,
        ["PgUp"] = Key.PageUp,
        ["PgDn"] = Key.PageDown,
    };

    /// <summary>The key of each symbol a shortcut or a browser writes.</summary>
    public static readonly IReadOnlyDictionary<char, Key> Symbols = new Dictionary<char, Key>
    {
        ['+'] = Key.Plus,
        ['-'] = Key.Minus,
        ['*'] = Key.Multiply,
        ['/'] = Key.Slash,
        ['.'] = Key.Period,
        [','] = Key.Comma,
        ['='] = Key.Equal,
        [';'] = Key.Semicolon,
        ['\''] = Key.Quote,
        ['\\'] = Key.Backslash,
        ['['] = Key.BracketLeft,
        [']'] = Key.BracketRight,
        ['`'] = Key.Backquote,
    };

    /// <summary>Whether a key is a symbol: Shift makes some (+, *), so it only counts for one when written.</summary>
    public static bool IsSymbol(Key key) => key >= Key.Plus;

    /// <summary>The modifiers of a shortcut and its key: <c>Ctrl++</c> is Ctrl and <c>+</c>.</summary>
    public static (IReadOnlyList<string> Modifiers, string Key) Parts(string shortcut)
    {
        if (shortcut == "+")
            return ([], "+");
        if (shortcut.Length > 2 && shortcut.EndsWith("++", StringComparison.Ordinal))
            return (shortcut[..^2].Split('+'), "+");
        var keys = shortcut.Split('+');
        return (keys[..^1], keys[^1]);
    }

    /// <summary>The key of a shortcut's last word; Other for one Clarity.UI has no key for (a symbol such as <c>+</c>).</summary>
    public static Key KeyOf(string word)
    {
        if (word.Length == 1 && char.IsAsciiLetter(word[0]))
            return Key.A + (char.ToUpperInvariant(word[0]) - 'A');
        if (word.Length == 1 && char.IsAsciiDigit(word[0]))
            return Key.D0 + (word[0] - '0');
        if (word.Length == 1 && Symbols.TryGetValue(word[0], out var symbol))
            return symbol;
        if (Named.TryGetValue(word, out var named))
            return named;
        return word.Length > 1 && Enum.TryParse<Key>(word, ignoreCase: true, out var key) && key is not (Key.Other or >= Key.A) ? key : Key.Other;
    }

    /// <summary>Whether <paramref name="shortcut"/> is the key pressed; <paramref name="typing"/> says the focus is in an input the person types in.</summary>
    public static bool Matches(string shortcut, KeyInput key, bool typing)
    {
        var (modifiers, word) = Parts(shortcut.Trim());
        var wanted = KeyOf(word);
        if (wanted == Key.Other || wanted != key.Key)
            return false;
        bool Held(string name) => modifiers.Any(m => string.Equals(m, name, StringComparison.OrdinalIgnoreCase));
        var control = Held("Ctrl") || Held("Control");
        if (key.Control != control || key.Modifiers.HasFlag(Modifiers.Alt) != Held("Alt") || key.Modifiers.HasFlag(Modifiers.Meta) != (Held("Meta") || Held("Cmd")))
            return false;
        if (!(IsSymbol(wanted) && !Held("Shift")) && key.Shift != Held("Shift"))
            return false;
        // A letter, digit or symbol without Ctrl, Alt or Meta is what the person types in an input.
        var typed = wanted is >= Key.A;
        return !(typed && typing && !control && !Held("Alt") && !Held("Meta"));
    }
}
