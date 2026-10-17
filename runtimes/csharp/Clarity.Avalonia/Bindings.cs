// What the generated windows bind with: the choices of a dropdown, the
// converters between record values and controls, and events that a control
// has no command for.

using System.Globalization;
using Avalonia.Controls;
using Avalonia.Data;
using Avalonia.Data.Converters;
using Avalonia.Input;
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
}

public static class Converters
{
    /// <summary>A record's date (DateOnly?) as a date picker's (DateTime?).</summary>
    public static readonly IValueConverter Date = new FuncValueConverter<DateOnly?, DateTime?>(
        d => d?.ToDateTime(TimeOnly.MinValue),
        t => t is { } v ? DateOnly.FromDateTime(v) : null);

    /// <summary>
    /// A radio button of one value: checked while the bound value is the
    /// converter parameter, and setting it when the button is checked.
    /// </summary>
    public static readonly IValueConverter Equal = new EqualConverter();

    sealed class EqualConverter : IValueConverter
    {
        public object? Convert(object? value, Type targetType, object? parameter, CultureInfo culture) => Equals(value, parameter);

        public object? ConvertBack(object? value, Type targetType, object? parameter, CultureInfo culture) =>
            value is true ? parameter : BindingOperations.DoNothing;
    }
}

/// <summary>
/// A value as text, the way Clarity shows it plainly: labels for enums (from
/// the application's own <c>Labels</c>), and the runtime's text for the rest.
/// </summary>
public sealed class ShownConverter(Func<object?, string?> labels) : IValueConverter
{
    public object? Convert(object? value, Type targetType, object? parameter, CultureInfo culture) =>
        labels(value) ?? value switch
        {
            null => "",
            string s => s,
            long n => Show.Of(n),
            decimal d => Show.Of(d),
            bool b => Show.Of(b),
            DateOnly d => Show.Of(d),
            DateTime d => Show.Of(d),
            _ => value.ToString(),
        };

    public object? ConvertBack(object? value, Type targetType, object? parameter, CultureInfo culture) => BindingOperations.DoNothing;
}

public static class Events
{
    /// <summary>
    /// Runs <paramref name="commit"/> when an edit of the text box is done: focus
    /// leaves it, or Enter is pressed. Clarity's <c>change</c> of a text input.
    /// </summary>
    public static void OnCommit(TextBox box, Action commit)
    {
        box.LostFocus += (_, _) => commit();
        box.KeyDown += (_, e) =>
        {
            if (e.Key == Key.Enter)
                commit();
        };
    }

    /// <summary>Runs <paramref name="change"/> when a radio button of the group is checked: Clarity's <c>change</c> of a radio group.</summary>
    public static void OnChoice(Panel group, Action change)
    {
        foreach (var button in group.Children.OfType<RadioButton>())
            button.IsCheckedChanged += (_, _) =>
            {
                if (button.IsChecked == true)
                    change();
            };
    }

    /// <summary>Runs <paramref name="submit"/> when Enter is pressed in a form: Clarity's <c>submit</c>.</summary>
    public static void OnEnter(Control form, Action submit) =>
        form.AddHandler(InputElement.KeyDownEvent, (_, e) =>
        {
            if (e.Key == Key.Enter)
                submit();
        }, global::Avalonia.Interactivity.RoutingStrategies.Bubble);
}
