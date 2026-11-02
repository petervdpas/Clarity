// A value as the person reads it, outside any part: in a search dropdown's
// cells, as the screen shows a value of a field (conformance/formats.json,
// list-fields.json). An enum's value shows by its label, which the
// generated code gives (Labels.Of); everything else shows here.

namespace Clarity.Runtime;

public static class ShownText
{
    /// <summary>
    /// <paramref name="value"/> as text: numbers and dates in the person's
    /// format, or in <paramref name="format"/> where the field has one; yes
    /// and no in the person's language; a list as its items, each as it is shown.
    /// </summary>
    public static string Of(object? value, FieldFormat? format = null) => value switch
    {
        null => "",
        string s => s,
        bool b => b ? Locale.Translate("yes", "Yes") : Locale.Translate("no", "No"),
        long or int => Locale.Formats.Show(format ?? new FieldFormat(FormatKind.Integer), value),
        decimal => Locale.Formats.Show(format ?? new FieldFormat(FormatKind.Decimal), value),
        DateOnly => Locale.Formats.Show(format ?? new FieldFormat(FormatKind.Date), value),
        DateTime => Locale.Formats.Show(format ?? new FieldFormat(FormatKind.DateTime), value),
        System.Collections.IList items => string.Join(", ", items.Cast<object?>().Select(i => Of(i))),
        _ => value.ToString() ?? "",
    };
}
