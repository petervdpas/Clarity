// Clarity's checkbox and switch on Avalonia, the same as on the web
// (conformance/forms.json, tables.json): only the person ticking one is a
// change; what the code sets on IsChecked is not.

using Avalonia.Controls;

namespace Clarity.Avalonia;

/// <summary>A checkbox: its value is <c>IsChecked</c>.</summary>
public class CheckInput : CheckBox, IInput
{
    public event EventHandler? ValueChanged;

    protected override Type StyleKeyOverride => typeof(CheckBox);

    /// <summary>Only a click or a key of the person toggles.</summary>
    protected override void Toggle()
    {
        base.Toggle();
        ValueChanged?.Invoke(this, EventArgs.Empty);
    }
}

/// <summary>A switch (<c>switch</c> part, <c>as switch</c> column): a checkbox drawn as a switch.</summary>
public class SwitchInput : ToggleSwitch, IInput
{
    public event EventHandler? ValueChanged;

    protected override Type StyleKeyOverride => typeof(ToggleSwitch);

    protected override void Toggle()
    {
        base.Toggle();
        ValueChanged?.Invoke(this, EventArgs.Empty);
    }
}
