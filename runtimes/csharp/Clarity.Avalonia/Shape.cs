// A shape a draw handler draws on its canvas, in the canvas's own units
// (conformance/drawing.json). CanvasView draws them.

using Clarity.Runtime;

namespace Clarity.Avalonia;

public abstract record Shape
{
    /// Fills the whole canvas.
    public sealed record Clear(string Color) : Shape;

    public sealed record Line(double X1, double Y1, double X2, double Y2, string Color, double Width = 1) : Shape;

    public sealed record Rect(double X, double Y, double Width, double Height, string Color) : Shape;

    public sealed record Circle(double X, double Y, double R, string Color) : Shape;

    /// A circle's outline.
    public sealed record Ring(double X, double Y, double R, string Color, double Width = 1) : Shape;

    /// Text centered on its point, shown in the person's format.
    public sealed record Words(double X, double Y, Text Text, string Color = "text", double Size = 10) : Shape;
}
