// The fonts text is drawn in: a theme's font role names its families, most
// wanted first ("system-ui, Segoe UI, sans-serif"); the first the system has
// is taken. Where the system has none, in a browser page, Clarity.UI's own
// Inter is (it carries Regular and Bold, under the SIL Open Font License).

using SkiaSharp;

namespace Clarity.Ui.Drawing;

public sealed class Fonts(ThemeLook look)
{
    readonly Dictionary<(string, bool), SKTypeface> typefaces = [];

    static readonly Lazy<SKTypeface> regular = new(() => Load("Inter-Regular.ttf"));
    static readonly Lazy<SKTypeface> bold = new(() => Load("Inter-Bold.ttf"));

    static SKTypeface Load(string name)
    {
        using var stream = typeof(Fonts).Assembly.GetManifestResourceStream($"Clarity.Ui.Fonts.{name}")
            ?? throw new InvalidOperationException($"Clarity.Ui carries no font {name}");
        return SKTypeface.FromStream(stream) ?? throw new InvalidOperationException($"the font {name} does not load");
    }

    /// <summary>Clarity.UI's own font.</summary>
    public static SKTypeface Carried(bool bold) => bold ? Fonts.bold.Value : regular.Value;

    /// <summary>The typeface of a font role of the theme.</summary>
    public SKTypeface Typeface(string role, bool bold)
    {
        if (typefaces.TryGetValue((role, bold), out var found))
            return found;
        var typeface = OfFamilies(look.Font(role), bold);
        typefaces[(role, bold)] = typeface;
        return typeface;
    }

    static SKTypeface OfFamilies(string families, bool bold)
    {
        if (OperatingSystem.IsBrowser())
            return Carried(bold);
        var style = bold ? SKFontStyle.Bold : SKFontStyle.Normal;
        foreach (var written in families.Split(','))
        {
            var family = written.Trim().Trim('"', '\'') switch
            {
                "system-ui" or "-apple-system" or "sans-serif" => "sans-serif",
                "ui-monospace" or "monospace" => "monospace",
                "serif" or "ui-serif" => "serif",
                var named => named,
            };
            if (family == "Inter")
                return Carried(bold);
            if (family.Length > 0 && SKFontManager.Default.MatchFamily(family, style) is { } typeface)
                return typeface;
        }
        return Carried(bold);
    }

    public SKFont Font(string role, double size, bool bold) =>
        new(Typeface(role, bold), (float)size) { Subpixel = true, Edging = SKFontEdging.SubpixelAntialias };

    /// <summary>How wide <paramref name="text"/> is in a font role, at <paramref name="size"/>.</summary>
    public double Measure(string text, string role, double size, bool bold)
    {
        using var font = Font(role, size, bold);
        return font.MeasureText(text);
    }
}
