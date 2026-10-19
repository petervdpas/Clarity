// Clarity's number input on Avalonia, the same as on the web
// (conformance/forms.json, tables.json): the value shows in the person's
// format and typed text is read the same way, when the input is left or
// Enter is pressed. Text that is no number stays as typed and is marked,
// and the value stays; a cleared input is zero; only a number that differs
// is a change.

using Avalonia;
using Avalonia.Controls;
using Avalonia.Data;
using Avalonia.Input;
using Avalonia.Interactivity;
using Clarity.Runtime;

namespace Clarity.Avalonia;

public class NumberInput : TextBox, IInput
{
    public static readonly StyledProperty<object?> ValueProperty =
        AvaloniaProperty.Register<NumberInput, object?>(nameof(Value), defaultBindingMode: BindingMode.TwoWay);

    /// <summary>The number: a long for whole numbers and durations, a decimal for the rest.</summary>
    public object? Value
    {
        get => GetValue(ValueProperty);
        set => SetValue(ValueProperty, value);
    }

    /// <summary>The field's kind and format, which say how the number shows and reads.</summary>
    public FieldFormat Format { get; set; } = new(FormatKind.Decimal);

    public event EventHandler? ValueChanged;

    protected override Type StyleKeyOverride => typeof(TextBox);

    public NumberInput()
    {
        Classes.Add("number");
        LostFocus += (_, _) => Take();
        AddHandler(KeyDownEvent, (_, e) =>
        {
            if (e.Key == Key.Enter)
                Take();
        }, RoutingStrategies.Tunnel, handledEventsToo: true);
    }

    protected override void OnPropertyChanged(AvaloniaPropertyChangedEventArgs change)
    {
        base.OnPropertyChanged(change);
        if (change.Property == ValueProperty)
        {
            Text = Locale.Formats.EditText(Format, Value);
            Classes.Remove("error");
        }
    }

    /// <summary>Takes what is typed: a change when it is a number that differs from the value.</summary>
    public void Take()
    {
        var typed = Text ?? "";
        var next = typed.Trim().Length == 0 ? Zero() : Locale.Formats.Read(Format, typed);
        if (next is null)
        {
            Classes.Add("error");
            return;
        }
        Classes.Remove("error");
        if (Equals(next, Value))
            return;
        Value = next;
        ValueChanged?.Invoke(this, EventArgs.Empty);
    }

    /// <summary>A number field always has a value; cleared, it is zero.</summary>
    object Zero() => Format.Kind is FormatKind.Integer or FormatKind.Duration ? 0L : 0m;
}
