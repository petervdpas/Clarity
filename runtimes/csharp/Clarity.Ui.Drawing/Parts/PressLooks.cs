// What the buttons and choices share: how a value they show compares with a
// choice's, the shapes a joined button is drawn with, the dimmed look of a
// disabled part and the focus ring.

using System.ComponentModel;
using Clarity.Runtime;
using SkiaSharp;

namespace Clarity.Ui;

/// <summary>The corners of a button that are round: all of them, or only the outer ones of a joined group.</summary>
[Flags]
public enum RoundCorners
{
    None = 0,
    Left = 1,
    Right = 2,
    All = Left | Right,
}

static class PressLooks
{
    /// <summary>Whether two values are the same choice: numbers by their value (an id read as an int or a long), the rest as they are.</summary>
    public static bool Same(object? a, object? b)
    {
        if (a is null || b is null)
            return a is null && b is null;
        // An enum's value is its name as a choice offers it.
        if (a is Enum || b is Enum)
            return a.ToString() == b.ToString();
        if (a is string || b is string)
            return a is string sa && b is string sb && sa == sb;
        if (NumberRange.Number(a) is { } na && NumberRange.Number(b) is { } nb)
            return na == nb;
        return Equals(a, b);
    }

    /// <summary>Draws at the opacity of a disabled part until disposed; nothing changes for one that is enabled.</summary>
    public static IDisposable Dimmed(Painter painter, bool enabled)
    {
        if (enabled)
            return Nothing.Instance;
        using var paint = new SKPaint { Color = new SKColor(0, 0, 0, 140) };
        painter.Canvas.SaveLayer(paint);
        return new Restore(painter.Canvas);
    }

    /// <summary>The focus ring around <paramref name="box"/>, in the theme's focus color.</summary>
    public static void FocusRing(Painter painter, Box box, double radius) => painter.Stroke(box, painter.Look.Color("focus"), 2, radius);

    static SKRoundRect Rounded(Box box, double radius, RoundCorners corners)
    {
        var r = (float)radius;
        var left = corners.HasFlag(RoundCorners.Left) ? new SKPoint(r, r) : SKPoint.Empty;
        var right = corners.HasFlag(RoundCorners.Right) ? new SKPoint(r, r) : SKPoint.Empty;
        var rounded = new SKRoundRect();
        rounded.SetRectRadii(box.Rect, [left, right, right, left]);
        return rounded;
    }

    /// <summary>A button's plate: filled, with a line around the inside, round at <paramref name="corners"/>.</summary>
    public static void Plate(Painter painter, Box box, SKColor fill, SKColor line, double width, double radius, RoundCorners corners)
    {
        using (var paint = new SKPaint { Color = fill, IsAntialias = true, Style = SKPaintStyle.Fill })
        using (var shape = Rounded(box, radius, corners))
            painter.Canvas.DrawRoundRect(shape, paint);
        if (width <= 0 || line.Alpha == 0)
            return;
        using (var paint = new SKPaint { Color = line, IsAntialias = true, Style = SKPaintStyle.Stroke, StrokeWidth = (float)width })
        using (var shape = Rounded(box.Inset(width / 2), Math.Max(0, radius - width / 2), corners))
            painter.Canvas.DrawRoundRect(shape, paint);
    }

    /// <summary>A small arrow pointing down, in the middle of <paramref name="box"/>: a dropdown's and a split button's.</summary>
    public static void DownArrow(Painter painter, Box box, SKColor color)
    {
        var cx = (float)(box.X + box.Width / 2);
        var cy = (float)(box.Y + box.Height / 2);
        using var paint = new SKPaint { Color = color, IsAntialias = true, Style = SKPaintStyle.Stroke, StrokeWidth = 1.5f, StrokeCap = SKStrokeCap.Round };
        painter.Canvas.DrawLine(cx - 4, cy - 2, cx, cy + 2, paint);
        painter.Canvas.DrawLine(cx, cy + 2, cx + 4, cy - 2, paint);
    }

    /// <summary>A part draws again when the view model or its state changes.</summary>
    public static void Watch(Part part, IPartHost host, PartState? state)
    {
        host.Changed += part.Invalidate;
        if (state is INotifyPropertyChanged observed)
            observed.PropertyChanged += (_, _) => part.Invalidate();
    }

    sealed class Nothing : IDisposable
    {
        public static readonly Nothing Instance = new();

        public void Dispose() { }
    }

    sealed class Restore(SKCanvas canvas) : IDisposable
    {
        public void Dispose() => canvas.Restore();
    }
}
