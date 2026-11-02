// The checkbox and the switch (DESIGN 7.9, conformance/forms.json): a box
// with a tick, or a track with a knob, and the caption beside it. Its value
// is bound, else kept in its state's IsChecked. Only the person ticking it
// is a change; what code sets is not. A read-only one cannot be ticked.
// As the shared screen draws them (runtimes/web/src/parts/CheckInput.svelte).

using SkiaSharp;

namespace Clarity.Ui;

[PartKind("checkbox", "switch")]
public sealed class CheckPart : Part
{
    /// <summary>How big the box is, and how wide and high the switch's track.</summary>
    public const double BoxSize = 16, TrackWidth = 32, TrackHeight = 18;

    bool kept;

    public override bool Focusable => Enabled;

    public bool IsSwitch => Spec.Kind == "switch";

    public bool ReadOnly => Spec.Flag("readonly") || (State?.IsReadOnly ?? false);

    public string Caption => State?.Caption is { Length: > 0 } set ? set : Host.Text(Spec.Text("caption"));

    /// <summary>Whether it is on: its bound value, else its state's, else its own.</summary>
    public bool IsChecked => Bound is { } bound ? Host.Read(bound) is true : State?.IsChecked ?? kept;

    protected internal override void Made() => PressLooks.Watch(this, Host, State);

    public override void OnClick() => Tick();

    public override bool OnKey(KeyInput key)
    {
        if (key.Key != Key.Space)
            return false;
        Tick();
        return true;
    }

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
            var knob = new Box(mark.X + (on ? TrackWidth - TrackHeight : 0) + 2, mark.Y + 2, TrackHeight - 4, TrackHeight - 4);
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
