// Showing and reading values in the locale of the person using the
// application (DESIGN section 1.5, I1a), the way the web renderer does it
// (app/src/lib/format.ts). .NET and the browser take their locale data from
// different places, so where they differ this file pins the shared answer of
// conformance/formats.json: dates with two-digit days and months, a 24-hour
// clock with two-digit hours, no weekday in a long date, no-break spaces
// beside a currency or percent sign, and a plain space before AM and PM.

using System.Collections.Concurrent;
using System.Globalization;
using System.Text;
using System.Text.RegularExpressions;

namespace Clarity.Runtime;

/// <summary>The types that are formatted, by the kind the designer sees.</summary>
public enum FormatKind
{
    Integer,
    Decimal,
    Money,
    Date,
    DateTime,
}

/// <summary>How a value of a field is shown: its kind, its scale (decimals), and the named format the field picks, if any.</summary>
public sealed record FieldFormat(FormatKind Kind, int? Scale = null, string? Format = null);

/// <summary>Where values are shown: the person's culture and the project's currency (an ISO 4217 code, or null).</summary>
public sealed partial class Formats(CultureInfo culture, string? currency)
{
    public CultureInfo Culture => culture;

    public string? Currency => currency;

    const char NoBreakSpace = ' ';

    // -- Showing ------------------------------------------------------------

    /// <summary>
    /// A stored value as people read it: a long, a decimal, a date or a date
    /// and time, or the same as invariant text (as slots carry them). A value
    /// that is no number or date (a database written by another program)
    /// shows as it is.
    /// </summary>
    public string Show(FieldFormat field, object? value)
    {
        switch (value)
        {
            case null:
                return "";
            case DateOnly d:
                return DatePart(d.ToDateTime(TimeOnly.MinValue), field.Format);
            case DateTime t when field.Kind == FormatKind.Date:
                return DatePart(t, field.Format);
            case DateTime t:
                return $"{DatePart(t, field.Format)} {TimePart(t, field.Format)}";
            case string s when field.Kind is FormatKind.Date or FormatKind.DateTime:
                var moment = Moment(s);
                return moment is { } m ? Show(field, field.Kind == FormatKind.Date ? DateOnly.FromDateTime(m) : m) : s;
            case string s:
                return Number().IsMatch(s.Trim()) ? ShowNumber(field, decimal.Parse(s.Trim(), CultureInfo.InvariantCulture)) : s;
            case long n:
                return ShowNumber(field, n);
            case int n:
                return ShowNumber(field, n);
            case decimal d:
                return ShowNumber(field, d);
            default:
                return Convert.ToString(value, CultureInfo.InvariantCulture) ?? "";
        }
    }

    /// <summary>What a number input shows: as the value is shown, but money without its currency sign, so what is typed and what is shown look alike.</summary>
    public string EditText(FieldFormat field, object? value) =>
        Show(field.Kind == FormatKind.Money ? field with { Format = "plain" } : field, value);

    string ShowNumber(FieldFormat field, decimal value)
    {
        var nf = culture.NumberFormat;
        var plain = field.Format == "plain";
        switch (field.Kind)
        {
            case FormatKind.Integer:
                return value.ToString(plain ? "0" : "N0", nf);
            case FormatKind.Decimal when field.Format == "percent":
            {
                // Without a scale (a local in code) a value keeps its own digits.
                var digits = field.Scale is { } scale ? Math.Max(scale - 2, 0) : Digits(value * 100);
                return Spaced(value.ToString($"P{digits}", nf));
            }
            case FormatKind.Decimal:
            {
                var digits = field.Scale ?? Digits(value);
                return value.ToString(plain ? $"F{digits}" : $"N{digits}", nf);
            }
            default:
            {
                var (symbol, digits) = currency is null ? (null, 2) : CurrencyOf(culture, currency);
                if (symbol is null || plain)
                    return value.ToString($"N{digits}", nf);
                var withSymbol = (NumberFormatInfo)nf.Clone();
                withSymbol.CurrencySymbol = symbol;
                return Spaced(value.ToString($"C{digits}", withSymbol));
            }
        }
    }

