// A chart (U1c), the same as on the web (conformance/charts.json): the rows
// of the list it binds grouped by a field, each group counted, summed or
// averaged, drawn as bars or a line on an axis in steps of 1, 2, 2.5 or 5
// times a power of ten, or as the slices of a pie in the palette's colors
// with a legend beside it. Bound to a list of numbers instead (DESIGN 5.26),
// it draws them as they are, the last `points` of them, on an axis up to
// `max` when it has one. Each bar, point and slice tells its group and
// number when pointed at. It reads its rows again whenever the view model
// changes. A trend (TrendPart) draws its values with the same drawing.

using System.Collections;
using System.Globalization;
using System.Text.Json.Nodes;
using Clarity.Runtime;
using SkiaSharp;

namespace Clarity.Ui;

/// <summary>A group as a chart shows it: its label, its number, and that number as people read it.</summary>
public sealed record ChartShown(string Label, double Value, string Text);

/// <summary>What a chart drew for one group: a bar, a point of its line, or a slice of its pie.</summary>
public enum ChartMarkKind
{
    Bar,
    Point,
    Slice,
}

/// <summary>
/// Where a chart drew a group: a bar's place; the place of a point of the
/// line (<see cref="Dot"/> when a dot is drawn there); the box of the pie a
/// slice is part of, with the angles it runs over, in degrees clockwise from
/// the right, and its color.
/// </summary>
public sealed record ChartMark(ChartMarkKind Kind, ChartShown Group, Box Box, bool Dot = false, double From = 0, double Sweep = 0, string? Color = null)
{
    /// <summary>A point well inside it: the middle of a bar or point, a point halfway out along the middle of a slice.</summary>
    public (double X, double Y) Center
    {
        get
        {
            var (cx, cy) = (Box.X + Box.Width / 2, Box.Y + Box.Height / 2);
            if (Kind != ChartMarkKind.Slice)
                return (cx, cy);
            var angle = (From + Sweep / 2) * Math.PI / 180;
            var r = Box.Width / 2 * 0.6;
            return (cx + r * Math.Cos(angle), cy + r * Math.Sin(angle));
        }
    }

    /// <summary>Whether a point of the screen is on it.</summary>
    public bool Holds(double x, double y)
    {
        switch (Kind)
        {
            case ChartMarkKind.Bar:
                return Box.Contains(x, y);
            case ChartMarkKind.Point:
                var (px, py) = Center;
                return Math.Abs(x - px) <= ChartDrawing.PointReach && Math.Abs(y - py) <= ChartDrawing.PointReach;
            default:
                var (cx, cy) = (Box.X + Box.Width / 2, Box.Y + Box.Height / 2);
                var (dx, dy) = (x - cx, y - cy);
                if (dx * dx + dy * dy > Box.Width * Box.Width / 4)
                    return false;
                var angle = Math.Atan2(dy, dx) * 180 / Math.PI;
                var from = ((angle - From) % 360 + 360) % 360;
                return from < Sweep;
        }
    }
}

/// <summary>How a chart is laid out and drawn, for a chart and a trend alike.</summary>
public static class ChartDrawing
{
    const double Left = 52, Bottom = 22, Top = 8, Right = 8;

    /// <summary>How near a point of the line the pointer is on it.</summary>
    public const double PointReach = 6;

    /// <summary>How many points a line draws with a dot each; more draw as a line alone (DESIGN 5.26).</summary>
    public const int MostDots = 30;

    static ChartScale ScaleOf(IReadOnlyList<ChartShown> groups, double? max)
    {
        var values = groups.Select(g => g.Value).ToList();
        return Charts.Scale(Math.Min(0, values.DefaultIfEmpty(0).Min()), max ?? Math.Max(0, values.DefaultIfEmpty(0).Max()));
    }

    static double PieRadius(Box box) => Math.Max(0, Math.Min(box.Width * 0.6, box.Height) / 2 - 8);

