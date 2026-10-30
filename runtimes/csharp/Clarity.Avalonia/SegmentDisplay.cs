// The display part: a number in seven-segment digits, as on a calculator.
// The text a value becomes and the cells it fills follow the cases every
// renderer shares (conformance/display.json); the segments are drawn with
// the same shapes as the web renderer's (app/src/lib/display.ts). Lit
// segments take the foreground; dark ones show faintly, as on an LCD.

using System.Globalization;
using System.Text;
using Avalonia;
using Avalonia.Controls.Primitives;
using Avalonia.Media;
using Avalonia.Media.Immutable;

namespace Clarity.Avalonia;

public class SegmentDisplay : TemplatedControl
{
    public static readonly StyledProperty<object?> ValueProperty =
        AvaloniaProperty.Register<SegmentDisplay, object?>(nameof(Value));

    public static readonly StyledProperty<int> DigitsProperty =
        AvaloniaProperty.Register<SegmentDisplay, int>(nameof(Digits), 12);

    /// The number shown: a decimal, an integer, or a number as text.
    public object? Value
    {
        get => GetValue(ValueProperty);
        set => SetValue(ValueProperty, value);
    }

    /// How many digits it has.
    public int Digits
    {
        get => GetValue(DigitsProperty);
        set => SetValue(DigitsProperty, value);
    }

    /// What it shows now.
    public string Text => ShownText(Invariant(Value), Digits);

    static SegmentDisplay()
    {
        AffectsRender<SegmentDisplay>(ValueProperty, DigitsProperty, BackgroundProperty, ForegroundProperty);
    }

    static string Invariant(object? value) => value switch
    {
        null => "0",
        string text when text.Trim().Length == 0 => "0",
        string text => text,
        IFormattable number => number.ToString(null, CultureInfo.InvariantCulture),
        _ => value.ToString() ?? "0",
    };

    /// The lit segments of each character: a top, b top right, c bottom
    /// right, d bottom, e bottom left, f top left, g middle.
    public static readonly IReadOnlyDictionary<char, string> Segments = new Dictionary<char, string>
    {
        ['0'] = "abcdef",
        ['1'] = "bc",
        ['2'] = "abdeg",
        ['3'] = "abcdg",
        ['4'] = "bcfg",
        ['5'] = "acdfg",
        ['6'] = "acdefg",
        ['7'] = "abc",
        ['8'] = "abcdefg",
        ['9'] = "abcdfg",
        ['-'] = "g",
        ['E'] = "adefg",
        [' '] = "",
    };

    /// `digits` plus one, as a decimal string: "0999" gives "1000".
    static string PlusOne(string digits)
    {
        var chars = digits.ToCharArray();
        for (var i = chars.Length - 1; i >= 0; i--)
        {
            if (chars[i] != '9')
            {
                chars[i]++;
                return new string(chars);
            }
            chars[i] = '0';
        }
        return "1" + new string(chars);
    }

    /// The text `value` (a number as text) shows in `digits` digits: rounded
    /// half away from zero to fit, without trailing zeros; E when it cannot fit.
    public static string ShownText(string value, int digits)
    {
        var text = value.Trim();
        if (!System.Text.RegularExpressions.Regex.IsMatch(text, @"^-?\d+(\.\d+)?$")) return "E";
        var negative = text.StartsWith('-');
        if (negative) text = text[1..];
        var parts = text.Split('.');
        var fraction = parts.Length > 1 ? parts[1] : "";
        var integer = parts[0].TrimStart('0');
        if (integer.Length == 0) integer = "0";
        if (integer.Length > digits) return "E";
        var places = digits - integer.Length;
        var kept = integer + (fraction.Length > places ? fraction[..places] : fraction.PadRight(places, '0'));
        if (fraction.Length > places && fraction[places] >= '5') kept = PlusOne(kept);
        var whole = kept[..(kept.Length - places)].TrimStart('0');
        if (whole.Length == 0) whole = "0";
        if (whole.Length > digits) return "E";
        var rest = kept[(kept.Length - places)..].TrimEnd('0');
        var shown = rest.Length > 0 ? $"{whole}.{rest}" : whole;
        return negative && shown.Any(c => c is >= '1' and <= '9') ? "-" + shown : shown;
    }

