// A moment (a datetime field) as a day picked from a calendar and a time
// typed as hh:mm, side by side, like the web's date-and-time input. One
// control for forms and table cells.

using System.Globalization;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Data;
using Avalonia.Input;
using Avalonia.Layout;

namespace Clarity.Avalonia;

public sealed class DateTimeBox : UserControl
{
    public static readonly StyledProperty<DateTime?> ValueProperty =
        AvaloniaProperty.Register<DateTimeBox, DateTime?>(nameof(Value), defaultBindingMode: BindingMode.TwoWay);

    /// <summary>The moment, or none.</summary>
    public DateTime? Value
    {
        get => GetValue(ValueProperty);
        set => SetValue(ValueProperty, value);
    }

    /// <summary>The person picked another day or typed another time: Clarity's <c>change</c>.</summary>
    public event EventHandler? ValueChanged;

    public CalendarDatePicker Day { get; } = new() { HorizontalAlignment = HorizontalAlignment.Stretch };
    public TextBox Time { get; } = new() { PlaceholderText = "hh:mm", Width = 64, HorizontalContentAlignment = HorizontalAlignment.Center };

    /// <summary>Set while the parts follow <see cref="Value"/>, so that is no change by the person.</summary>
    bool showing;

    public DateTimeBox()
    {
        var grid = new Grid { ColumnDefinitions = new ColumnDefinitions("*,Auto"), ColumnSpacing = 4 };
        Grid.SetColumn(Time, 1);
        grid.Children.Add(Day);
        grid.Children.Add(Time);
        Content = grid;
        Day.PropertyChanged += (_, e) =>
        {
            if (e.Property == CalendarDatePicker.SelectedDateProperty && !showing)
                Take(Day.SelectedDate?.Date + (Value?.TimeOfDay ?? TimeSpan.Zero));
        };
        Time.LostFocus += (_, _) => TypedTime();
        Time.KeyDown += (_, e) =>
        {
            if (e.Key == Key.Enter)
                TypedTime();
        };
    }

    protected override void OnPropertyChanged(AvaloniaPropertyChangedEventArgs change)
    {
        base.OnPropertyChanged(change);
        if (change.Property == ValueProperty)
            Show();
    }

    void Show()
    {
        showing = true;
        Day.SelectedDate = Value?.Date;
        Time.Text = Value is { } v ? v.ToString("HH:mm", CultureInfo.InvariantCulture) : "";
        Time.Classes.Remove("error");
        showing = false;
    }

    /// <summary>The typed time: hh:mm, or h:mm. Without a day it waits for one.</summary>
    void TypedTime()
    {
        var text = Time.Text?.Trim() ?? "";
        if (!TimeSpan.TryParseExact(text, ["h\\:mm", "hh\\:mm"], CultureInfo.InvariantCulture, out var time) || time.TotalHours >= 24)
        {
            if (text.Length > 0)
            {
                Time.Classes.Add("error");
                return;
            }
            time = TimeSpan.Zero;
        }
        Time.Classes.Remove("error");
        if (Value is { } v)
            Take(v.Date + time);
    }

    void Take(DateTime? next)
    {
        if (next == Value)
        {
            Show();
            return;
        }
        Value = next;
        ValueChanged?.Invoke(this, EventArgs.Empty);
    }
}
