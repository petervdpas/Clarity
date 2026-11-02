// The checkbox and the switch (DESIGN 7.9, conformance/forms.json): a box
// with a tick, or a track with a knob, and the caption beside it. Its value
// is bound, else kept in its state's IsChecked. Only the person ticking it
// is a change; what code sets is not. A click, Space or Enter ticks it; a
// switch's knob can also be dragged to the other side. A read-only one
// cannot be ticked and takes no focus, as a disabled one.
// As the shared screen draws them (runtimes/web/src/parts/CheckInput.svelte).

using SkiaSharp;

namespace Clarity.Ui;

[PartKind("checkbox", "switch")]
public sealed class CheckPart : Part
{
    /// <summary>How big the box is, and how wide and high the switch's track.</summary>
    public const double BoxSize = 16, TrackWidth = 32, TrackHeight = 18;

    bool kept;

    /// <summary>Where a press on a switch's track started, while its knob may be dragged; how far it went; whether it moved far enough to be a drag.</summary>
    double? dragFrom;
    double dragBy;
    bool dragged;

    public override bool Focusable => Enabled && !ReadOnly;

    public bool IsSwitch => Spec.Kind == "switch";


    public string Caption => State?.Caption is { Length: > 0 } set ? set : Host.Text(Spec.Text("caption"));

    /// <summary>Whether it is on: its bound value, else its state's, else its own.</summary>
    public bool IsChecked => Bound is { } bound ? Host.Read(bound) is true : State?.IsChecked ?? kept;

    protected internal override void Made() => PressLooks.Watch(this, Host, State);

    /// <summary>The track of a switch, where its knob slides.</summary>
    Box Track => new(Bounds.X, Bounds.Y + (Bounds.Height - TrackHeight) / 2, TrackWidth, TrackHeight);

    public override void OnPointerPressed(PointerInput at)
    {
        dragged = false;
        dragBy = 0;
        dragFrom = IsSwitch && Enabled && !ReadOnly && Track.Contains(at.ScreenX, at.ScreenY) ? at.ScreenX : null;
    }

    public override void OnPointerMoved(PointerInput at)
    {
        if (dragFrom is not { } from || !Pressed)
            return;
        dragBy = at.ScreenX - from;
        if (Math.Abs(dragBy) > 3)
            dragged = true;
        Invalidate();
    }

    /// <summary>A switch's knob let go: it is on when the knob ended past the middle of the track.</summary>
    public override void OnPointerReleased(PointerInput at)
    {
        if (dragFrom is null)
            return;
        dragFrom = null;
        if (!dragged)
            return;
        var knob = KnobLeft(IsChecked) + dragBy + (TrackHeight - 4) / 2;
        var on = knob > Track.X + TrackWidth / 2;
        if (on != IsChecked)
            Tick();
        Invalidate();
    }

    public override void OnClick()
    {
        // A drag of the knob already decided.
        if (dragged)
        {
            dragged = false;
            return;
        }
        Tick();
    }

    public override bool OnKey(KeyInput key)
    {
        if (key.Key is not (Key.Space or Key.Enter) || key.Control || key.Modifiers.HasFlag(Modifiers.Alt))
            return false;
        Tick();
        return true;
    }

    /// <summary>Where the knob's left edge is, on or off, before a drag moves it.</summary>
    double KnobLeft(bool on) => Track.X + (on ? TrackWidth - TrackHeight : 0) + 2;

    /// <summary>The person ticked it: it flips and tells its change.</summary>
    public void Tick()
    {
        if (!Enabled || ReadOnly)
            return;
        var next = !IsChecked;
        if (Bound is { } bound)
            Host.Write(Spec.Name ?? "", bound, next);
        else if (State is { } state)
        {
            state.IsChecked = next;
            Host.Edited(Spec.Name!);
        }
        else
            kept = next;
        Fire("change");
        Invalidate();
    }

    public override void Draw(Painter painter)
    {
        var look = painter.Look;
        using var dimmed = PressLooks.Dimmed(painter, Enabled && !ReadOnly);
        var on = IsChecked;
        var mark = IsSwitch
            ? new Box(Bounds.X, Bounds.Y + (Bounds.Height - TrackHeight) / 2, TrackWidth, TrackHeight)
            : new Box(Bounds.X + 1, Bounds.Y + (Bounds.Height - BoxSize) / 2, BoxSize, BoxSize);
        if (IsSwitch)
        {
            painter.Fill(mark, on ? look.Color("accent") : look.Color("strong_border"), TrackHeight / 2);
            // While it is dragged the knob follows the pointer, within the track.
            var left = KnobLeft(on) + (dragFrom is not null && dragged ? dragBy : 0);
            left = Math.Clamp(left, mark.X + 2, mark.Right - TrackHeight + 2);
            var knob = new Box(left, mark.Y + 2, TrackHeight - 4, TrackHeight - 4);
            painter.Fill(knob, look.Color("panel"), knob.Height / 2);
        }
        else
        {
            var radius = Math.Min(4, look.Radius("input"));
            var error = State?.HasError == true;
            painter.Fill(mark, on ? look.Color("accent") : look.Color("input"), radius);
            if (!on || error)
                painter.Stroke(mark, error ? look.Color("error") : Hovered && Enabled ? look.Color("accent") : look.Color("strong_border"), look.Border, radius);
            if (on)
                Tick(painter, mark, look.Color("on_accent"));
        }
        if (Focused)
            PressLooks.FocusRing(painter, mark.Inset(-3), IsSwitch ? TrackHeight / 2 + 3 : 6);
        var text = Bounds with { X = mark.Right + 8, Width = Math.Max(0, Bounds.Right - mark.Right - 8) };
        painter.Text(Caption, text, look.Color("text"), look.Size("text"));
    }

    static void Tick(Painter painter, Box box, SKColor color)
    {
        using var paint = new SKPaint { Color = color, IsAntialias = true, Style = SKPaintStyle.Stroke, StrokeWidth = 2, StrokeCap = SKStrokeCap.Round, StrokeJoin = SKStrokeJoin.Round };
        var x = (float)box.X;
        var y = (float)box.Y;
        var s = (float)box.Width;
        painter.Canvas.DrawLine(x + s * 0.25f, y + s * 0.52f, x + s * 0.43f, y + s * 0.7f, paint);
        painter.Canvas.DrawLine(x + s * 0.43f, y + s * 0.7f, x + s * 0.76f, y + s * 0.32f, paint);
    }
}
