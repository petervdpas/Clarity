// The dropdown's options of U1c, the same as on the web
// (conformance/dropdown-options.json). A `typed` dropdown offers the values
// its field already has: each once, without empty text, those containing
// the typed text (ignoring case), sorted ignoring case and then as written.
// A `search` dropdown offers the records whose text or shown fields hold
// every typed word, ignoring case, in the order they came.

namespace Clarity.Avalonia;

public static class DropdownOptions
{
    static readonly char[] Spaces = [' ', '\t', '\r', '\n'];

    /// <summary>The suggestions of a typed dropdown for what is typed.</summary>
    public static List<string> Suggestions(IEnumerable<string> values, string typed)
    {
        var wanted = typed.Trim();
        return
        [
            .. values
                .Where(v => v != "" && v.Contains(wanted, StringComparison.OrdinalIgnoreCase))
                .Distinct(StringComparer.Ordinal)
                .OrderBy(v => v.ToLowerInvariant(), StringComparer.Ordinal)
                .ThenBy(v => v, StringComparer.Ordinal),
        ];
    }

    /// <summary>The records a search dropdown offers for what is typed.</summary>
    public static List<Choice> Searched(IEnumerable<Choice> choices, string typed)
    {
        var words = typed.Split(Spaces, StringSplitOptions.RemoveEmptyEntries);
        return
        [
            .. choices.Where(c =>
            {
                string[] texts = [c.Label, .. c.Cells ?? []];
                return words.All(w => texts.Any(t => t.Contains(w, StringComparison.OrdinalIgnoreCase)));
            }),
        ];
    }
}
