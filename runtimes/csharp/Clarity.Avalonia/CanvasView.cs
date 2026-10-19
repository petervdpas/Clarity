// A canvas drawn by code: the shapes its draw handler drew, in the canvas's
// own units, scaled to fit the control in proportion and centered, as the
// web renderer draws them (conformance/drawing.json). A color is the
// theme's by name, through its Clarity<Name> brush, or written out.

using System.Globalization;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Media;
using Clarity.Runtime;

namespace Clarity.Avalonia;

public class CanvasView : Control
{
    public static readonly StyledProperty<IReadOnlyList<Shape>> DrawingProperty =
        AvaloniaProperty.Register<CanvasView, IReadOnlyList<Shape>>(nameof(Drawing), []);

    public static readonly StyledProperty<double> UnitsWidthProperty =
        AvaloniaProperty.Register<CanvasView, double>(nameof(UnitsWidth), 100);

    public static readonly StyledProperty<double> UnitsHeightProperty =
        AvaloniaProperty.Register<CanvasView, double>(nameof(UnitsHeight), 100);

    public IReadOnlyList<Shape> Drawing
    {
        get => GetValue(DrawingProperty);
        set => SetValue(DrawingProperty, value);
    }

    /// The width of the canvas in its own units.
    public double UnitsWidth
    {
        get => GetValue(UnitsWidthProperty);
        set => SetValue(UnitsWidthProperty, value);
    }

    public double UnitsHeight
    {
        get => GetValue(UnitsHeightProperty);
        set => SetValue(UnitsHeightProperty, value);
    }

    static CanvasView()
    {
        AffectsRender<CanvasView>(DrawingProperty, UnitsWidthProperty, UnitsHeightProperty);
    }

    public CanvasView()
    {
        // The theme's colors may change after a drawing: light or dark, or another theme.
        ActualThemeVariantChanged += (_, _) => InvalidateVisual();
        ResourcesChanged += (_, _) => InvalidateVisual();
    }

    /// The brush of a color: the theme's by name, written out (#rrggbb), or
    /// the theme's text color for a name the theme does not have.
    public IBrush BrushOf(string color)
    {
        if (color.StartsWith('#') && Color.TryParse(color, out var written))
            return new SolidColorBrush(written);
        var key = "Clarity" + string.Concat(color.Split('_', StringSplitOptions.RemoveEmptyEntries).Select(w => char.ToUpperInvariant(w[0]) + w[1..]));
        if (this.TryFindResource(key, ActualThemeVariant, out var found) && found is IBrush brush)
            return brush;
        if (this.TryFindResource("ClarityText", ActualThemeVariant, out var text) && text is IBrush textBrush)
            return textBrush;
        return Brushes.Black;
    }

    /// Pixels a unit, and where the canvas's units start: kept in
    /// proportion, centered in the control.
    public (double Scale, double Left, double Top) Placement()
    {
        var scale = Math.Min(Bounds.Width / UnitsWidth, Bounds.Height / UnitsHeight);
        if (!double.IsFinite(scale) || scale <= 0)
            return (0, 0, 0);
        return (scale, (Bounds.Width - UnitsWidth * scale) / 2, (Bounds.Height - UnitsHeight * scale) / 2);
    }

    public override void Render(DrawingContext context)
    {
        var (scale, left, top) = Placement();
        if (scale <= 0)
            return;
        Point At(double x, double y) => new(left + x * scale, top + y * scale);
        foreach (var shape in Drawing)
        {
            switch (shape)
            {
                case Shape.Clear clear:
                    context.FillRectangle(BrushOf(clear.Color), new Rect(left, top, UnitsWidth * scale, UnitsHeight * scale));
                    break;
                case Shape.Line line:
                    context.DrawLine(new Pen(BrushOf(line.Color), line.Width * scale, lineCap: PenLineCap.Round), At(line.X1, line.Y1), At(line.X2, line.Y2));
                    break;
                case Shape.Rect rect:
                    context.FillRectangle(BrushOf(rect.Color), new Rect(At(rect.X, rect.Y), new Size(rect.Width * scale, rect.Height * scale)));
                    break;
                case Shape.Circle circle:
                    context.DrawEllipse(BrushOf(circle.Color), null, At(circle.X, circle.Y), circle.R * scale, circle.R * scale);
                    break;
                case Shape.Ring ring:
                    context.DrawEllipse(null, new Pen(BrushOf(ring.Color), ring.Width * scale), At(ring.X, ring.Y), ring.R * scale, ring.R * scale);
                    break;
                case Shape.Words words:
                    var font = this.TryFindResource("ClarityFont", ActualThemeVariant, out var family) && family is FontFamily f ? f : FontFamily.Default;
                    var text = new FormattedText(Locale.Show(words.Text), CultureInfo.CurrentUICulture, FlowDirection.LeftToRight, new Typeface(font), words.Size * scale, BrushOf(words.Color));
                    context.DrawText(text, At(words.X, words.Y) - new Point(text.Width / 2, text.Height / 2));
                    break;
            }
        }
    }
}
