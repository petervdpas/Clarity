// What the generated windows bind with: the choices of a dropdown or radio
// group, the converters that show values in the person's format, and the
// form's submit, which no control has a command for.

using System.ComponentModel;
using System.Globalization;
using Avalonia.Controls;
using Avalonia.Controls.Primitives;
using Avalonia.Data;
using Avalonia.Data.Converters;
using Avalonia.Input;
using Avalonia.Interactivity;
using Clarity.Runtime;

namespace Clarity.Avalonia;

/// <summary>One choice of a dropdown or radio group: the value stored and the label shown.</summary>
public sealed record Choice(object? Value, string Label)
{
    /// <summary>The records as choices, after an empty one for no reference.</summary>
    public static Choice[] Of<R>(IEnumerable<R> records, Func<R, string> label, Func<R, long> id) =>
        [new(null, ""), .. records.Select(r => new Choice(id(r), label(r)))];

    /// <summary>The records of an entity as the choices of a reference.</summary>
    public static Choice[] Of<R>(IEnumerable<R> records, Func<R, string> label) where R : IRecord<R> =>
        Of(records, label, r => r.Id);

    /// <summary>The project's languages, each by its own name: the choices of the setting <c>language</c>.</summary>
    public static Choice[] LanguagesOffered => [.. Locale.Offered.Select(l => new Choice(l, Languages.Name(l)))];

    /// <summary>The offered themes by their titles in the language shown: the choices of the setting <c>theme</c>.</summary>
    public static Choice[] ThemesOffered => [.. Themes.Offered.Select(t => new Choice(t.Name, Locale.Translate(t.Label)))];
}

/// <summary>
/// A value as text, the way the person reads it (Locale): enums by their
/// labels (from the application's own <c>Labels</c>), yes and no in the
/// language shown, numbers and dates in the person's format, by the field's
/// <paramref name="format"/> when it has one.
/// </summary>
public sealed class ShownConverter(Func<object?, string?> labels, FieldFormat? format = null) : IValueConverter
{
    public object? Convert(object? value, Type targetType, object? parameter, CultureInfo culture) =>
        labels(value) ?? value switch
        {
            null => "",
            string s => s,
            bool b => b ? Locale.Translate("yes", "Yes") : Locale.Translate("no", "No"),
            long or int => Locale.Formats.Show(format ?? new FieldFormat(FormatKind.Integer), value),
            decimal => Locale.Formats.Show(format ?? new FieldFormat(FormatKind.Decimal), value),
            DateOnly => Locale.Formats.Show(format ?? new FieldFormat(FormatKind.Date), value),
            DateTime => Locale.Formats.Show(format ?? new FieldFormat(FormatKind.DateTime), value),
            // A list field: its items, each as it is shown (conformance/list-fields.json).
            System.Collections.IList items => string.Join(", ", items.Cast<object?>().Select(i => Convert(i, targetType, parameter, culture))),
            _ => value.ToString(),
        };

    public object? ConvertBack(object? value, Type targetType, object? parameter, CultureInfo culture) => BindingOperations.DoNothing;
}

/// <summary>A value of a field with a format, shown in the person's format: a table cell, a text part.</summary>
public sealed class FormatConverter(FieldFormat format) : IValueConverter
{
    public object? Convert(object? value, Type targetType, object? parameter, CultureInfo culture) => Locale.Formats.Show(format, value);

    public object? ConvertBack(object? value, Type targetType, object? parameter, CultureInfo culture) => BindingOperations.DoNothing;
}

public static class Events
{
    /// <summary>Runs <paramref name="submit"/> when Enter is pressed in a form: Clarity's <c>submit</c>.</summary>
    public static void OnEnter(Control form, Action submit) =>
        form.AddHandler(InputElement.KeyDownEvent, (_, e) =>
        {
            if (e.Key == Key.Enter)
                submit();
        }, global::Avalonia.Interactivity.RoutingStrategies.Bubble);
}
