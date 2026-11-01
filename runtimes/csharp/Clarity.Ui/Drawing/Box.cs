// A place on the screen, in the screen's units (pixels at scale 1), from its
// top left.

namespace Clarity.Ui.Drawing;

public readonly record struct Box(double X, double Y, double Width, double Height)
{
    public double Right => X + Width;
    public double Bottom => Y + Height;

    public bool Contains(double x, double y) => x >= X && x < Right && y >= Y && y < Bottom;

    public Box Offset(double x, double y) => this with { X = X + x, Y = Y + y };

    public Box Inset(double by) => new(X + by, Y + by, Math.Max(0, Width - 2 * by), Math.Max(0, Height - 2 * by));

    public SkiaSharp.SKRect Rect => new((float)X, (float)Y, (float)Right, (float)Bottom);

    public override string ToString() => $"[{X:0.###}, {Y:0.###}, {Width:0.###}, {Height:0.###}]";
}
