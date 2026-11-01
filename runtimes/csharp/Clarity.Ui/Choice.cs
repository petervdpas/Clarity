// One choice of a dropdown or radio group: the value stored and the label shown.

using Clarity.Runtime;

namespace Clarity.Ui;

/// <summary>One choice of a dropdown or radio group: the value stored and the label shown.</summary>
public sealed record Choice(object? Value, string Label, IReadOnlyList<string>? Cells = null)
{
    /// <summary>The records as the choices of a search dropdown (U1c), each with the texts of the fields it shows.</summary>
    public static Choice[] Of<R>(IEnumerable<R> records, Func<R, string> label, Func<R, string[]> cells) where R : IRecord<R> =>
        [.. records.Select(r => new Choice(r.Id, label(r), cells(r)))];

    /// <summary>The records as choices, after an empty one for no reference.</summary>
    public static Choice[] Of<R>(IEnumerable<R> records, Func<R, string> label, Func<R, long> id) =>
        [new(null, ""), .. records.Select(r => new Choice(id(r), label(r)))];

    /// <summary>The records of an entity as the choices of a reference.</summary>
    public static Choice[] Of<R>(IEnumerable<R> records, Func<R, string> label) where R : IRecord<R> =>
        Of(records, label, r => r.Id);

    /// <summary>The project's languages, each by its own name: the choices of the setting <c>language</c>.</summary>
    public static Choice[] LanguagesOffered => [.. Locale.Offered.Select(l => new Choice(l, Languages.Name(l)))];

    /// <summary>The offered themes by their titles in the language shown: the choices of the setting <c>theme</c>.</summary>
    public static Choice[] ThemesOffered => [.. OfferedThemes.All.Select(t => new Choice(t.Name, Locale.Translate(t.Label)))];
}