    /// <summary>The decimals a value has without its trailing zeros.</summary>
    static int Digits(decimal value) => (value / 1.000000000000000000000000000000000m).Scale;

    /// <summary>The space .NET puts beside a currency or percent sign, as the no-break space the shared cases have.</summary>
    static string Spaced(string text) => text.Replace(' ', NoBreakSpace);

    /// <summary>A stored date (<c>2026-10-17</c>) or date and time (<c>2026-10-17T09:30:00</c>).</summary>
    static DateTime? Moment(string value)
    {
        var m = MomentPattern().Match(value);
        if (!m.Success)
            return null;
        int Part(int i) => m.Groups[i].Success ? int.Parse(m.Groups[i].Value, CultureInfo.InvariantCulture) : 0;
        try
        {
            return new DateTime(Part(1), Part(2), Part(3), Part(4), Part(5), Part(6));
        }
        catch (ArgumentOutOfRangeException)
        {
            return null;
        }
    }

    string DatePart(DateTime date, string? format)
    {
        var pattern = format switch
        {
            "iso" => "yyyy-MM-dd",
            "long" => LongDate(culture.DateTimeFormat.LongDatePattern),
            _ => OneLetter().Replace(culture.DateTimeFormat.ShortDatePattern, "$1$1"),
        };
        return date.ToString(pattern, culture);
    }

    /// <summary>A long date without its weekday: <c>dddd d MMMM yyyy</c> is <c>d MMMM yyyy</c>.</summary>
    static string LongDate(string pattern) => Weekday().Replace(pattern, "").Trim(' ', ',');

    string TimePart(DateTime time, string? format)
    {
        if (format == "iso")
            return time.ToString("HH:mm", CultureInfo.InvariantCulture);
        var pattern = culture.DateTimeFormat.ShortTimePattern;
        // A 24-hour clock writes 09:30; a 12-hour one 9:30 AM.
        if (!pattern.Contains('h'))
            pattern = OneHour().Replace(pattern, "HH");
        return time.ToString(pattern, culture).Replace(' ', ' ');
    }

    // -- Reading ------------------------------------------------------------

    /// <summary>
    /// What was typed in a number input, read in the locale: the value to
    /// store (a long for integers, a decimal otherwise), or null when it is
    /// not a number of that kind. Group separators, spaces, the currency sign
    /// and the percent sign may be typed or left out.
    /// </summary>
    public object? Read(FieldFormat field, string text)
    {
        var nf = culture.NumberFormat;
        var clean = new StringBuilder();
        foreach (var c in text)
        {
            if (char.IsWhiteSpace(c) || c == '%')
                continue;
            clean.Append(c == '−' ? '-' : c);
        }
        var s = clean.ToString();
        if (currency is not null)
        {
            var (symbol, _) = CurrencyOf(culture, currency);
            if (symbol is not null)
                s = s.Replace(symbol, "");
            s = s.Replace(currency, "");
        }
        if (nf.NumberGroupSeparator.Trim().Length > 0)
            s = s.Replace(nf.NumberGroupSeparator, "");
        if (nf.NumberDecimalSeparator != ".")
            s = s.Replace(nf.NumberDecimalSeparator, ".");
        if (!Number().IsMatch(s))
            return null;
        if (field.Kind == FormatKind.Integer)
            return s.Contains('.') || !long.TryParse(s, NumberStyles.AllowLeadingSign, CultureInfo.InvariantCulture, out var n) ? null : n;
        if (!decimal.TryParse(s, NumberStyles.AllowLeadingSign | NumberStyles.AllowDecimalPoint, CultureInfo.InvariantCulture, out var d))
            return null;
        return field.Format == "percent" ? d / 100 : d;
    }

    // -- Text with slots ----------------------------------------------------

