// The fonts text is drawn in: a theme's font role names its families, most
// wanted first ("system-ui, Segoe UI, sans-serif"); the first the system has
// is taken. Where the system has none, in a browser page, Clarity.UI's own
// Inter is (it carries Regular and Bold, under the SIL Open Font License).
// A character the role's font lacks (Japanese, Chinese, Korean, Arabic in
// Inter) is drawn in a fallback font, a run of text at a time: first the
// fonts the application carries (resources/fonts/*.ttf or *.otf, or given
// with AddFallback), then on the desktop the system's font for that
// character. A browser page has no system fonts, so there only a carried
// fallback helps.
//
// A line of text is shaped once per text, font role, size and weight
// (ShapedText, kept in a cache), and measured, drawn and hit by the caret
// from that one shape, so they agree.

using System.Collections.Concurrent;
using System.Globalization;
using System.Reflection;
using System.Text;
using SkiaSharp;

namespace Clarity.Ui;

/// <summary>A piece of a line of text drawn in one typeface.</summary>
public readonly record struct TextRun(string Text, SKTypeface Typeface);

public sealed class Fonts(ThemeLook look)
{
    readonly Dictionary<(string, bool), SKTypeface> typefaces = [];

    static readonly Lazy<SKTypeface> regular = new(() => Load("Inter-Regular.ttf"));
    static readonly Lazy<SKTypeface> bold = new(() => Load("Inter-Bold.ttf"));

    static readonly List<SKTypeface> carriedFallbacks = [];

    /// <summary>Counts the fallbacks added: a shape made before one was added may be drawn in another font now.</summary>
    static int fallbacksAdded;

    readonly Dictionary<(string Text, string Role, bool Bold, double Size, bool Unhinted), ShapedText> shapes = [];
    int shapesFor;

    /// <summary>How many shapes it keeps before it starts again.</summary>
    const int MostShapes = 4096;
    static readonly ConcurrentDictionary<(int, bool), SKTypeface?> fallbackOf = new();

    static SKTypeface Load(string name)
    {
        using var stream = typeof(Fonts).Assembly.GetManifestResourceStream($"Clarity.Ui.Fonts.{name}")
            ?? throw new InvalidOperationException($"Clarity.Ui carries no font {name}");
        return SKTypeface.FromStream(stream) ?? throw new InvalidOperationException($"the font {name} does not load");
    }

    /// <summary>Clarity.UI's own font.</summary>
    public static SKTypeface Carried(bool bold) => bold ? Fonts.bold.Value : regular.Value;

    /// <summary>
    /// Adds a font (TrueType or OpenType data) for the characters the theme's
    /// fonts lack; fallbacks added first are asked first. False when the data
    /// is no font.
    /// </summary>
    public static bool AddFallback(byte[] font)
    {
        using var data = SKData.CreateCopy(font);
        if (SKTypeface.FromData(data) is not { } typeface)
            return false;
        lock (carriedFallbacks)
            carriedFallbacks.Add(typeface);
        // A character that had no fallback, or the system's, may have one now.
        fallbackOf.Clear();
        Interlocked.Increment(ref fallbacksAdded);
        return true;
    }

    /// <summary>
    /// Adds the fallback fonts <paramref name="app"/> carries: its resources
    /// under <c>resources/fonts/</c> ending in <c>.ttf</c> or <c>.otf</c>, in
    /// the order of their names. A file that is no font is said and left out.
    /// How many it added.
    /// </summary>
    public static int CarryFallbacks(Assembly app)
    {
        var added = 0;
        var names = app.GetManifestResourceNames()
            .Where(n => n.StartsWith("resources/fonts/", StringComparison.OrdinalIgnoreCase)
                && (n.EndsWith(".ttf", StringComparison.OrdinalIgnoreCase) || n.EndsWith(".otf", StringComparison.OrdinalIgnoreCase)))
            .Order(StringComparer.Ordinal);
        foreach (var name in names)
        {
            using var stream = app.GetManifestResourceStream(name);
            if (stream is null)
                continue;
            using var copy = new MemoryStream();
            stream.CopyTo(copy);
            if (AddFallback(copy.ToArray()))
                added++;
            else
                Console.Error.WriteLine($"the fallback font {name} does not load");
        }
        return added;
    }

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

    public SKFont Font(string role, double size, bool bold) => ShapedText.FontOf(Typeface(role, bold), size, unhinted: false);