    /// What each of the `digits` + 1 cells shows, from the left: a character
    /// and whether its point is lit; the extra cell is for the minus sign.
    public static (char Char, bool Point)[] Cells(string text, int digits)
    {
        var lit = new List<(char Char, bool Point)>();
        foreach (var c in text)
        {
            if (c == '.' && lit.Count > 0) lit[^1] = (lit[^1].Char, true);
            else lit.Add((c, false));
        }
        var dark = Enumerable.Repeat((' ', false), Math.Max(digits + 1 - lit.Count, 0));
        return [.. dark, .. lit];
    }

    // A cell is 13 units wide and 20 high, as on the web.
    const double CellWidth = 13, CellHeight = 20, T = 1, G = 0.6;

    static (double X, double Y)[] Across(double x1, double x2, double y) =>
        [(x1, y), (x1 + T, y - T), (x2 - T, y - T), (x2, y), (x2 - T, y + T), (x1 + T, y + T)];

    static (double X, double Y)[] Down(double x, double y1, double y2) =>
        [(x, y1), (x + T, y1 + T), (x + T, y2 - T), (x, y2), (x - T, y2 - T), (x - T, y1 + T)];

    static readonly Dictionary<char, (double X, double Y)[]> Shapes = new()
    {
        ['a'] = Across(2 + G, 10 - G, 2),
        ['b'] = Down(10, 2 + G, 10 - G),
        ['c'] = Down(10, 10 + G, 18 - G),
        ['d'] = Across(2 + G, 10 - G, 18),
        ['e'] = Down(2, 10 + G, 18 - G),
        ['f'] = Down(2, 2 + G, 10 - G),
        ['g'] = Across(2 + G, 10 - G, 10),
    };

    // Fail safe: a drawing that fails shows why in the control's place.
    public override void Render(DrawingContext context) => Failed = FailSafe.Render(this, context, () => RenderSafely(context));

    /// <summary>Why the last drawing failed, or null.</summary>
    public string? Failed { get; private set; }

    void RenderSafely(DrawingContext context)
    {
        var bounds = new Rect(Bounds.Size);
        if (Background is { } background) context.FillRectangle(background, bounds, (float)CornerRadius.TopLeft);
        var cells = Cells(Text, Digits);
        // Fitted inside the padding, kept in proportion, on the right.
        var inner = bounds.Deflate(new Thickness(10, 6));
        var width = cells.Length * CellWidth + 1;
        var scale = Math.Min(inner.Width / width, inner.Height / CellHeight);
        if (scale <= 0) return;
        var left = inner.Right - width * scale;
        var top = inner.Top + (inner.Height - CellHeight * scale) / 2;
        var lit = Foreground ?? Brushes.Black;
        var dark = new ImmutableSolidColorBrush(lit is ISolidColorBrush solid ? solid.Color : Colors.Black, 0.1);
        for (var i = 0; i < cells.Length; i++)
        {
            var (character, point) = cells[i];
            var on = Segments.GetValueOrDefault(character, "");
            foreach (var (name, shape) in Shapes)
            {
                var figure = new StreamGeometry();
                using (var g = figure.Open())
                {
                    g.BeginFigure(new Point(left + (shape[0].X + i * CellWidth) * scale, top + shape[0].Y * scale), true);
                    foreach (var (x, y) in shape.Skip(1)) g.LineTo(new Point(left + (x + i * CellWidth) * scale, top + y * scale));
                    g.EndFigure(true);
                }
                context.DrawGeometry(on.Contains(name) ? lit : dark, null, figure);
            }
            var centre = new Point(left + (11.5 + i * CellWidth) * scale, top + 18 * scale);
            context.DrawEllipse(point ? lit : dark, null, centre, scale, scale);
        }
    }
}