    /// <summary>
    /// Text as shown: each slot filled with its value in the locale, <c>{{</c>
    /// and <c>}}</c> as braces. <paramref name="label"/> gives the label of an
    /// enum value, <paramref name="word"/> the runtime's own words (Yes, No) in
    /// the language shown.
    /// </summary>
    public string Fill(Text text, Func<string, string, string> label, Func<string, string>? word = null)
    {
        var t = text.Template;
        var out_ = new StringBuilder();
        var i = 0;
        while (i < t.Length)
        {
            if (i + 1 < t.Length && (t[i] == '{' || t[i] == '}') && t[i + 1] == t[i])
            {
                out_.Append(t[i]);
                i += 2;
            }
            else if (t[i] == '{' && t.IndexOf('}', i) is var end && end > i)
            {
                text.Values.TryGetValue(t[(i + 1)..end], out var slot);
                out_.Append(SlotText(slot, label, word ?? (w => w)));
                i = end + 1;
            }
            else
            {
                out_.Append(t[i]);
                i += 1;
            }
        }
        return out_.ToString();
    }

    string SlotText(Slot? slot, Func<string, string, string> label, Func<string, string> word) => slot switch
    {
        null or { Value: null } => "",
        { Kind: SlotKind.Text } => slot.Value.ToString() ?? "",
        { Kind: SlotKind.Bool } => word(slot.Value is true ? "Yes" : "No"),
        { Kind: SlotKind.Enum } => label(slot.EnumName ?? "", slot.Value.ToString() ?? ""),
        _ => Show(new FieldFormat(KindOf(slot.Kind), slot.Scale, slot.Format), slot.Value),
    };

    static FormatKind KindOf(SlotKind kind) => kind switch
    {
        SlotKind.Integer => FormatKind.Integer,
        SlotKind.Money => FormatKind.Money,
        SlotKind.Date => FormatKind.Date,
        SlotKind.DateTime => FormatKind.DateTime,
        _ => FormatKind.Decimal,
    };

    // -- Currencies ---------------------------------------------------------

    static readonly ConcurrentDictionary<(string, string), (string?, int)> Currencies = new();

    /// <summary>
    /// The sign and decimals of a currency in a culture: the culture's own
    /// when it is its country's currency (￥ in Japan), else those of a
    /// country that pays with it (€ from the Netherlands), so EUR has 2
    /// decimals and JPY none.
    /// </summary>
    static (string? Symbol, int Digits) CurrencyOf(CultureInfo culture, string code) =>
        Currencies.GetOrAdd((culture.Name, code), _ =>
        {
            if (RegionOf(culture)?.ISOCurrencySymbol == code)
                return (culture.NumberFormat.CurrencySymbol, culture.NumberFormat.CurrencyDecimalDigits);
            var payer = CultureInfo.GetCultures(CultureTypes.SpecificCultures)
                .Where(c => RegionOf(c)?.ISOCurrencySymbol == code)
                .OrderBy(c => c.TwoLetterISOLanguageName == culture.TwoLetterISOLanguageName ? 0 : 1)
                .FirstOrDefault();
            return payer is null ? (code, 2) : (payer.NumberFormat.CurrencySymbol, payer.NumberFormat.CurrencyDecimalDigits);
        });

    static RegionInfo? RegionOf(CultureInfo culture)
    {
        try
        {
            return culture.IsNeutralCulture || culture.Name == "" ? null : new RegionInfo(culture.Name);
        }
        catch (ArgumentException)
        {
            return null;
        }
    }

    [GeneratedRegex(@"^-?\d+(\.\d+)?$")]
    private static partial Regex Number();

    [GeneratedRegex(@"^(\d{4})-(\d{2})-(\d{2})(?:[T ](\d{2}):(\d{2})(?::(\d{2}))?)?")]
    private static partial Regex MomentPattern();

    /// <summary>A day or month written with one letter (<c>d</c>, <c>M</c>), to write with two.</summary>
    [GeneratedRegex(@"(?<![dM])([dM])(?![dM])")]
    private static partial Regex OneLetter();

    [GeneratedRegex(@"dddd[,\s]*")]
    private static partial Regex Weekday();

    [GeneratedRegex(@"(?<!H)H(?!H)")]
    private static partial Regex OneHour();
}