    /// <summary>
    /// The text in runs, each in one typeface: the role's where it has the
    /// characters, a fallback's where it lacks them. Spaces and marks that
    /// join a character stay in the run before them.
    /// </summary>
    public IReadOnlyList<TextRun> Runs(string text, string role, bool bold)
    {
        var main = Typeface(role, bold);
        if (text.Length == 0 || Has(main, text))
            return [new TextRun(text, main)];
        var runs = new List<TextRun>();
        var run = new StringBuilder();
        SKTypeface? current = null;
        foreach (var rune in text.EnumerateRunes())
        {
            var category = Rune.GetUnicodeCategory(rune);
            var joins = Rune.IsWhiteSpace(rune) || Rune.IsControl(rune)
                || category is UnicodeCategory.NonSpacingMark or UnicodeCategory.EnclosingMark or UnicodeCategory.SpacingCombiningMark or UnicodeCategory.Format;
            var typeface = joins && current is not null ? current
                : Has(main, rune.Value) ? main
                : Fallback(rune.Value, bold) ?? main;
            if (current is not null && typeface != current)
            {
                runs.Add(new TextRun(run.ToString(), current));
                run.Clear();
            }
            current = typeface;
            run.Append(rune.ToString());
        }
        if (current is not null)
            runs.Add(new TextRun(run.ToString(), current));
        return runs;
    }

    /// <summary>The font for a character the theme's fonts lack: a carried one that has it, else the system's; none when nothing has it.</summary>
    static SKTypeface? Fallback(int character, bool bold) => fallbackOf.GetOrAdd((character, bold), key =>
    {
        lock (carriedFallbacks)
            foreach (var carried in carriedFallbacks)
                if (Has(carried, key.Item1))
                    return carried;
        if (OperatingSystem.IsBrowser())
            return null;
        try
        {
            var style = key.Item2 ? SKFontStyle.Bold : SKFontStyle.Normal;
            var languages = new[] { CultureInfo.CurrentUICulture.Name }.Where(n => n.Length > 0).ToArray();
            return SKFontManager.Default.MatchCharacter(null, style, languages, key.Item1) is { } system && Has(system, key.Item1) ? system : null;
        }
        catch (Exception e) when (e is not OutOfMemoryException)
        {
            // Fail safe: a system that cannot be asked draws the character as the role's font does.
            return null;
        }
    });

    /// <summary>Whether <paramref name="typeface"/> has a glyph for <paramref name="character"/>.</summary>
    public static bool Has(SKTypeface typeface, int character)
    {
        using var font = new SKFont(typeface);
        return font.ContainsGlyph(character);
    }

    /// <summary>Whether <paramref name="typeface"/> has a glyph for every character of <paramref name="text"/>.</summary>
    public static bool Has(SKTypeface typeface, string text)
    {
        using var font = new SKFont(typeface);
        return font.ContainsGlyphs(text);
    }

    /// <summary>
    /// <paramref name="text"/>, one line, as it is drawn in a font role at
    /// <paramref name="size"/>: shaped, in visual order, with its caret
    /// places. <paramref name="unhinted"/> measures as a zoomed canvas draws.
    /// </summary>
    public ShapedText Shape(string text, string role, double size, bool bold, bool unhinted = false)
    {
        var key = (text, role, bold, size, unhinted);
        lock (shapes)
        {
            if (shapesFor != Volatile.Read(ref fallbacksAdded))
            {
                shapes.Clear();
                shapesFor = Volatile.Read(ref fallbacksAdded);
            }
            if (shapes.TryGetValue(key, out var known))
                return known;
        }
        var shaped = ShapedText.Of(text, Runs(text, role, bold), size, unhinted);
        lock (shapes)
        {
            if (shapes.Count >= MostShapes)
                shapes.Clear();
            shapes[key] = shaped;
        }
        return shaped;
    }

    /// <summary>How wide <paramref name="text"/> is in a font role, at <paramref name="size"/>, as it is drawn.</summary>
    public double Measure(string text, string role, double size, bool bold, bool unhinted = false) =>
        text.Length == 0 ? 0 : Shape(text, role, size, bold, unhinted).Width;

    /// <summary>
    /// Draws a line of <paramref name="text"/> from <paramref name="x"/> at
    /// <paramref name="baseline"/>, shaped, each run in its own typeface,
    /// slanted by <paramref name="skewX"/> for italics; how wide it was.
    /// </summary>
    public double Draw(SKCanvas canvas, string text, double x, double baseline, string role, double size, bool bold, SKPaint paint, float skewX = 0, bool unhinted = false)
    {
        if (text.Length == 0)
            return 0;
        var shaped = Shape(text, role, size, bold, unhinted);
        shaped.Draw(canvas, x, baseline, paint, skewX);
        return shaped.Width;
    }
}
