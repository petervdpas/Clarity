// Clarity's number input on Avalonia, the same as on the web
// (conformance/forms.json, tables.json): the value shows in the person's
// format and typed text is read the same way, when the input is left or
// Enter is pressed. Text that is no number stays as typed and is marked,
// and the value stays; a cleared input is zero; only a number that differs
// is a change. With a step it is a spin box (conformance/ranges.json): its
// arrows and keys move it a step, or ten, and a typed number is kept
// within its range.

using Avalonia;
using Avalonia.Controls;
using Avalonia.Data;
using Avalonia.Input;
using Avalonia.Controls.Primitives;
using Avalonia.Interactivity;
using Avalonia.Layout;
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

    /// <summary>A spin box's lowest value, in Clarity's notation; none without it.</summary>
    public string? Min { get => min; set { min = value; Spins(); } }

    /// <summary>A spin box's highest value, in Clarity's notation; none without it.</summary>
    public string? Max { get => max; set { max = value; Spins(); } }

    /// <summary>How far a spin box moves at a time; a plain number input without it.</summary>
    public string? Step { get => step; set { step = value; Spins(); } }

    string? min, max, step;

    /// <summary>A spin box's range; null for a plain number input.</summary>
    public NumberRange? Range { get; private set; }

    protected override Type StyleKeyOverride => typeof(TextBox);

    public NumberInput()
    {
        Classes.Add("number");
        LostFocus += (_, _) => Take();
        AddHandler(KeyDownEvent, (_, e) =>
        {
            if (e.Key == Key.Enter)
                Take();
            // Left and right move the caret; home and end too.
            else if (Range is not null && e.Key is Key.Up or Key.Down or Key.PageUp or Key.PageDown && NumberRange.KeyOf(e.Key) is { } key)
            {
                e.Handled = true;
                Spin(key);
            }
        }, RoutingStrategies.Tunnel, handledEventsToo: true);
    }

    /// <summary>Becomes a spin box, with its arrows, once it has a step.</summary>
    void Spins()
    {
        Range = Step is null && Min is null && Max is null ? null : NumberRange.Of(Min, Max, Step);
        if (Range is null || InnerRightContent is not null)
            return;
        var up = Arrow("M0,5 L5,0 L10,5", RangeKey.Up);
        var down = Arrow("M0,0 L5,5 L10,0", RangeKey.Down);
        InnerRightContent = new StackPanel { Classes = { "spin" }, VerticalAlignment = VerticalAlignment.Center, Children = { up, down } };
    }

    RepeatButton Arrow(string path, RangeKey key)
    {
        var shape = new global::Avalonia.Controls.Shapes.Path { Data = global::Avalonia.Media.Geometry.Parse(path), StrokeThickness = 1.5 };
        var arrow = new RepeatButton { Focusable = false, Padding = new Thickness(6, 1), Content = shape };
        // Drawn in the button's own text color, in every theme.
        shape[!global::Avalonia.Controls.Shapes.Shape.StrokeProperty] = arrow[!TemplatedControl.ForegroundProperty];
        arrow.Click += (_, _) => Spin(key);
        return arrow;
    }

    /// <summary>A spin box's arrow or key: a step, or ten, from the value.</summary>
    public void Spin(RangeKey key)
    {
        if (Range is null || IsReadOnly || !IsEnabled)
            return;
        Classes.Remove("error");
        Offer(Range.Keyed(NumberRange.Number(Value), key));
    }

    void Offer(decimal next)
    {
        var held = NumberRange.Held(next, Whole);
        if (NumberRange.Number(Value) == next)
        {
            Text = Locale.Formats.EditText(Format, Value);
            return;
        }
        Value = held;
        ValueChanged?.Invoke(this, EventArgs.Empty);
    }

    bool Whole => Format.Kind is FormatKind.Integer or FormatKind.Duration;

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
        if (Range is not null && NumberRange.Number(next) is { } number)
        {
            Offer(Range.TypedIn(number));
            return;
        }
        if (Equals(next, Value))
            return;
        Value = next;
        ValueChanged?.Invoke(this, EventArgs.Empty);
    }

    /// <summary>A number field always has a value; cleared, it is zero.</summary>
    object Zero() => Format.Kind is FormatKind.Integer or FormatKind.Duration ? 0L : 0m;
}