    /// <summary>Where each group is drawn in <paramref name="box"/>: a bar, a point, or a slice (none for a slice of 0 or less).</summary>
    public static List<ChartMark> Marks(Box box, ChartShape shape, IReadOnlyList<ChartShown> groups, double? max)
    {
        var marks = new List<ChartMark>();
        if (shape == ChartShape.Pie)
        {
            var colors = Charts.SliceColors(groups.Select(g => g.Value));
            var total = groups.Sum(g => Math.Max(0, g.Value));
            var radius = PieRadius(box);
            var circle = new Box(box.X + 8, box.Y + box.Height / 2 - radius, radius * 2, radius * 2);
            var start = -90.0;
            for (var i = 0; i < groups.Count; i++)
            {
                if (colors[i] is not { } color || total <= 0)
                    continue;
                var sweep = groups[i].Value / total * 360;
                marks.Add(new ChartMark(ChartMarkKind.Slice, groups[i], circle, From: start, Sweep: sweep, Color: color));
                start += sweep;
            }
            return marks;
        }
        var scale = ScaleOf(groups, max);
        var plotW = Math.Max(0, box.Width - Left - Right);
        var plotH = Math.Max(0, box.Height - Top - Bottom);
        double Y(double v) => box.Y + Top + plotH * (1 - (v - scale.Bottom) / (scale.Top - scale.Bottom));
        var band = groups.Count > 0 ? plotW / groups.Count : 0;
        double X(int i) => box.X + Left + band * (i + 0.5);
        var dots = groups.Count <= MostDots;
        for (var i = 0; i < groups.Count; i++)
        {
            var g = groups[i];
            if (shape == ChartShape.Bar)
            {
                var top = Math.Min(Y(g.Value), Y(0));
                marks.Add(new ChartMark(ChartMarkKind.Bar, g, new Box(X(i) - band * 0.3, top, band * 0.6, Math.Abs(Y(g.Value) - Y(0)))));
            }
            else
                marks.Add(new ChartMark(ChartMarkKind.Point, g, new Box(X(i) - 3.5, Y(g.Value) - 3.5, 7, 7), Dot: dots));
        }
        return marks;
    }

    /// <summary>What a group pointed at tells: its label and number, or its number alone without a label.</summary>
    public static string Told(ChartShown group) => group.Label.Length > 0 ? $"{group.Label}: {group.Text}" : group.Text;

    /// <summary>The group at a point of the screen, as it tells itself; none where there is none.</summary>
    public static string? TipAt(Box box, ChartShape shape, IReadOnlyList<ChartShown> groups, double? max, double x, double y) =>
        Marks(box, shape, groups, max).LastOrDefault(m => m.Holds(x, y)) is { } mark ? Told(mark.Group) : null;

    /// <summary>Draws a chart in <paramref name="box"/>, and the tip of what the pointer is on at <paramref name="pointer"/>.</summary>
    public static void Draw(Painter painter, Box box, ChartShape shape, IReadOnlyList<ChartShown> groups, double? max, (double X, double Y)? pointer)
    {
        using (painter.Clip(box))
        {
            if (shape == ChartShape.Pie)
                DrawPie(painter, box, groups);
            else
                DrawAxis(painter, box, shape, groups, max);
            if (pointer is { } at && TipAt(box, shape, groups, max, at.X, at.Y) is { } tip)
                DrawTip(painter, box, tip, at);
        }
    }

    static SKPaint Paint(SKColor color, bool stroke = false, double width = 1) =>
        new() { Color = color, IsAntialias = true, Style = stroke ? SKPaintStyle.Stroke : SKPaintStyle.Fill, StrokeWidth = (float)width };

    static void DrawAxis(Painter painter, Box box, ChartShape shape, IReadOnlyList<ChartShown> groups, double? max)
    {
        var look = painter.Look;
        var small = look.Size("small");
        var scale = ScaleOf(groups, max);
        var plotW = Math.Max(0, box.Width - Left - Right);
        var plotH = Math.Max(0, box.Height - Top - Bottom);
        double Y(double v) => box.Y + Top + plotH * (1 - (v - scale.Bottom) / (scale.Top - scale.Bottom));
        var band = groups.Count > 0 ? plotW / groups.Count : 0;
        double X(int i) => box.X + Left + band * (i + 0.5);
        for (var t = scale.Bottom; t <= scale.Top + scale.Step / 1e6; t = Math.Round((t + scale.Step) * 1e12) / 1e12)
        {
            var y = Math.Round(Y(t)) + 0.5;
            using (var line = Paint(look.Color(t == 0 ? "strong_border" : "border"), stroke: true))
                painter.Canvas.DrawLine((float)(box.X + Left), (float)y, (float)(box.X + Left + plotW), (float)y, line);
            painter.Text(t.ToString(CultureInfo.CurrentCulture), new Box(box.X, Y(t) - 8, Left - 6, 16), look.Color("muted"), small, align: TextAlign.Right);
        }
        for (var i = 0; i < groups.Count; i++)
            if (groups[i].Label.Length > 0)
                painter.Text(groups[i].Label, new Box(X(i) - band / 2, box.Y + Top + plotH + 4, band, 16), look.Color("muted"), small, align: TextAlign.Center);
        var marks = Marks(box, shape, groups, max);
        var accent = look.Color("accent");
        if (shape == ChartShape.Bar)
        {
            foreach (var mark in marks)
                painter.Fill(mark.Box, accent);
            return;
        }
        if (marks.Count > 1)
        {
            using var line = Paint(accent, stroke: true, width: 2);
            line.StrokeJoin = SKStrokeJoin.Round;
            line.StrokeCap = SKStrokeCap.Round;
            painter.Canvas.DrawPoints(SKPointMode.Polygon, [.. marks.Select(m => new SKPoint((float)m.Center.X, (float)m.Center.Y))], line);
        }
        using var dot = Paint(accent);
        foreach (var mark in marks.Where(m => m.Dot))
        {
            var (x, y) = mark.Center;
            painter.Canvas.DrawCircle((float)x, (float)y, 3.5f, dot);
        }
    }

