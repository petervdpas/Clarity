// Clarity's radio group on Avalonia, the same as on the web
// (conformance/forms.json): one button per choice of an enum, and only the
// person picking another one is a change.

using Avalonia;
using Avalonia.Controls;
using Avalonia.Data;
using Avalonia.Layout;

namespace Clarity.Avalonia;

public sealed class RadioInput : UserControl, IInput
{
    public static readonly StyledProperty<object?> ValueProperty =
        AvaloniaProperty.Register<RadioInput, object?>(nameof(Value), defaultBindingMode: BindingMode.TwoWay);

    public static readonly StyledProperty<System.Collections.IEnumerable?> ChoicesProperty =
        AvaloniaProperty.Register<RadioInput, System.Collections.IEnumerable?>(nameof(Choices));

    public static readonly StyledProperty<bool> VerticalProperty =
        AvaloniaProperty.Register<RadioInput, bool>(nameof(Vertical));

    /// <summary>The chosen value.</summary>
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

    /// <summary>The buttons one under the other instead of side by side.</summary>
    public bool Vertical
    {
        get => GetValue(VerticalProperty);
        set => SetValue(VerticalProperty, value);
    }

    public event EventHandler? ValueChanged;

    readonly StackPanel panel = new() { Orientation = Orientation.Horizontal, Spacing = 12 };

    /// <summary>The buttons, one per choice.</summary>
    public IEnumerable<RadioButton> Buttons => panel.Children.OfType<RadioButton>();

    /// <summary>The button of <paramref name="value"/>.</summary>
    public RadioButton? ButtonOf(object? value) => Buttons.FirstOrDefault(b => Equals((b.Tag as Choice)?.Value, value));

    bool showing;

    public RadioInput() => Content = panel;

    protected override void OnPropertyChanged(AvaloniaPropertyChangedEventArgs change)
    {
        base.OnPropertyChanged(change);
        if (change.Property == ChoicesProperty)
            Build();
        else if (change.Property == VerticalProperty)
            panel.Orientation = Vertical ? Orientation.Vertical : Orientation.Horizontal;
        if (change.Property == ValueProperty || change.Property == ChoicesProperty)
            Show();
    }

    void Build()
    {
        panel.Children.Clear();
        foreach (var choice in Choices?.OfType<Choice>() ?? [])
        {
            var button = new RadioButton { Content = choice.Label, Tag = choice };
            button.IsCheckedChanged += (_, _) =>
            {
                if (showing || button.IsChecked != true || Equals(choice.Value, Value))
                    return;
                Value = choice.Value;
                ValueChanged?.Invoke(this, EventArgs.Empty);
            };
            panel.Children.Add(button);
        }
    }

    void Show()
    {
        showing = true;
        foreach (var button in Buttons)
            button.IsChecked = Equals((button.Tag as Choice)?.Value, Value);
        showing = false;
    }
}
