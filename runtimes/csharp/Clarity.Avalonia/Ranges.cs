// The range of a slider and a spin box (U1c), the same as on the web
// (conformance/ranges.json): how a key, a drag or a typed number moves a
// value within its lowest and highest value. Decimal sums, so a step of
// 0.1 stays exact.

using System.Globalization;

namespace Clarity.Avalonia;

/// <summary>The keys a slider and a spin box answer to.</summary>
public enum RangeKey
{
    Up,
    Down,
    PageUp,
    PageDown,
    Home,
    End,
}

/// <summary>A range: its lowest and highest value (null for no end) and its step.</summary>
public sealed record NumberRange(decimal? Min, decimal? Max, decimal Step)
{
    /// <summary>A range as the generated window writes it: numbers in Clarity's notation, null or empty for no end.</summary>
    public static NumberRange Of(string? min, string? max, string? step) => new(Number(min), Number(max), Number(step) ?? 1m);

    static decimal? Number(string? text) =>
        decimal.TryParse(text, NumberStyles.Number, CultureInfo.InvariantCulture, out var n) ? n : null;

    decimal Within(decimal n)
    {
        if (Max is { } high && n > high)
            n = high;
        if (Min is { } low && n < low)
            n = low;
        return n;
    }

    /// <summary>The value after <paramref name="key"/>: a step up or down, ten, or an end; within the range. No value counts as the lowest, or 0.</summary>
    public decimal Keyed(decimal? value, RangeKey key)
    {
        var now = value ?? Min ?? 0m;
        var next = key switch
        {
            RangeKey.Up => now + Step,
            RangeKey.Down => now - Step,
            RangeKey.PageUp => now + 10 * Step,
            RangeKey.PageDown => now - 10 * Step,
            RangeKey.Home => Min ?? now,
            _ => Max ?? now,
        };
        return Within(next);
    }

    /// <summary>The value a slider takes where it is dragged to: the nearest step from the lowest value, at most the highest step in the range.</summary>
    public decimal Dragged(decimal at)
    {
        var low = Min ?? 0m;
        var steps = Math.Round((at - low) / Step, MidpointRounding.AwayFromZero);
        if (Max is { } high)
            steps = Math.Min(steps, Math.Floor((high - low) / Step));
        return low + Math.Max(0, steps) * Step;
    }

    /// <summary>A typed number, kept within the range.</summary>
    public decimal TypedIn(decimal value) => Within(value);

    /// <summary>A number as the window holds it: a long when whole, else a decimal.</summary>
    public static object Held(decimal n, bool whole) => whole ? (object)(long)n : n;

    /// <summary>The number a value the window holds is, if any.</summary>
    public static decimal? Number(object? value) => value switch
    {
        long l => l,
        int i => i,
        decimal d => d,
        double d => (decimal)d,
        // An unbound part keeps its value as text.
        string s => Number(s),
        _ => null,
    };

    /// <summary>The range key a key press is, when it is one.</summary>
    public static RangeKey? KeyOf(global::Avalonia.Input.Key key) => key switch
    {
        global::Avalonia.Input.Key.Up or global::Avalonia.Input.Key.Right => RangeKey.Up,
        global::Avalonia.Input.Key.Down or global::Avalonia.Input.Key.Left => RangeKey.Down,
        global::Avalonia.Input.Key.PageUp => RangeKey.PageUp,
        global::Avalonia.Input.Key.PageDown => RangeKey.PageDown,
        global::Avalonia.Input.Key.Home => RangeKey.Home,
        global::Avalonia.Input.Key.End => RangeKey.End,
        _ => null,
    };
}
