// A dropdown of choices (an enum's values, or the records a reference can
// point at) whose value changes only when the person picks one. A plain
// ComboBox bound to a value clears it when its choices come after the
// value, which in a table cell would be an edit nobody made.

using Avalonia;
using Avalonia.Controls;
using Avalonia.Data;
using Avalonia.Layout;

namespace Clarity.Avalonia;

public sealed class ChoiceInput : UserControl, IInput
{
    public static readonly StyledProperty<object?> ValueProperty =
        AvaloniaProperty.Register<ChoiceInput, object?>(nameof(Value), defaultBindingMode: BindingMode.TwoWay);

    public static readonly StyledProperty<System.Collections.IEnumerable?> ChoicesProperty =
        AvaloniaProperty.Register<ChoiceInput, System.Collections.IEnumerable?>(nameof(Choices));

    /// <summary>The chosen value: an enum value, a record's id, or none.</summary>
    public object? Value
    {
        get => GetValue(ValueProperty);
        set => SetValue(ValueProperty, value);
    }

    /// <summary>The choices (<see cref="Choice"/>), in order. Untyped, as ItemsSource is, so AXAML can set an array.</summary>
    public System.Collections.IEnumerable? Choices
    {
        get => GetValue(ChoicesProperty);
        set => SetValue(ChoicesProperty, value);
    }

    /// <summary>The person picked another choice: Clarity's <c>change</c>.</summary>
    public event EventHandler? ValueChanged;

    public ComboBox Combo { get; } = new() { HorizontalAlignment = HorizontalAlignment.Stretch, DisplayMemberBinding = new Binding(nameof(Choice.Label)) };

    bool showing;

    public ChoiceInput()
    {
        Content = Combo;
        Combo.SelectionChanged += (_, _) =>
        {
            if (showing)
                return;
            var next = (Combo.SelectedItem as Choice)?.Value;
            if (Equals(next, Value))
                return;
            Value = next;
            ValueChanged?.Invoke(this, EventArgs.Empty);
        };
    }

    protected override void OnPropertyChanged(AvaloniaPropertyChangedEventArgs change)
    {
        base.OnPropertyChanged(change);
        if (change.Property != ValueProperty && change.Property != ChoicesProperty)
            return;
        showing = true;
        if (change.Property == ChoicesProperty)
            Combo.ItemsSource = Choices?.OfType<Choice>().ToList();
        Combo.SelectedItem = Choices?.OfType<Choice>().FirstOrDefault(c => Equals(c.Value, Value));
        showing = false;
    }
}