    static void DrawPie(Painter painter, Box box, IReadOnlyList<ChartShown> groups)
    {
        var look = painter.Look;
        foreach (var mark in Marks(box, ChartShape.Pie, groups, null))
        {
            using var fill = Paint(SKColor.Parse(mark.Color!));
            if (mark.Sweep >= 360 - 1e-9)
                painter.Canvas.DrawOval(mark.Box.Rect, fill);
            else
                painter.Canvas.DrawArc(mark.Box.Rect, (float)mark.From, (float)mark.Sweep, true, fill);
        }
        // The legend beside the pie.
        var colors = Charts.SliceColors(groups.Select(g => g.Value));
        var left = box.X + PieRadius(box) * 2 + 28;
        var row = 0;
        for (var i = 0; i < groups.Count; i++)
        {
            if (colors[i] is not { } color)
                continue;
            var top = box.Y + Top + 16 * row;
            painter.Fill(new Box(left, top + 2, 10, 10), SKColor.Parse(color));
            painter.Text(groups[i].Label, new Box(left + 16, top - 1, Math.Max(0, box.Right - left - 16), 16), look.Color("text"), look.Size("small"));
            row++;
        }
    }

    static void DrawTip(Painter painter, Box box, string tip, (double X, double Y) at)
    {
        var look = painter.Look;
        var size = look.Size("small");
        var width = painter.Fonts.Measure(tip, "text", size, false) + 2 * look.Padding + 4;
        var height = size + 2 * look.Padding + 4;
        var x = Math.Clamp(at.X + 12, box.X, Math.Max(box.X, box.Right - width));
        var y = at.Y - height - 8 < box.Y ? at.Y + 16 : at.Y - height - 8;
        var place = new Box(x, Math.Min(y, Math.Max(box.Y, box.Bottom - height)), width, height);
        var radius = look.Radius("input");
        painter.Fill(place, look.Color("raised"), radius);
        painter.Stroke(place, look.Color("border"), look.Border, radius);
        painter.Text(tip, place.Inset(look.Padding + 2), look.Color("text"), size);
    }
}

[PartKind("chart")]
public sealed class ChartPart : Part
{
    (double X, double Y)? pointer;

    public override bool Grows => true;

    protected internal override void Made() => Host.Changed += Invalidate;

    /// <summary>What it draws: <c>chart bar</c>, <c>line</c> or <c>pie</c>.</summary>
    public ChartShape Shape => Spec.Text("chart") switch
    {
        "line" => ChartShape.Line,
        "pie" => ChartShape.Pie,
        _ => ChartShape.Bar,
    };

    /// <summary>The field its rows are grouped by; none for a chart of a list of numbers.</summary>
    public string? By => Spec.Text("by") is { Length: > 0 } by ? by : null;

    /// <summary>Each group's number, and the field it sums or averages: <c>count</c>, <c>sum total</c>, <c>average total</c>.</summary>
    public (ChartMeasure Measure, string? Of) Measure
    {
        get
        {
            var (kind, named) = Spec.Json("measure") switch
            {
                JsonObject o => (o["kind"]?.GetValue<string>(), o["field"]?.GetValue<string>()),
                JsonValue v when v.TryGetValue<string>(out var written) && written.Split(' ', 2) is var words => (words[0], words.Length > 1 ? words[1] : null),
                _ => (null, null),
            };
            return kind switch
            {
                "sum" when named is not null => (ChartMeasure.Sum, named),
                "average" when named is not null => (ChartMeasure.Average, named),
                _ => (ChartMeasure.Count, null),
            };
        }
    }

    /// <summary>The top of the axis, <c>max 100</c>; none lets the axis take in the largest number.</summary>
    public double? Max => ReadNumber(Spec.Text("max"));

    /// <summary>How many of the last numbers a chart of numbers shows; 0 shows them all.</summary>
    public int Points => (int)Math.Max(0, Spec.Number("points") ?? 0);

