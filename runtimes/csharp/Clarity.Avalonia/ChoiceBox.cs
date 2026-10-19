// A dropdown of choices (an enum's values, or the records a reference can
// point at) whose value changes only when the person picks one. A plain
// ComboBox bound to a value clears it when its choices come after the
// value, which in a table cell would be an edit nobody made.

using Avalonia;
using Avalonia.Controls;
using Avalonia.Data;
using Avalonia.Layout;

namespace Clarity.Avalonia;

public sealed class ChoiceBox : UserControl
{
    public static readonly StyledProperty<object?> ValueProperty =
        AvaloniaProperty.Register<ChoiceBox, object?>(nameof(Value), defaultBindingMode: BindingMode.TwoWay);

    public static readonly StyledProperty<IEnumerable<Choice>?> ChoicesProperty =
        AvaloniaProperty.Register<ChoiceBox, IEnumerable<Choice>?>(nameof(Choices));

    /// <summary>The chosen value: an enum value, a record's id, or none.</summary>
    public object? Value
    {
        get => GetValue(ValueProperty);
        set => SetValue(ValueProperty, value);
    }

    public IEnumerable<Choice>? Choices
    {
        get => GetValue(ChoicesProperty);
        set => SetValue(ChoicesProperty, value);
    }

    /// <summary>The person picked another choice: Clarity's <c>change</c>.</summary>
    public event EventHandler? ValueChanged;

    public ComboBox Combo { get; } = new() { HorizontalAlignment = HorizontalAlignment.Stretch, DisplayMemberBinding = new Binding(nameof(Choice.Label)) };

    bool showing;

    public ChoiceBox()
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
            Combo.ItemsSource = Choices?.ToList();
        Combo.SelectedItem = Choices?.FirstOrDefault(c => Equals(c.Value, Value));
        showing = false;
    }
}
