// Clarity's text input on Avalonia: a line of text, a password or a memo
// (AcceptsReturn), the same as on the web (conformance/forms.json). Typed
// text is taken when the input is left or Enter is pressed (a memo: only
// when it is left, Enter is a new line); only text that differs from the
// value is a change.

using Avalonia;
using Avalonia.Controls;
using Avalonia.Data;
using Avalonia.Input;
using Avalonia.Interactivity;

namespace Clarity.Avalonia;

public class TextInput : TextBox, IInput
{
    public static readonly StyledProperty<string?> ValueProperty =
        AvaloniaProperty.Register<TextInput, string?>(nameof(Value), defaultBindingMode: BindingMode.TwoWay);

    /// <summary>The text as taken; what is typed is in <see cref="TextBox.Text"/> until then.</summary>
    public string? Value
    {
        get => GetValue(ValueProperty);
        set => SetValue(ValueProperty, value);
    }

    public event EventHandler? ValueChanged;

    protected override Type StyleKeyOverride => typeof(TextBox);

    public TextInput()
    {
        LostFocus += (_, _) => Take();
        // Before the form sees the Enter, so a submit finds the text taken;
        // also when a table took the Enter already, so it does not move rows.
        AddHandler(KeyDownEvent, (_, e) =>
        {
            if (e.Key == Key.Enter && !AcceptsReturn)
                Take();
        }, RoutingStrategies.Tunnel, handledEventsToo: true);
    }

    protected override void OnPropertyChanged(AvaloniaPropertyChangedEventArgs change)
    {
        base.OnPropertyChanged(change);
        if (change.Property == ValueProperty)
            Text = Value ?? "";
    }

    /// <summary>Takes what is typed: a change when it differs from the value.</summary>
    public void Take()
    {
        var typed = Text ?? "";
        if (typed == (Value ?? ""))
            return;
        Value = typed;
        ValueChanged?.Invoke(this, EventArgs.Empty);
    }
}
