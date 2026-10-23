// Clarity's chart on Avalonia, the same as on the web
// (conformance/charts.json): the rows it is given grouped by a property,
// each group counted, summed or averaged, drawn as bars or a line on an
// axis in steps of 1, 2, 2.5 or 5 times a power of ten, or as the slices of
// a pie in the palette's colors. Each bar, point and slice tells its group
// and number when pointed at. It draws again when its rows or size change.

using System.Collections;
using System.Collections.Specialized;
using System.Globalization;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Controls.Shapes;
using Avalonia.Data.Converters;
using Avalonia.Media;

namespace Clarity.Avalonia;

/// <summary>A group as the chart shows it: its label, its number, and that number as people read it.</summary>
public sealed record ChartShown(string Label, double Value, string Text);

public class ChartView : UserControl
{
    public static readonly StyledProperty<IEnumerable?> ItemsSourceProperty = AvaloniaProperty.Register<ChartView, IEnumerable?>(nameof(ItemsSource));

    /// <summary>The rows: records of the list the chart binds.</summary>
    public IEnumerable? ItemsSource { get => GetValue(ItemsSourceProperty); set => SetValue(ItemsSourceProperty, value); }

    public ChartShape Shape { get; set; }

    /// <summary>The record's property the rows are grouped by, and how its values are ordered.</summary>
    public string By { get; set; } = "";
    public ChartBy ByKind { get; set; }

    /// <summary>Each group's number, and the property it sums or averages.</summary>
    public ChartMeasure Number { get; set; }
    public string? Of { get; set; }

    /// <summary>A group's value as people read it: an enum's label, a date in their format.</summary>
    public IValueConverter? ByShown { get; set; }

    public static readonly StyledProperty<IEnumerable<Choice>?> ByChoicesProperty = AvaloniaProperty.Register<ChartView, IEnumerable<Choice>?>(nameof(ByChoices));

    /// <summary>The records a reference names, whose texts a group by a reference shows and is ordered by.</summary>
    public IEnumerable<Choice>? ByChoices { get => GetValue(ByChoicesProperty); set => SetValue(ByChoicesProperty, value); }

    /// <summary>A summed or averaged number as people read it.</summary>
    public IValueConverter? OfShown { get; set; }

    /// <summary>What is drawn.</summary>
    public Canvas Drawing { get; } = new() { ClipToBounds = true };

    /// <summary>The groups as last drawn.</summary>
    public IReadOnlyList<ChartShown> Shown => shown;

    List<ChartShown> shown = [];
    INotifyCollectionChanged? watched;

    const double Left = 52, Bottom = 22, Top = 8, Right = 8;

    public ChartView()
    {
        Classes.Add("chart");
        Content = Drawing;
        SizeChanged += (_, _) => Draw();
    }

    protected override void OnPropertyChanged(AvaloniaPropertyChangedEventArgs change)
    {
        base.OnPropertyChanged(change);
        if (change.Property == ByChoicesProperty)
            Draw();
        if (change.Property != ItemsSourceProperty)
            return;
        if (watched is not null)
            watched.CollectionChanged -= RowsChanged;
        watched = ItemsSource as INotifyCollectionChanged;
        if (watched is not null)
            watched.CollectionChanged += RowsChanged;
        Draw();
    }

    void RowsChanged(object? sender, NotifyCollectionChangedEventArgs e) => Draw();

    static object? Read(object row, string? property) => property is null ? null : row.GetType().GetProperty(property)?.GetValue(row);

    static string Text(IValueConverter? converter, object? value) =>
        converter?.Convert(value, typeof(string), null, CultureInfo.CurrentCulture) as string ?? Convert.ToString(value, CultureInfo.CurrentCulture) ?? "";

    /// <summary>The groups of the rows, as the chart shows them.</summary>
    public List<ChartShown> Groups()
    {
        var rows = ItemsSource?.Cast<object>() ?? [];
        // A reference groups, and orders, by the text of the record it names.
        string? Named(object? id) => ByChoices?.FirstOrDefault(c => Equals(c.Value, id))?.Label;
        var pairs = rows.Select(r =>
        {
            var key = Read(r, By);
            return (ByChoices is null ? key : Named(key), Read(r, Of));
        });
        var groups = Charts.Groups(pairs, ByChoices is null ? ByKind : ChartBy.Text, Number);
        return
        [
            .. groups.Select(g => new ChartShown(
                ByChoices is null ? Text(ByShown, g.Key) : g.Key.ToString() ?? "",
                g.Value,
                Number == ChartMeasure.Count ? g.Value.ToString(CultureInfo.CurrentCulture) : Text(OfShown, (decimal)g.Value))),
        ];
    }

    void Draw()
    {
        shown = Groups();
        Drawing.Children.Clear();
        var (width, height) = (Bounds.Width, Bounds.Height);
        if (Shape == ChartShape.Pie)
            DrawPie(height);
        else
            DrawAxis(width, height);
    }

    static IBrush Accent => Application.Current?.FindResource("ClarityAccent") as IBrush ?? Brushes.SteelBlue;

    static IBrush Muted => Application.Current?.FindResource("ClarityMuted") as IBrush ?? Brushes.Gray;

