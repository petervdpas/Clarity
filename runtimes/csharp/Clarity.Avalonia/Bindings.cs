// What the generated windows bind with: the choices of a dropdown, the
// converters between record values and controls, and events that a control
// has no command for.

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
            bool b => Locale.Translate(b ? "Yes" : "No"),
            long or int => Locale.Formats.Show(format ?? new FieldFormat(FormatKind.Integer), value),
            decimal => Locale.Formats.Show(format ?? new FieldFormat(FormatKind.Decimal), value),
            DateOnly => Locale.Formats.Show(format ?? new FieldFormat(FormatKind.Date), value),
            DateTime => Locale.Formats.Show(format ?? new FieldFormat(FormatKind.DateTime), value),
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

/// <summary>
/// A number input: the value as typed in the person's format (money without
/// its sign), and what is typed read back the same way. Text that is no
/// number of the field's kind marks the input, as on the web, and the value
/// stays; a cleared input is zero, or no value where the field may have none.
/// </summary>
public sealed class EditConverter(FieldFormat format) : IValueConverter
{
    public object? Convert(object? value, Type targetType, object? parameter, CultureInfo culture) => Locale.Formats.EditText(format, value);

    public object? ConvertBack(object? value, Type targetType, object? parameter, CultureInfo culture)
    {
        var text = value as string ?? "";
        if (text.Trim().Length == 0)
            return Nullable.GetUnderlyingType(targetType) is not null || !targetType.IsValueType ? null : Zero(targetType);
        return Locale.Formats.Read(format, text) switch
        {
            null => new BindingNotification(new FormatException(""), BindingErrorType.DataValidationError),
            long n when Underlying(targetType) == typeof(int) => (int)n,
            var read => read,
        };
    }

    static Type Underlying(Type type) => Nullable.GetUnderlyingType(type) ?? type;

    static object Zero(Type type) => Underlying(type) == typeof(decimal) ? 0m : Underlying(type) == typeof(int) ? 0 : 0L;
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

    /// <summary>
    /// Runs <paramref name="edited"/> with the row whose edited column
    /// changed: Clarity's <c>change</c> of a table. It watches the records, so
    /// every kind of cell counts alike and only a value that differs is an
    /// edit, as on the web: a check box or dropdown when it changes, a text
    /// cell when it is committed (focus leaves it, or Enter). What a handler
    /// changes is no edit, and neither is a row being drawn.
    /// </summary>
    public static void OnCellEdit(DataGrid table, Func<bool> handling, IReadOnlyCollection<string> columns, Action<object> edited)
    {
        var watched = new List<INotifyPropertyChanged>();
        void Changed(object? row, PropertyChangedEventArgs e)
        {
            if (row is not null && e.PropertyName is { } name && columns.Contains(name) && !handling())
                edited(row);
        }
        void Watch(System.Collections.IEnumerable? rows)
        {
            foreach (var row in watched)
                row.PropertyChanged -= Changed;
            watched.Clear();
            foreach (var row in rows?.OfType<INotifyPropertyChanged>() ?? [])
            {
                row.PropertyChanged += Changed;
                watched.Add(row);
            }
        }
        Watch(table.ItemsSource);
        table.PropertyChanged += (_, e) =>
        {
            if (e.Property == DataGrid.ItemsSourceProperty)
                Watch(table.ItemsSource);
        };
        // Enter commits a text cell, as leaving it does.
        table.AddHandler(InputElement.KeyDownEvent, (_, e) =>
        {
            if (e.Key == Key.Enter && e.Source is TextBox box)
                BindingOperations.GetBindingExpressionBase(box, TextBox.TextProperty)?.UpdateSource();
        }, RoutingStrategies.Bubble);
    }

    /// <summary>Runs <paramref name="submit"/> when Enter is pressed in a form: Clarity's <c>submit</c>.</summary>
    public static void OnEnter(Control form, Action submit) =>
        form.AddHandler(InputElement.KeyDownEvent, (_, e) =>
        {
            if (e.Key == Key.Enter)
                submit();
        }, global::Avalonia.Interactivity.RoutingStrategies.Bubble);
}
