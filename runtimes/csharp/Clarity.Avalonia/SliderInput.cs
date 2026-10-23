// Clarity's slider on Avalonia, the same as on the web
// (conformance/ranges.json): dragged, it takes the nearest step when let
// go; its keys move a step, ten, or to an end. Only a value that differs is
// a change. Value is a long for whole numbers, a decimal for the rest.

using Avalonia;
using Avalonia.Controls;
using Avalonia.Controls.Primitives;
using Avalonia.Data;
using Avalonia.Input;
using Avalonia.Interactivity;

namespace Clarity.Avalonia;

public class SliderInput : UserControl, IInput
{
    public static readonly StyledProperty<object?> ValueProperty =
        AvaloniaProperty.Register<SliderInput, object?>(nameof(Value), defaultBindingMode: BindingMode.TwoWay);

    public static readonly StyledProperty<bool> IsReadOnlyProperty = AvaloniaProperty.Register<SliderInput, bool>(nameof(IsReadOnly));

    /// <summary>The number: a long when <see cref="Whole"/>, else a decimal.</summary>
    public object? Value { get => GetValue(ValueProperty); set => SetValue(ValueProperty, value); }

    /// <summary>Shown, but not moved by the person.</summary>
    public bool IsReadOnly { get => GetValue(IsReadOnlyProperty); set => SetValue(IsReadOnlyProperty, value); }

    /// <summary>The lowest value, in Clarity's notation: 0 without it.</summary>
    public string? Min { get => min; set { min = value; Ranged(); } }

    /// <summary>The highest value, in Clarity's notation: 100 without it.</summary>
    public string? Max { get => max; set { max = value; Ranged(); } }

    /// <summary>How far it moves at a time: 1 without it.</summary>
    public string? Step { get => step; set { step = value; Ranged(); } }

    string? min, max, step;

    /// <summary>The field holds whole numbers.</summary>
    public bool Whole { get; set; } = true;

    public NumberRange Range { get; private set; } = new(0, 100, 1);

    public event EventHandler? ValueChanged;

    /// <summary>The bar the person drags.</summary>
    public Slider Bar { get; } = new() { IsSnapToTickEnabled = true };

    /// <summary>The value is being shown, not moved by the person.</summary>
    bool showing;
    bool dragging;

    public SliderInput()
    {
        Classes.Add("slider");
        Content = Bar;
        Ranged();
        Bar.AddHandler(Thumb.DragCompletedEvent, (_, _) => Take(), RoutingStrategies.Bubble);
        Bar.AddHandler(PointerReleasedEvent, (_, _) => Take(), RoutingStrategies.Tunnel, handledEventsToo: true);
        Bar.AddHandler(KeyDownEvent, (_, e) =>
        {
            if (NumberRange.KeyOf(e.Key) is { } key)
            {
                e.Handled = true;
                Press(key);
            }
        }, RoutingStrategies.Tunnel);
        Bar.PropertyChanged += (_, e) =>
        {
            // A press on the bar away from the thumb, outside a drag.
            if (e.Property == RangeBase.ValueProperty && !showing && !dragging)
                Take();
        };
        Bar.AddHandler(Thumb.DragStartedEvent, (_, _) => dragging = true, RoutingStrategies.Bubble);
    }

    void Ranged()
    {
        var range = NumberRange.Of(Min, Max, Step);
        Range = range with { Min = range.Min ?? 0, Max = range.Max ?? 100 };
        showing = true;
        Bar.Minimum = (double)Range.Min!.Value;
        Bar.Maximum = (double)Range.Max!.Value;
        Bar.SmallChange = (double)Range.Step;
        Bar.LargeChange = (double)(10 * Range.Step);
        Bar.TickFrequency = (double)Range.Step;
        Show();
        showing = false;
    }

    protected override void OnPropertyChanged(AvaloniaPropertyChangedEventArgs change)
    {
        base.OnPropertyChanged(change);
        if (change.Property == ValueProperty)
            Show();
        else if (change.Property == IsReadOnlyProperty)
            Bar.IsHitTestVisible = !IsReadOnly;
    }

    void Show()
    {
        showing = true;
        Bar.Value = (double)(NumberRange.Number(Value) ?? Range.Min ?? 0);
        showing = false;
    }

    /// <summary>A key: a step, ten, or an end from the value.</summary>
    public void Press(RangeKey key)
    {
        if (!IsReadOnly)
            Set(Range.Keyed(NumberRange.Number(Value), key));
    }

    /// <summary>Let go where the bar is now: the nearest step.</summary>
    public void Take()
    {
        dragging = false;
        if (!showing)
            DropAt((decimal)Bar.Value);
    }

    /// <summary>Let go at <paramref name="at"/>: the nearest step, a change when it differs.</summary>
    public void DropAt(decimal at)
    {
        if (IsReadOnly)
        {
            Show();
            return;
        }
        Set(Range.Dragged(at));
    }

    void Set(decimal next)
    {
        var held = NumberRange.Held(next, Whole);
        if (NumberRange.Number(Value) == next)
        {
            Show();
            return;
        }
        Value = held;
        ValueChanged?.Invoke(this, EventArgs.Empty);
    }
}
