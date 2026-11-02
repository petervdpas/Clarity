// The slider (DESIGN 7.9, conformance/ranges.json): a number between its
// lowest and highest value (its bound field's range when the part writes
// none, else 0 and 100). Dragged,
// its thumb snaps from step to step and it takes the nearest step when let
// go; its keys move a step, ten, or to an
// end. Only a value that differs is a change. Bound, it changes the field
// (a long for whole numbers, a decimal for the rest); unbound, its state's
// text. As the shared screen draws it (runtimes/web/src/parts/SliderInput.svelte).

using System.Globalization;
using Clarity.Runtime;

namespace Clarity.Ui;

[PartKind("slider")]
public sealed class SliderPart : Part
{
    /// <summary>How big the thumb is, and how high the track.</summary>
    public const double Thumb = 16, Track = 4;

    decimal? dragging;
    string kept = "";

    public override bool Focusable => Enabled;


    /// <summary>Its range: as the part writes it, else its bound field's own, 0 to 100 without ends.</summary>
    public NumberRange Range
    {
        get
        {
            var facts = Bound is { } bound ? Host.Field(bound) : null;
            var range = NumberRange.Of(Spec.Text("min") ?? facts?.Min, Spec.Text("max") ?? facts?.Max, Spec.Text("step"));
            var min = range.Min ?? 0m;
            var max = range.Max ?? 100m;
            return range with { Min = min, Max = max < min ? min : max, Step = range.Step <= 0 ? 1m : range.Step };
        }
    }

    /// <summary>Whether it holds whole numbers: its bound field's type says, else the range as written.</summary>
    public bool Whole
    {
        get
        {
            var type = Bound is { } bound ? Host.TypeOf(bound) : null;
            type = type is null ? null : Nullable.GetUnderlyingType(type) ?? type;
            if (type == typeof(long) || type == typeof(int) || type == typeof(short))
                return true;
            if (type == typeof(decimal) || type == typeof(double) || type == typeof(float))
                return false;
            return new[] { Spec.Text("min"), Spec.Text("max"), Spec.Text("step") }.All(n => n is null || !n.Contains('.'));
        }
    }

    /// <summary>Its value as a number; none when it has none.</summary>
    public decimal? Value => NumberRange.Number(Bound is { } bound ? Host.Read(bound) : State?.Text ?? kept);

    /// <summary>The value it shows: the step it is dragged to, else its value, else its lowest.</summary>
    public decimal Shown => dragging ?? Value ?? Range.Min ?? 0m;

    protected internal override void Made() => PressLooks.Watch(this, Host, State);

    Box TrackBox => new(Bounds.X + Thumb / 2, Bounds.Y + (Bounds.Height - Track) / 2, Math.Max(0, Bounds.Width - Thumb), Track);

    /// <summary>The value at a point across it, within its range, not yet a step.</summary>
    decimal At(double screenX)
    {
        var range = Range;
        var track = TrackBox;
        var part = track.Width <= 0 ? 0 : Math.Clamp((screenX - track.X) / track.Width, 0, 1);
        return range.Min!.Value + (decimal)part * (range.Max!.Value - range.Min.Value);
    }

    bool Usable => Enabled && !ReadOnly;

    public override void OnPointerPressed(PointerInput at)
    {
        if (!Usable)
            return;
        dragging = Range.Dragged(At(at.ScreenX));
        Invalidate();
    }

    public override void OnPointerMoved(PointerInput at)
    {
        if (!Pressed || dragging is null)
            return;
        // The thumb snaps to the steps while it is dragged, as it will when let go.
        dragging = Range.Dragged(At(at.ScreenX));
        Invalidate();
    }

    public override void OnPointerReleased(PointerInput at)
    {
        if (dragging is null)
            return;
        dragging = null;
        DropAt(At(at.ScreenX));
    }

    /// <summary>Let go at <paramref name="at"/>: the nearest step, a change when it differs.</summary>
    public void DropAt(decimal at)
    {
        if (Usable)
            Set(Range.Dragged(at));
    }

    static readonly Dictionary<Key, RangeKey> Keys = new()
    {
        [Key.Up] = RangeKey.Up,
        [Key.Right] = RangeKey.Up,
        [Key.Down] = RangeKey.Down,
        [Key.Left] = RangeKey.Down,
        [Key.PageUp] = RangeKey.PageUp,
        [Key.PageDown] = RangeKey.PageDown,
        [Key.Home] = RangeKey.Home,
        [Key.End] = RangeKey.End,
    };

    public override bool OnKey(KeyInput key)
    {
        if (!Keys.TryGetValue(key.Key, out var moved))
            return false;
        Press(moved);
        return true;
    }

    /// <summary>A key: a step, ten, or an end from its value.</summary>
    public void Press(RangeKey key)
    {
        if (Usable)
            Set(Range.Keyed(Value, key));
    }

    void Set(decimal next)
    {
        Invalidate();
        if (Value == next)
            return;
        if (Bound is { } bound)
            Host.Write(Spec.Name ?? "", bound, NumberRange.Held(next, Whole));
        else
        {
            var text = NumberRange.Held(next, Whole) is decimal d ? d.ToString("0.############################", CultureInfo.InvariantCulture) : ((long)next).ToString(CultureInfo.InvariantCulture);
            if (State is { } state)
            {
                state.Text = text;
                Host.Edited(Spec.Name!);
            }
            else
                kept = text;
        }
        Fire("change");
    }

    public override void OnFocusChanged(bool focused) => Fire(focused ? "enter" : "leave");

    public override void Draw(Painter painter)
    {
        var look = painter.Look;
        using var dimmed = PressLooks.Dimmed(painter, Enabled);
        var range = Range;
        var track = TrackBox;
        var span = range.Max!.Value - range.Min!.Value;
        var part = span <= 0 ? 0 : (double)Math.Clamp((Shown - range.Min.Value) / span, 0, 1);
        var x = track.X + part * track.Width;
        painter.Fill(track, look.Color("strong_border"), Track / 2);
        painter.Fill(track with { Width = x - track.X }, look.Color("accent"), Track / 2);
        var thumb = new Box(x - Thumb / 2, Bounds.Y + (Bounds.Height - Thumb) / 2, Thumb, Thumb);
        painter.Fill(thumb, look.Color("accent"), Thumb / 2);
        painter.Fill(thumb.Inset(Pressed || Hovered ? 5 : 4), look.Color("on_accent"), Thumb / 2);
        if (Focused)
            PressLooks.FocusRing(painter, thumb.Inset(-3), Thumb / 2 + 3);
    }
}