    void Add(Control shape, string? tip = null)
    {
        if (tip is not null)
            ToolTip.SetTip(shape, tip);
        Drawing.Children.Add(shape);
    }

    void Label(string text, double x, double y, double width, TextAlignment align)
    {
        var block = new TextBlock { Text = text, FontSize = 11, Width = width, TextAlignment = align, Foreground = Muted };
        Canvas.SetLeft(block, x);
        Canvas.SetTop(block, y);
        Drawing.Children.Add(block);
    }

    void DrawAxis(double width, double height)
    {
        var values = shown.Select(g => g.Value).ToList();
        var scale = Charts.Scale(Math.Min(0, values.DefaultIfEmpty(0).Min()), Math.Max(0, values.DefaultIfEmpty(0).Max()));
        var plotW = Math.Max(0, width - Left - Right);
        var plotH = Math.Max(0, height - Top - Bottom);
        double Y(double v) => Top + plotH * (1 - (v - scale.Bottom) / (scale.Top - scale.Bottom));
        var band = shown.Count > 0 ? plotW / shown.Count : 0;
        double X(int i) => Left + band * (i + 0.5);
        for (var t = scale.Bottom; t <= scale.Top + scale.Step / 1e6; t = Math.Round((t + scale.Step) * 1e12) / 1e12)
        {
            Add(new Line { StartPoint = new Point(Left, Y(t)), EndPoint = new Point(Left + plotW, Y(t)), Stroke = Muted, StrokeThickness = t == 0 ? 1 : 0.4 });
            Label(t.ToString(CultureInfo.CurrentCulture), 0, Y(t) - 8, Left - 6, TextAlignment.Right);
        }
        for (var i = 0; i < shown.Count; i++)
            Label(shown[i].Label, X(i) - band / 2, Top + plotH + 4, band, TextAlignment.Center);
        if (Shape == ChartShape.Bar)
        {
            for (var i = 0; i < shown.Count; i++)
            {
                var g = shown[i];
                var bar = new Rectangle { Width = band * 0.6, Height = Math.Abs(Y(g.Value) - Y(0)), Fill = Accent, Classes = { "bar" } };
                Canvas.SetLeft(bar, X(i) - band * 0.3);
                Canvas.SetTop(bar, Math.Min(Y(g.Value), Y(0)));
                Add(bar, $"{g.Label}: {g.Text}");
            }
            return;
        }
        Add(new Polyline { Points = [.. shown.Select((g, i) => new Point(X(i), Y(g.Value)))], Stroke = Accent, StrokeThickness = 2 });
        for (var i = 0; i < shown.Count; i++)
        {
            var g = shown[i];
            var point = new Ellipse { Width = 7, Height = 7, Fill = Accent, Classes = { "point" } };
            Canvas.SetLeft(point, X(i) - 3.5);
            Canvas.SetTop(point, Y(g.Value) - 3.5);
            Add(point, $"{g.Label}: {g.Text}");
        }
    }

    void DrawPie(double height)
    {
        var colors = Charts.SliceColors(shown.Select(g => g.Value));
        var total = shown.Sum(g => Math.Max(0, g.Value));
        var radius = Math.Max(0, Math.Min(Bounds.Width * 0.6, height) / 2 - 8);
        var center = new Point(radius + 8, height / 2);
        var start = -Math.PI / 2;
        Point At(double angle) => new(center.X + radius * Math.Cos(angle), center.Y + radius * Math.Sin(angle));
        for (var i = 0; i < shown.Count; i++)
        {
            if (colors[i] is not { } color || total <= 0)
                continue;
            var g = shown[i];
            var angle = g.Value / total * Math.PI * 2;
            var end = start + angle;
            Geometry geometry = angle >= Math.PI * 2 - 1e-9
                ? new EllipseGeometry(new Rect(center.X - radius, center.Y - radius, radius * 2, radius * 2))
                : new PathGeometry
                {
                    Figures =
                    [
                        new PathFigure
                        {
                            StartPoint = center,
                            IsClosed = true,
                            Segments =
                            [
                                new LineSegment { Point = At(start) },
                                new ArcSegment { Point = At(end), Size = new Size(radius, radius), IsLargeArc = angle > Math.PI, SweepDirection = SweepDirection.Clockwise },
                            ],
                        },
                    ],
                };
            Add(new global::Avalonia.Controls.Shapes.Path { Data = geometry, Fill = Palette.Brush(color), Classes = { "slice" } }, $"{g.Label}: {g.Text}");
            start = end;
        }
        // The legend beside the pie.
        var row = 0;
        for (var i = 0; i < shown.Count; i++)
        {
            if (colors[i] is not { } color)
                continue;
            var swatch = new Rectangle { Width = 10, Height = 10, Fill = Palette.Brush(color) };
            Canvas.SetLeft(swatch, radius * 2 + 28);
            Canvas.SetTop(swatch, Top + 16 * row + 2);
            Drawing.Children.Add(swatch);
            var text = new TextBlock { Text = shown[i].Label, FontSize = 11 };
            Canvas.SetLeft(text, radius * 2 + 44);
            Canvas.SetTop(text, Top + 16 * row);
            Drawing.Children.Add(text);
            row++;
        }
    }
}
