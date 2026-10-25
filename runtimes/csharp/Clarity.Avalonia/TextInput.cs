// Clarity's text input on Avalonia: a line of text, a password or a memo
// (AcceptsReturn), the same as on the web (conformance/forms.json). Typed
// text is taken when the input is left or Enter is pressed (a memo: only
// when it is left, Enter is a new line); only text that differs from the
// value is a change. A password has an eye that shows what is typed or hides
// it again, and a button that copies it, named in Clarity's own words.

using Avalonia;
using Avalonia.Automation;
using Avalonia.Controls;
using Avalonia.Controls.Primitives;
using Shapes = Avalonia.Controls.Shapes;
using Avalonia.Data;
using Avalonia.Input.Platform;
using Avalonia.Layout;
using Avalonia.Media;
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
        else if (change.Property == PasswordCharProperty)
            InnerRightContent = PasswordChar == '\0' ? null : SecretButtons();
    }

    /// <summary>The eye and the copy button of a password, the same as on the web.</summary>
    StackPanel SecretButtons()
    {
        var eye = Icon("M2 12s3.6-7 10-7 10 7 10 7-3.6 7-10 7S2 12 2 12z M15 12a3 3 0 1 1-6 0a3 3 0 1 1 6 0");
        var shut = Icon("M2 12s3.6-7 10-7 10 7 10 7-3.6 7-10 7S2 12 2 12z M15 12a3 3 0 1 1-6 0a3 3 0 1 1 6 0 M3 3l18 18");
        var reveal = SecretButton(eye, shut);
        void Name()
        {
            var name = RevealPassword ? Locale.Translate("hideSecret", "Hide") : Locale.Translate("showSecret", "Show");
            reveal.Content = RevealPassword ? shut : eye;
            ToolTip.SetTip(reveal, name);
            AutomationProperties.SetName(reveal, name);
        }
        Name();
        reveal.Click += (_, _) =>
        {
            RevealPassword = !RevealPassword;
            Name();
        };
        var copy = SecretButton(Icon("M11 9h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2h-8a2 2 0 0 1-2-2v-8a2 2 0 0 1 2-2z M5 15H4a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1h10a1 1 0 0 1 1 1v1"));
        var copyName = Locale.Translate("copySecret", "Copy");
        ToolTip.SetTip(copy, copyName);
        AutomationProperties.SetName(copy, copyName);
        // What is in the input, typed or not yet taken.
        copy.Click += async (_, _) =>
        {
            if (TopLevel.GetTopLevel(this)?.Clipboard is { } clipboard)
                await clipboard.SetTextAsync(Text ?? "");
        };
        return new StackPanel { Orientation = Orientation.Horizontal, Children = { reveal, copy } };
    }

    /// <summary>A small flat button; its icons draw in the button's text color.</summary>
    static Button SecretButton(params Shapes.Path[] icons)
    {
        var button = new Button
        {
            Content = icons[0],
            Padding = new Thickness(4, 0),
            Background = Brushes.Transparent,
            BorderThickness = new Thickness(0),
            VerticalAlignment = VerticalAlignment.Stretch,
            Focusable = false,
        };
        foreach (var icon in icons)
            icon[!Shapes.Shape.StrokeProperty] = button[!TemplatedControl.ForegroundProperty];
        return button;
    }

    static Shapes.Path Icon(string data) => new()
    {
        Data = Geometry.Parse(data),
        Width = 16,
        Height = 16,
        Stretch = Stretch.Uniform,
        StrokeThickness = 1.6,
        StrokeLineCap = PenLineCap.Round,
        StrokeJoin = PenLineJoin.Round,
    };

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
