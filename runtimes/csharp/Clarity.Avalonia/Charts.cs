// Charts of U1c, the same as on the web (conformance/charts.json): a list's
// rows grouped by a field, each group counted, summed or averaged, the
// groups in the order of their values; the axis of a bar or line chart in
// steps of 1, 2, 2.5 or 5 times a power of ten; a pie's slices in the
// palette's fourth row of colors.

using System.Globalization;

namespace Clarity.Avalonia;

/// <summary>What a chart draws.</summary>
public enum ChartShape
{
    Bar,
    Line,
    Pie,
}

/// <summary>What a chart groups its rows by, for the order of the groups.</summary>
public enum ChartBy
{
    Text,
    Number,
    Date,
    Enum,
    Bool,
}

/// <summary>Each group's number: its rows counted, or a number summed or averaged.</summary>
public enum ChartMeasure
{
    Count,
    Sum,
    Average,
}

/// <summary>A group of a chart: the value its rows share, and its number.</summary>
public sealed record ChartGroup(object Key, double Value);

/// <summary>The axis of a bar or line chart: from <c>Bottom</c> to <c>Top</c> in steps of <c>Step</c>.</summary>
public readonly record struct ChartScale(double Bottom, double Top, double Step);

public static class Charts
{
    static double Round12(double n) => Math.Round(n * 1e12) / 1e12;

    static double? NumberOf(object? value) => value switch
    {
        null or "" => null,
        string s => double.TryParse(s, NumberStyles.Float, CultureInfo.InvariantCulture, out var d) ? d : null,
        IConvertible c => c.ToDouble(CultureInfo.InvariantCulture),
        _ => null,
    };

    /// <summary>The groups of a chart's rows, in the order of their values; an enum's in <paramref name="order"/>, or its own.</summary>
    public static List<ChartGroup> Groups(IEnumerable<(object? By, object? Of)> rows, ChartBy by, ChartMeasure measure, IReadOnlyList<object>? order = null)
    {
        var groups = new Dictionary<object, (int Count, double Sum, int Numbers)>();
        var keys = new List<object>();
        foreach (var (key, of) in rows)
        {
            if (key is null || key is string { Length: 0 })
                continue;
            if (!groups.TryGetValue(key, out var g))
                keys.Add(key);
            g.Count++;
            if (NumberOf(of) is { } n)
            {
                g.Sum += n;
                g.Numbers++;
            }
            groups[key] = g;
        }
        int Compare(object a, object b) => by switch
        {
            ChartBy.Number => NumberOf(a)!.Value.CompareTo(NumberOf(b)!.Value),
            ChartBy.Enum when order is not null => IndexIn(order, a).CompareTo(IndexIn(order, b)),
            ChartBy.Enum => Convert.ToInt64(a, CultureInfo.InvariantCulture).CompareTo(Convert.ToInt64(b, CultureInfo.InvariantCulture)),
            ChartBy.Bool => (a is true).CompareTo(b is true),
            ChartBy.Date => a is IComparable ca && a.GetType() == b.GetType() ? ca.CompareTo(b) : string.CompareOrdinal(a.ToString(), b.ToString()),
            _ => string.CompareOrdinal(a.ToString()!.ToLowerInvariant(), b.ToString()!.ToLowerInvariant()) is var c && c != 0 ? c : string.CompareOrdinal(a.ToString(), b.ToString()),
        };
        keys.Sort(Compare);
        return
        [
            .. keys.Select(k =>
            {
                var g = groups[k];
                var value = measure switch
                {
                    ChartMeasure.Count => g.Count,
                    ChartMeasure.Sum => g.Sum,
                    _ => g.Numbers > 0 ? g.Sum / g.Numbers : 0,
                };
                return new ChartGroup(k, Round12(value));
            }),
        ];
    }

    static int IndexIn(IReadOnlyList<object> order, object key)
    {
        for (var i = 0; i < order.Count; i++)
            if (Equals(order[i], key) || order[i].ToString() == key.ToString())
                return i;
        return order.Count;
    }

    /// <summary>The axis for the lowest and highest number shown.</summary>
    public static ChartScale Scale(double low, double high)
    {
        var lo = Math.Min(low, 0);
        var hi = Math.Max(high, 0);
        if (lo == 0 && hi == 0)
            hi = 1;
        var raw = (hi - lo) / 5;
        var power = Math.Pow(10, Math.Floor(Math.Log10(raw)));
        var rest = Round12(raw / power);
        var factor = rest <= 1 ? 1 : rest <= 2 ? 2 : rest <= 2.5 ? 2.5 : rest <= 5 ? 5 : 10;
        var step = Round12(factor * power);
        var bottom = Round12(Math.Floor(Round12(lo / step)) * step);
        var top = Round12(Math.Ceiling(Round12(hi / step)) * step);
        return new ChartScale(bottom == 0 ? 0 : bottom, top, step);
    }

    /// <summary>The colors of a pie's slices: none for a slice of 0 or less.</summary>
    public static List<string?> SliceColors(IEnumerable<double> values)
    {
        var next = 0;
        var row = Palette.Rows[3];
        return [.. values.Select(v => v > 0 ? row[next++ % row.Count] : null)];
    }
}
