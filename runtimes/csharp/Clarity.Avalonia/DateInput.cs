// Clarity's date input on Avalonia, the same as on the web
// (conformance/forms.json, tables.json): a day picked from the calendar or
// typed, and only another day picked by the person is a change.

using Avalonia;
using Avalonia.Controls;
using Avalonia.Data;

namespace Clarity.Avalonia;

public class DateInput : CalendarDatePicker, IInput
{
    public static readonly StyledProperty<DateOnly?> ValueProperty =
        AvaloniaProperty.Register<DateInput, DateOnly?>(nameof(Value), defaultBindingMode: BindingMode.TwoWay);

    /// <summary>The day, or none.</summary>
    public DateOnly? Value
    {
        get => GetValue(ValueProperty);
        set => SetValue(ValueProperty, value);
    }

    public event EventHandler? ValueChanged;

    protected override Type StyleKeyOverride => typeof(CalendarDatePicker);

    /// <summary>Set while the picker follows <see cref="Value"/>, so that is no change by the person.</summary>
    bool showing;

    protected override void OnPropertyChanged(AvaloniaPropertyChangedEventArgs change)
    {
        base.OnPropertyChanged(change);
        if (change.Property == ValueProperty)
        {
            showing = true;
            SelectedDate = Value?.ToDateTime(TimeOnly.MinValue);
            showing = false;
        }
        else if (change.Property == SelectedDateProperty && !showing)
        {
            var next = SelectedDate is { } day ? DateOnly.FromDateTime(day) : (DateOnly?)null;
            if (next == Value)
                return;
            Value = next;
            ValueChanged?.Invoke(this, EventArgs.Empty);
        }
    }
}
