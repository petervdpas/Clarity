// What elements draw with: the canvas, the theme and its fonts, and the few
// shapes every part is made of.

using SkiaSharp;

namespace Clarity.Ui;

public enum TextAlign
{
    Left,
    Center,
    Right,
}

public sealed class Painter(SKCanvas canvas, ThemeLook look, Fonts fonts)
{
    public SKCanvas Canvas { get; } = canvas;
    public ThemeLook Look { get; } = look;
    public Fonts Fonts { get; } = fonts;

    /// <summary>A part's style says bold (its <c>weight</c>): every text it draws is bold, as an Avalonia class's FontWeight was.</summary>
    public bool StyleBold { get; init; }

    /// <summary>A part's style says where its text sits (its <c>align</c>): every text it draws at the left goes there instead.</summary>
    public TextAlign? StyleAlign { get; init; }

    public void Fill(Box box, SKColor color, double radius = 0)
    {
        using var paint = new SKPaint { Color = color, IsAntialias = radius > 0, Style = SKPaintStyle.Fill };
        if (radius > 0)
            Canvas.DrawRoundRect(box.Rect, (float)radius, (float)radius, paint);
        else
            Canvas.DrawRect(box.Rect, paint);
    }

    /// <summary>A line around the inside of <paramref name="box"/>.</summary>
    public void Stroke(Box box, SKColor color, double width, double radius = 0)
    {
        if (width <= 0 || color.Alpha == 0)
            return;
        using var paint = new SKPaint { Color = color, IsAntialias = true, Style = SKPaintStyle.Stroke, StrokeWidth = (float)width };
        var inside = box.Inset(width / 2).Rect;
        if (radius > 0)
            Canvas.DrawRoundRect(inside, (float)radius, (float)radius, paint);
        else
            Canvas.DrawRect(inside, paint);
    }

    /// <summary>A line of text in <paramref name="box"/>, in the middle of its height, cut off at its edges.</summary>
    public void Text(string text, Box box, SKColor color, double size, string fontRole = "text", bool bold = false, TextAlign align = TextAlign.Left)
    {
        if (text.Length == 0)
            return;
        bold |= StyleBold;
        if (align == TextAlign.Left && StyleAlign is { } styled)
            align = styled;
        using var font = Fonts.Font(fontRole, size, bold);
        using var paint = new SKPaint { Color = color, IsAntialias = true };
        var width = Fonts.Measure(text, fontRole, size, bold);
        var metrics = font.Metrics;
        var x = align switch
        {
            TextAlign.Center => box.X + (box.Width - width) / 2,
            TextAlign.Right => box.Right - width,
            _ => box.X,
        };
        var baseline = box.Y + (box.Height - (metrics.Descent - metrics.Ascent)) / 2 - metrics.Ascent;
        Canvas.Save();
        Canvas.ClipRect(box.Rect);
        // Characters the role's font lacks are drawn in a fallback font, run by run.
        Fonts.Draw(Canvas, text, x, baseline, fontRole, size, bold, paint);
        Canvas.Restore();
    }

    /// <summary>Draws only inside <paramref name="box"/> until disposed.</summary>
    public IDisposable Clip(Box box)
    {
        Canvas.Save();
        Canvas.ClipRect(box.Rect);
        return new Restore(Canvas);
    }

    sealed class Restore(SKCanvas canvas) : IDisposable
    {
        public void Dispose() => canvas.Restore();
    }
}
