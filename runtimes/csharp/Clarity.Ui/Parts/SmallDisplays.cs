// Small parts that show and take nothing: a progress bar, how full it is of
// its max (conformance/small-parts.json `progress`), and a busy indicator
// running beside what it is busy with.

using System.Globalization;
using Clarity.Runtime;
using SkiaSharp;

namespace Clarity.Ui;

/// <summary>A bar filled as far as its value is of its max, 100 without one.</summary>
[PartKind("progress")]
public sealed class ProgressPart : Part
{
    /// <summary>How full a bar is, from 0 to 1, for a value and its max (100 without one): below 0 is empty, above max is full, and no value is empty.</summary>
    public static double Full(object? value, string? max)
    {
        var n = value switch
        {
            null => double.NaN,
            string text when text.Trim().Length == 0 => double.NaN,
            string text => double.TryParse(text, NumberStyles.Float, CultureInfo.InvariantCulture, out var parsed) ? parsed : double.NaN,
            IConvertible number => number.ToDouble(CultureInfo.InvariantCulture),
            _ => double.NaN,
        };
        var top = max is null ? 100 : double.TryParse(max, NumberStyles.Float, CultureInfo.InvariantCulture, out var m) ? m : double.NaN;
        if (!double.IsFinite(n) || !(top > 0))
            return 0;
        return Math.Clamp(n / top, 0, 1);
    }

    /// <summary>How full it is now.</summary>
    public double Shown => Full(Bound is { } path ? Host.Read(path) : null, Spec.Text("max"));

    protected internal override void Made() => Host.Changed += Invalidate;

    public override void Draw(Painter painter)
    {
        if (!IsShown)
            return;
        var look = painter.Look;
        var bar = new Box(Bounds.X, Bounds.Y + (Bounds.Height - 8) / 2, Bounds.Width, 8);
        painter.Fill(bar, look.Color("raised"), 4);
        var full = Shown;
        if (full > 0)
            using (painter.Clip(bar with { Width = bar.Width * full }))
                painter.Fill(bar, look.Color("accent"), 4);
        painter.Stroke(bar, look.Color("border"), look.Border, 4);
    }
}

/// <summary>A running indicator with what it is busy with: what it binds, else its caption.</summary>
[PartKind("busy")]
public sealed class BusyPart : Part
{
    /// <summary>How long a turn of the indicator takes, in milliseconds.</summary>
    const double Turn = 900;

    /// <summary>The clock it turns by; a test sets its own.</summary>
    public Func<double> Clock { get; set; } = () => Environment.TickCount64;

    public string Text =>
        Bound is { } path ? ShownText.Of(Host.Read(path))
        : State?.Caption is { Length: > 0 } caption ? caption
        : Host.Text(Spec.Text("caption"));

    protected internal override void Made() => Host.Changed += Invalidate;

    public override void Draw(Painter painter)
    {
        if (!IsShown)
            return;
        var look = painter.Look;
        var spinner = new Box(Bounds.X, Bounds.Y + (Bounds.Height - 16) / 2, 16, 16);
        using (var ring = new SKPaint { Color = look.Color("border"), IsAntialias = true, Style = SKPaintStyle.Stroke, StrokeWidth = 2 })
            painter.Canvas.DrawOval(spinner.Inset(1).Rect, ring);
        var angle = (float)(Clock() % Turn / Turn * 360);
        using (var arc = new SKPaint { Color = look.Color("accent"), IsAntialias = true, Style = SKPaintStyle.Stroke, StrokeWidth = 2, StrokeCap = SKStrokeCap.Round })
            painter.Canvas.DrawArc(spinner.Inset(1).Rect, angle - 90, 90, false, arc);
        painter.Text(Text, new Box(spinner.Right + 8, Bounds.Y, Math.Max(0, Bounds.Width - 24), Bounds.Height), look.Color("text"), look.Size("text"));
        // It keeps running while it shows: the screen draws every frame while it does.
        Screen?.Animate(this);
    }
}
