// The display part: a number in seven-segment digits, as on a calculator.
// The text a value becomes and the cells it fills follow the cases every
// renderer shares (conformance/display.json); the segments have the same
// shapes as the web's (runtimes/web/src/screen/display.ts). Lit segments
// take the text's color; dark ones show faintly, as on an LCD. The digits
// are scaled to fit the place, kept in proportion, on the right.

using System.Globalization;
using System.Text;
using SkiaSharp;

namespace Clarity.Ui;

[PartKind("display")]
public sealed class DisplayPart : Part
{
    /// <summary>How many digits a display has without <c>digits</c>.</summary>
    public const int DefaultDigits = 12;

    /// <summary>How many digits it has.</summary>
    public int Digits => Math.Max(1, (int)(Spec.Number("digits") ?? DefaultDigits));

    /// <summary>What it shows now: what it binds, 0 without a value.</summary>
    public string Text => ShownText(Invariant(Bound is { } path ? Host.Read(path) : null), Digits);

    protected internal override void Made() => Host.Changed += Invalidate;

    static string Invariant(object? value) => value switch
    {
        null => "0",
        string text when text.Trim().Length == 0 => "0",
        string text => text,
        IFormattable number => number.ToString(null, CultureInfo.InvariantCulture),
        _ => value.ToString() ?? "0",
    };

    /// <summary>The lit segments of each character: a top, b top right, c bottom right, d bottom, e bottom left, f top left, g middle.</summary>
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

    /// <summary><paramref name="digits"/> plus one, as a decimal text: "0999" gives "1000".</summary>
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

    /// <summary>The text <paramref name="value"/> (a number as text) shows in <paramref name="digits"/> digits: rounded half away from zero to fit, without trailing zeros; E when it cannot fit.</summary>
    public static string ShownText(string value, int digits)
    {
        var text = value.Trim();
        if (!System.Text.RegularExpressions.Regex.IsMatch(text, @"^-?\d+(\.\d+)?$"))
            return "E";
        var negative = text.StartsWith('-');
        if (negative)
            text = text[1..];
        var parts = text.Split('.');
        var fraction = parts.Length > 1 ? parts[1] : "";
        var integer = parts[0].TrimStart('0');
        if (integer.Length == 0)
            integer = "0";
        if (integer.Length > digits)
            return "E";
        var places = digits - integer.Length;
        var kept = integer + (fraction.Length > places ? fraction[..places] : fraction.PadRight(places, '0'));
        if (fraction.Length > places && fraction[places] >= '5')
            kept = PlusOne(kept);
        var whole = kept[..(kept.Length - places)].TrimStart('0');
        if (whole.Length == 0)
            whole = "0";
        if (whole.Length > digits)
            return "E";
        var rest = kept[(kept.Length - places)..].TrimEnd('0');
        var shown = rest.Length > 0 ? $"{whole}.{rest}" : whole;
        return negative && shown.Any(c => c is >= '1' and <= '9') ? "-" + shown : shown;
    }

    /// <summary>What each of the <paramref name="digits"/> + 1 cells shows, from the left: a character and whether its point is lit; the extra cell is for the minus sign.</summary>
    public static (char Char, bool Point)[] Cells(string text, int digits)
    {
        var lit = new List<(char Char, bool Point)>();
        foreach (var c in text)
        {
            if (c == '.' && lit.Count > 0)
                lit[^1] = (lit[^1].Char, true);
            else
                lit.Add((c, false));
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

    /// <summary>Where the cells are drawn in <paramref name="box"/>: the left edge, the top and the scale of a unit.</summary>
    public static (double Left, double Top, double Scale) Fit(Box box, int cells)
    {
        var inner = new Box(box.X + 10, box.Y + 6, Math.Max(0, box.Width - 20), Math.Max(0, box.Height - 12));
        var width = cells * CellWidth + 1;
        var scale = Math.Min(inner.Width / width, inner.Height / CellHeight);
        return (inner.Right - width * scale, inner.Y + (inner.Height - CellHeight * scale) / 2, scale);
    }

    public override void Draw(Painter painter)
    {
        if (!IsShown)
            return;
        var look = painter.Look;
        // As Avalonia drew it: no background of its own, the segments in the text color over what is behind it.
        var cells = Cells(Text, Digits);
        var (left, top, scale) = Fit(Bounds, cells.Length);
        if (scale <= 0)
            return;
        var lit = look.Color("text");
        using var on = new SKPaint { Color = lit, IsAntialias = true, Style = SKPaintStyle.Fill };
        using var off = new SKPaint { Color = lit.WithAlpha(26), IsAntialias = true, Style = SKPaintStyle.Fill };
        for (var i = 0; i < cells.Length; i++)
        {
            var (character, point) = cells[i];
            var segments = Segments.GetValueOrDefault(character, "");
            foreach (var (name, shape) in Shapes)
            {
                var data = new StringBuilder();
                for (var k = 0; k < shape.Length; k++)
                {
                    var x = left + (shape[k].X + i * CellWidth) * scale;
                    var y = top + shape[k].Y * scale;
                    data.Append(k == 0 ? "M " : " L ").Append(x.ToString(CultureInfo.InvariantCulture)).Append(' ').Append(y.ToString(CultureInfo.InvariantCulture));
                }
                data.Append(" Z");
                using var path = SKPath.ParseSvgPathData(data.ToString());
                painter.Canvas.DrawPath(path, segments.Contains(name) ? on : off);
            }
            painter.Canvas.DrawCircle((float)(left + (11.5 + i * CellWidth) * scale), (float)(top + 18 * scale), (float)scale, point ? on : off);
        }
    }
}