    internal static double? ReadNumber(object? value) => value switch
    {
        null => null,
        string s => double.TryParse(s.Trim(), NumberStyles.Float, CultureInfo.InvariantCulture, out var d) && double.IsFinite(d) ? d : null,
        bool => null,
        IConvertible c => TryDouble(c),
        _ => null,
    };

    static double? TryDouble(IConvertible c)
    {
        try
        {
            var d = c.ToDouble(CultureInfo.InvariantCulture);
            return double.IsFinite(d) ? d : null;
        }
        catch (Exception e) when (e is FormatException or InvalidCastException or OverflowException)
        {
            return null;
        }
    }

    /// <summary>The numbers of a list as a chart shows them: its last <paramref name="points"/> (all for 0), each that is a number.</summary>
    public static List<ChartShown> NumbersShown(IEnumerable<object?> numbers, int points)
    {
        var values = numbers.Select(ReadNumber).OfType<double>().ToList();
        if (points > 0 && values.Count > points)
            values = values[^points..];
        return [.. values.Select(v => new ChartShown("", v, v.ToString(CultureInfo.CurrentCulture)))];
    }

    static ChartBy KindOf(Type? type) => type switch
    {
        null => ChartBy.Text,
        { IsEnum: true } => ChartBy.Enum,
        _ when type == typeof(bool) => ChartBy.Bool,
        _ when type == typeof(DateOnly) || type == typeof(DateTime) || type == typeof(DateTimeOffset) => ChartBy.Date,
        _ when type == typeof(int) || type == typeof(long) || type == typeof(short) || type == typeof(double) || type == typeof(float) || type == typeof(decimal) => ChartBy.Number,
        _ => ChartBy.Text,
    };

    /// <summary>The groups it draws, as it shows them: its rows grouped, or its numbers.</summary>
    public List<ChartShown> Groups()
    {
        if (Bound is not { } bound)
            return [];
        if (By is not { } by)
            return NumbersShown(Host.Read(bound) is IEnumerable list and not string ? list.Cast<object?>() : Host.Rows(bound), Points);
        var rows = Host.Rows(bound);
        var (measure, of) = Measure;
        var type = rows.Count > 0 ? RowFields.TypeOf(rows[0], by) : null;
        var kind = KindOf(type);
        var choices = Host.Choices(RowFields.ColumnPath(bound, by));
        // A reference groups, and orders, by the text of the record it names.
        var reference = choices.Count > 0 && kind is not (ChartBy.Enum or ChartBy.Bool);
        string? Named(object? id) => choices.FirstOrDefault(c => RowsPart.Same(c.Value, id))?.Label;
        var pairs = rows.Select(r =>
        {
            var key = RowFields.Read(r, by);
            return (reference ? Named(key) : key, of is null ? null : RowFields.Read(r, of));
        });
        var byFormat = Host.Field(RowFields.ColumnPath(bound, by))?.Format;
        var ofFormat = of is null ? null : Host.Field(RowFields.ColumnPath(bound, of))?.Format;
        string Label(object key)
        {
            if (reference)
                return key.ToString() ?? "";
            if (choices.FirstOrDefault(c => RowsPart.Same(c.Value, key)) is { } choice)
                return choice.Label;
            return ShownText.Of(key, byFormat);
        }
        string Number(double value) => measure == ChartMeasure.Count || Math.Abs(value) >= 7.9e27
            ? value.ToString(CultureInfo.CurrentCulture)
            : ShownText.Of((decimal)value, ofFormat);
        return [.. Charts.Groups(pairs, reference ? ChartBy.Text : kind, measure).Select(g => new ChartShown(Label(g.Key), g.Value, Number(g.Value)))];
    }

    /// <summary>Where it draws each group in <paramref name="box"/>.</summary>
    public IReadOnlyList<ChartMark> Marks(Box box) => ChartDrawing.Marks(box, Shape, Groups(), Max);

    /// <summary>What the group at a point of the screen tells; none where there is none.</summary>
    public string? TipAt(double x, double y) => ChartDrawing.TipAt(Bounds, Shape, Groups(), Max, x, y);

    /// <summary>What the group the pointer is on tells, while the pointer is over it.</summary>
    public string? Tip => Hovered && pointer is { } at ? TipAt(at.X, at.Y) : null;

    public override void OnPointerMoved(PointerInput at)
    {
        pointer = (at.ScreenX, at.ScreenY);
        Invalidate();
    }

    public override void Draw(Painter painter)
    {
        if (!IsShown)
            return;
        ChartDrawing.Draw(painter, Bounds, Shape, Groups(), Max, Hovered ? pointer : null);
    }
}
