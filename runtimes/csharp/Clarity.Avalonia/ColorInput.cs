// Clarity's color input on Avalonia, the same as on the web
// (conformance/colors.json): a swatch that opens the palette every
// renderer shares, and the color as "#rrggbb" to type. Typed text is taken
// when the input is left or Enter is pressed; text that is no color stays
// as typed and is marked, and the value stays. Only a color that differs
// is a change.

using Avalonia;
using Avalonia.Controls;
using Avalonia.Controls.Primitives;
using Avalonia.Data;
using Avalonia.Input;
using Avalonia.Interactivity;
using Avalonia.Layout;
using Avalonia.Media;

namespace Clarity.Avalonia;

public class ColorInput : UserControl, IInput
{
    public static readonly StyledProperty<string?> ValueProperty =
        AvaloniaProperty.Register<ColorInput, string?>(nameof(Value), defaultBindingMode: BindingMode.TwoWay);

    public static readonly StyledProperty<bool> IsReadOnlyProperty = AvaloniaProperty.Register<ColorInput, bool>(nameof(IsReadOnly));

    /// <summary>The color as kept, "#rrggbb", or empty for none.</summary>
    public string? Value { get => GetValue(ValueProperty); set => SetValue(ValueProperty, value); }

    /// <summary>Shown, but not changed by the person.</summary>
    public bool IsReadOnly { get => GetValue(IsReadOnlyProperty); set => SetValue(IsReadOnlyProperty, value); }

    public event EventHandler? ValueChanged;

    /// <summary>The swatch, which opens the palette.</summary>
    public Button Swatch { get; }

    /// <summary>The color as text, to type.</summary>
    public TextBox Box { get; }

    /// <summary>The palette's colors, as buttons.</summary>
    public IReadOnlyList<Button> Choices => choices;

    readonly List<Button> choices = [];
    readonly Border shown = new() { CornerRadius = new CornerRadius(2) };
    readonly Flyout flyout = new() { Placement = PlacementMode.BottomEdgeAlignedLeft };

    public ColorInput()
    {
        Classes.Add("color");
        // The color fills the button, whatever a theme says about what buttons hold.
        Swatch = new Button
        {
            Classes = { "swatch" },
            Padding = new Thickness(3),
            Content = shown,
            Width = 32,
            VerticalAlignment = VerticalAlignment.Stretch,
            HorizontalContentAlignment = HorizontalAlignment.Stretch,
            VerticalContentAlignment = VerticalAlignment.Stretch,
        };
        shown.MinHeight = 14;
        var grid = new UniformGrid { Columns = 10 };
        foreach (var color in Palette.Rows.SelectMany(row => row))
        {
            var choice = new Button
            {
                Classes = { "choice" },
                Width = 22,
                Height = 22,
                Margin = new Thickness(1.5),
                Padding = new Thickness(0),
                Background = Palette.Brush(color),
                [ToolTip.TipProperty] = color,
                Tag = color,
            };
            choice.Click += (_, _) => Pick(color);
            choices.Add(choice);
            grid.Children.Add(choice);
        }
        flyout.Content = grid;
        Swatch.Flyout = flyout;
        Box = new TextBox { MaxLength = 7, Classes = { "color" } };
        Box.LostFocus += (_, _) => Take();
        Box.AddHandler(KeyDownEvent, (_, e) =>
        {
            if (e.Key == Key.Enter)
                Take();
        }, RoutingStrategies.Tunnel, handledEventsToo: true);
        var row = new Grid { ColumnDefinitions = new ColumnDefinitions("Auto,*"), ColumnSpacing = 4 };
        Grid.SetColumn(Box, 1);
        row.Children.Add(Swatch);
        row.Children.Add(Box);
        Content = row;
        Show();
    }

    protected override void OnPropertyChanged(AvaloniaPropertyChangedEventArgs change)
    {
        base.OnPropertyChanged(change);
        if (change.Property == ValueProperty)
            Show();
        else if (change.Property == IsReadOnlyProperty)
        {
            Box.IsReadOnly = IsReadOnly;
            Swatch.IsEnabled = !IsReadOnly;
        }
    }

    void Show()
    {
        Box.Text = Value ?? "";
        Box.Classes.Remove("error");
        shown.Background = Palette.Brush(Value) ?? Brushes.Transparent;
        foreach (var choice in choices)
            choice.Classes.Set("chosen", Equals(choice.Tag, Value));
    }

    /// <summary>A color of the palette picked: a change when it differs.</summary>
    public void Pick(string color)
    {
        flyout.Hide();
        Offer(color);
    }

    /// <summary>Takes what is typed: a change when it is a color that differs from the value.</summary>
    public void Take()
    {
        var typed = Box.Text ?? "";
        if (Palette.Read(typed) is not { } color)
        {
            Box.Classes.Add("error");
            return;
        }
        Offer(color);
    }

    void Offer(string color)
    {
        if (IsReadOnly)
            return;
        if (color == (Value ?? ""))
        {
            Show();
            return;
        }
        Value = color;
        ValueChanged?.Invoke(this, EventArgs.Empty);
    }
}
