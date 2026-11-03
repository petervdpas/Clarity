// What a custom part draws with on the library's canvas (DESIGN 7.9): a box
// in the screen's units, a color as 0xAARRGGBB, the theme's look by its
// roles, and the painter's commands (fill, stroke, line, oval, an SVG path,
// text, pixels, a clip). The library draws them with its own Skia; a C# part
// never holds a Skia canvas of its own.

using Clarity.Ui.Native;

namespace Clarity.Ui;

/// <summary>A place on the screen, in its units.</summary>
public readonly record struct Box(double X, double Y, double Width, double Height)
{
    public double Right => X + Width;
    public double Bottom => Y + Height;

    public bool Contains(double x, double y) => x >= X && x < Right && y >= Y && y < Bottom;

    public Box Offset(double x, double y) => this with { X = X + x, Y = Y + y };

    public Box Inset(double by) => new(X + by, Y + by, Math.Max(0, Width - 2 * by), Math.Max(0, Height - 2 * by));

    public override string ToString() => $"[{X:0.###}, {Y:0.###}, {Width:0.###}, {Height:0.###}]";
}

/// <summary>A color as the library takes it: alpha, red, green and blue in one number (0xAARRGGBB).</summary>
public readonly record struct Rgba(uint Argb)
{
    public static Rgba Of(byte red, byte green, byte blue, byte alpha = 255) => new((uint)(alpha << 24 | red << 16 | green << 8 | blue));

    public byte Alpha => (byte)(Argb >> 24);

    /// <summary>The same color at <paramref name="alpha"/>.</summary>
    public Rgba WithAlpha(byte alpha) => new((Argb & 0x00FFFFFF) | (uint)alpha << 24);
}

/// <summary>Where text goes in its box.</summary>
public enum TextAlign
{
    Left = 0,
    Center = 1,
    Right = 2,
}

/// <summary>The keys held with a key or the pointer.</summary>
[Flags]
public enum Modifiers
{
    None = 0,
    Shift = 1,
    Control = 2,
    Alt = 4,
    Meta = 8,
}

/// <summary>The pointer on a custom part: where, in the screen's units, with the keys held and how many presses in a row.</summary>
public readonly record struct PointerInput(double X, double Y, Modifiers Modifiers = Modifiers.None, int Clicks = 1);

/// <summary>A key on a custom part, by the library's name ("Enter", "A", "F2"), with the keys held.</summary>
public readonly record struct KeyInput(string Name, Modifiers Modifiers)
{
    public bool Shift => Modifiers.HasFlag(Modifiers.Shift);
    public bool Control => Modifiers.HasFlag(Modifiers.Control);
}

/// <summary>The theme's look as a custom part reads it: a color by its role, the line and corner sizes, the text size.</summary>
public sealed class ThemeLook
{
    readonly Func<string, Rgba> color;

    internal ThemeLook(Func<string, Rgba> color) => this.color = color;

    /// <summary>A color of the theme by its role: <c>accent</c>, <c>text</c>, <c>border</c>, <c>panel</c>.</summary>
    public Rgba Color(string role) => color(role);

    /// <summary>How wide a border is.</summary>
    public double Border => 1;

    /// <summary>The roundness of a kind of part's corners (<c>input</c>, <c>button</c>).</summary>
    public double Radius(string kind) => 4;

    /// <summary>The size of a kind of text; 0 draws at the theme's own text size.</summary>
    public double Size(string role) => 0;

    /// <summary>The room inside a part's border.</summary>
    public double Padding => 6;
}

/// <summary>What a custom part draws on: the library's canvas, valid while its Draw runs.</summary>
public sealed unsafe class Painter
{
    readonly nint canvas;

    internal Painter(nint canvas)
    {
        this.canvas = canvas;
        Look = new ThemeLook(LookColor);
    }

    public ThemeLook Look { get; }

    Rgba LookColor(string role)
    {
        var r = Interop.Utf8(role);
        uint argb;
        fixed (byte* rp = r)
            Interop.Check(Lib.CanvasLookColor(canvas, rp, (nuint)r.Length, out argb), "clarity_canvas_look_color");
        return new Rgba(argb);
    }

    public void Fill(Box box, Rgba color, double radius = 0) =>
        Interop.Check(Lib.CanvasFill(canvas, box.X, box.Y, box.Width, box.Height, color.Argb, radius), "clarity_canvas_fill");

    public void Stroke(Box box, Rgba color, double width, double radius = 0) =>
        Interop.Check(Lib.CanvasStroke(canvas, box.X, box.Y, box.Width, box.Height, color.Argb, width, radius), "clarity_canvas_stroke");

    public void Line(double x1, double y1, double x2, double y2, Rgba color, double width = 1) =>
        Interop.Check(Lib.CanvasLine(canvas, x1, y1, x2, y2, color.Argb, width), "clarity_canvas_line");

    public void Oval(Box box, Rgba color, bool filled = true, double width = 1) =>
        Interop.Check(Lib.CanvasOval(canvas, box.X, box.Y, box.Width, box.Height, color.Argb, filled ? 1 : 0, width), "clarity_canvas_oval");

    /// <summary>An SVG path's data (<c>M0 0 L10 10 Z</c>), filled or stroked.</summary>
    public void Path(string svg, Rgba color, bool filled = true, double width = 1)
    {
        var s = Interop.Utf8(svg);
        fixed (byte* sp = s)
            Interop.Check(Lib.CanvasPath(canvas, sp, (nuint)s.Length, color.Argb, filled ? 1 : 0, width), "clarity_canvas_path");
    }

    /// <summary>Text in a box; a size of 0 is the theme's text size.</summary>
    public void Text(string text, Box box, Rgba color, double size = 0, bool bold = false, TextAlign align = TextAlign.Left)
    {
        var t = Interop.Utf8(text);
        fixed (byte* tp = t)
            Interop.Check(Lib.CanvasText(canvas, tp, (nuint)t.Length, box.X, box.Y, box.Width, box.Height, color.Argb, size, bold ? 1 : 0, (int)align), "clarity_canvas_text");
    }

    /// <summary>How wide text is at a size (0: the theme's text size).</summary>
    public double Measure(string text, double size = 0, bool bold = false)
    {
        var t = Interop.Utf8(text);
        fixed (byte* tp = t)
        {
            Interop.Check(Lib.CanvasMeasure(canvas, tp, (nuint)t.Length, size, bold ? 1 : 0, out var width), "clarity_canvas_measure");
            return width;
        }
    }

    /// <summary>RGBA pixels (not premultiplied, rows of <paramref name="stride"/> bytes) drawn into a box.</summary>
    public void Pixels(ReadOnlySpan<byte> rgba, int width, int height, int stride, Box box)
    {
        fixed (byte* pp = rgba)
            Interop.Check(Lib.CanvasPixels(canvas, pp, (uint)width, (uint)height, (nuint)stride, box.X, box.Y, box.Width, box.Height), "clarity_canvas_pixels");
    }

    /// <summary>Clips what follows to <paramref name="box"/> until the result is disposed.</summary>
    public IDisposable Clip(Box box)
    {
        Interop.Check(Lib.CanvasClip(canvas, box.X, box.Y, box.Width, box.Height), "clarity_canvas_clip");
        return new Restore(canvas);
    }

    sealed class Restore(nint canvas) : IDisposable
    {
        bool done;

        public void Dispose()
        {
            if (done)
                return;
            done = true;
            Lib.CanvasRestore(canvas);
        }
    }
}
