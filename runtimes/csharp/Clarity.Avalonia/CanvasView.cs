// A canvas drawn by code: the shapes its draw handler drew, in the canvas's
// own units, scaled to fit the control in proportion and centered, as the
// web renderer draws them (conformance/drawing.json). A color is the
// theme's by name, through its Clarity<Name> brush, or written out. A canvas
// with `on click` runs its Command with where it was clicked, in its units.

using System.Globalization;
using System.Windows.Input;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Input;
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

    public static readonly StyledProperty<ICommand?> CommandProperty =
        AvaloniaProperty.Register<CanvasView, ICommand?>(nameof(Command));

    /// What a click runs, given the Pointer of where it was; none: clicks do nothing.
    public ICommand? Command
    {
        get => GetValue(CommandProperty);
        set => SetValue(CommandProperty, value);
    }

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

    protected override void OnPropertyChanged(AvaloniaPropertyChangedEventArgs change)
    {
        base.OnPropertyChanged(change);
        if (change.Property == CommandProperty)
            Cursor = Command is null ? null : new Cursor(StandardCursorType.Hand);
    }

    /// Where in a drawing of `units` a click `at` a part of size `part` is,
    /// the drawing fitted as it is drawn, whole and centered; rounded down.
    /// Null beside the drawing, where the part shows none of it.
    public static Pointer? PointOf(Size units, Size part, Point at)
    {
        var scale = Math.Min(part.Width / units.Width, part.Height / units.Height);
        if (!(scale > 0) || !double.IsFinite(scale))
            return null;
        var x = (at.X - (part.Width - units.Width * scale) / 2) / scale;
        var y = (at.Y - (part.Height - units.Height * scale) / 2) / scale;
        if (x < 0 || y < 0 || x >= units.Width || y >= units.Height)
            return null;
        return new Pointer((long)Math.Floor(x), (long)Math.Floor(y));
    }

    // A click is the primary button pressed and let go on the canvas.
    bool pressed;

    protected override void OnPointerPressed(PointerPressedEventArgs e)
    {
        base.OnPointerPressed(e);
        pressed = Command is not null && e.GetCurrentPoint(this).Properties.IsLeftButtonPressed;
        if (pressed)
            e.Handled = true;
    }

    protected override void OnPointerReleased(PointerReleasedEventArgs e)
    {
        base.OnPointerReleased(e);
        if (!pressed)
            return;
        pressed = false;
        var at = e.GetPosition(this);
        if (at.X < 0 || at.Y < 0 || at.X > Bounds.Width || at.Y > Bounds.Height)
            return;
        var point = PointOf(new Size(UnitsWidth, UnitsHeight), Bounds.Size, at);
        if (point is not null && Command?.CanExecute(point) == true)
        {
            Command.Execute(point);
            e.Handled = true;
        }
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
