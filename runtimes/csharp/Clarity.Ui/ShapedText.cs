// A line of text as Clarity.UI draws it, measures it and places a caret in
// it, so the three always agree. Text that needs no shaping (Latin, Greek,
// Cyrillic, Chinese, Japanese, Korean syllables, without combining marks) is
// drawn by Skia a run of one font at a time, exactly as before. Any other
// text is shaped by HarfBuzz as Avalonia shaped it: split into runs of one
// bidirectional level, one script and one font (the role's, or the fallback
// that has the letters), each shaped in its direction, so Arabic letters
// join, marks sit on their letters, Indic and Thai clusters form and the
// font's ligatures show; the runs are then laid out in visual order
// (Bidi.VisualOrder), Hebrew and Arabic from the right.

using System.Globalization;
using System.Runtime.CompilerServices;
using HarfBuzzSharp;
using SkiaSharp;
using SkiaSharp.HarfBuzz;

namespace Clarity.Ui;

public sealed class ShapedText
{
    /// <summary>A piece drawn in one font: plain text Skia draws, or glyphs HarfBuzz placed.</summary>
    sealed class Piece(SKTypeface typeface, double x, double width)
    {
        public SKTypeface Typeface { get; } = typeface;
        public double X { get; } = x;
        public double Width { get; } = width;
        /// <summary>The text of a piece that needs no shaping.</summary>
        public string? Text { get; init; }
        /// <summary>The glyphs of a shaped piece and where each goes, from the line's start at its baseline.</summary>
        public ushort[]? Glyphs { get; init; }
        public SKPoint[]? Positions { get; init; }
        public SKTextBlob? Blob { get; set; }
    }

    readonly List<Piece> pieces;
    readonly double size;
    readonly bool unhinted;
    double[]? left, right;
    bool[]? backward;
    bool[]? stops;

    /// <summary>The text it shows.</summary>
    public string Text { get; }

    /// <summary>How wide it is.</summary>
    public double Width { get; }

    /// <summary>Whether its paragraph runs right to left (its first strong letter is Hebrew or Arabic).</summary>
    public bool RightToLeft { get; }

    /// <summary>Whether any of it runs right to left.</summary>
    public bool HasRightToLeft { get; }

    /// <summary>Whether HarfBuzz shaped it; false for text drawn as before, a run of one font at a time.</summary>
    public bool Shaped { get; }

    /// <summary>The glyphs drawn, from the left: what a test reads to see Arabic letters joined.</summary>
    public IReadOnlyList<ushort> Glyphs => [.. pieces.SelectMany(p => p.Glyphs ?? (p.Text is { } t ? p.Typeface.GetGlyphs(t) : []))];

    ShapedText(string text, List<Piece> pieces, double width, double size, bool unhinted, bool shaped, bool rightToLeft, bool hasRightToLeft)
    {
        Text = text;
        this.pieces = pieces;
        Width = width;
        this.size = size;
        this.unhinted = unhinted;
        Shaped = shaped;
        RightToLeft = rightToLeft;
        HasRightToLeft = hasRightToLeft;
    }

    internal static SKFont FontOf(SKTypeface typeface, double size, bool unhinted, float skewX = 0)
    {
        var font = new SKFont(typeface, (float)size) { Subpixel = true, Edging = SKFontEdging.SubpixelAntialias, SkewX = skewX };
        if (unhinted)
        {
            font.Hinting = SKFontHinting.None;
            font.LinearMetrics = true;
        }
        return font;
    }

    // -- Shaping ------------------------------------------------------------------

    /// <summary>Whether a character needs HarfBuzz: a mark, a format character, or a script whose letters change by their neighbours or run right to left.</summary>
    static bool NeedsShaping(int c)
    {
        if (c < 0x300)
            return false;
        if (c is >= 0x0590 and <= 0x08FF or >= 0x0900 and <= 0x0DFF or >= 0x0E00 and <= 0x0FFF or >= 0x1000 and <= 0x109F
            or >= 0x1100 and <= 0x11FF or >= 0x1700 and <= 0x18AF or >= 0x1900 and <= 0x1AFF or >= 0x1B00 and <= 0x1C7F
            or >= 0xA800 and <= 0xABFF or >= 0xD7B0 and <= 0xD7FF or >= 0xFB1D and <= 0xFDFF or >= 0xFE70 and <= 0xFEFF
            or >= 0x10800 and <= 0x10FFF or >= 0x11000 and <= 0x11FFF or >= 0x1E800 and <= 0x1EFFF)
            return true;
        return CharUnicodeInfo.GetUnicodeCategory(c) is UnicodeCategory.NonSpacingMark or UnicodeCategory.EnclosingMark
            or UnicodeCategory.SpacingCombiningMark or UnicodeCategory.Format;
    }

    /// <summary>Whether <paramref name="text"/> needs HarfBuzz to be drawn right.</summary>
    public static bool NeedsShaping(string text)
    {
        foreach (var rune in text.EnumerateRunes())
            if (NeedsShaping(rune.Value))
                return true;
        return false;
    }

    /// <summary>The line <paramref name="text"/>, already split into runs of one font (Fonts.Runs), shaped at <paramref name="size"/>.</summary>
    internal static ShapedText Of(string text, IReadOnlyList<TextRun> runs, double size, bool unhinted)
    {
        if (!NeedsShaping(text))
            return Plain(text, runs, size, unhinted);
        try
        {
            return Shape(text, runs, size, unhinted);
        }
        catch (Exception e) when (e is DllNotFoundException or EntryPointNotFoundException or TypeInitializationException or BadImageFormatException or NotSupportedException)
        {
            // Fail safe: a system without HarfBuzz's library draws the text unshaped, as before.
            return Plain(text, runs, size, unhinted);
        }
    }

    /// <summary>Text drawn as Skia draws it, a run of one font at a time.</summary>
    static ShapedText Plain(string text, IReadOnlyList<TextRun> runs, double size, bool unhinted)
    {
        var pieces = new List<Piece>(runs.Count);
        var at = 0.0;
        foreach (var run in runs)
        {
            using var font = FontOf(run.Typeface, size, unhinted);
            var width = (double)font.MeasureText(run.Text);
            pieces.Add(new Piece(run.Typeface, at, width) { Text = run.Text });
            at += width;
        }
        return new ShapedText(text, pieces, at, size, unhinted, shaped: false, rightToLeft: false, hasRightToLeft: false);
    }

    /// <summary>A part of the line shaped as one: one level, one script, one font.</summary>
    readonly record struct Item(int Start, int End, int Level, Script Script, SKTypeface Typeface);

    static ShapedText Shape(string text, IReadOnlyList<TextRun> runs, double size, bool unhinted)
    {
        var paragraph = Bidi.ParagraphLevel(text);
        var levels = Bidi.Levels(text, paragraph);
        var items = Items(text, runs, levels);
        var order = Bidi.VisualOrder([.. items.Select(i => i.Level)]);

        var left = new double[text.Length];
        var right = new double[text.Length];
        var starts = Starts(text);
        var pieces = new List<Piece>(items.Count);
        var pen = 0.0;
        foreach (var index in order)
        {
            var item = items[index];
            var shaper = Shaper.Of(item.Typeface);
            var scale = size / shaper.UnitsPerEm;
            using var buffer = new HarfBuzzSharp.Buffer();
            buffer.AddUtf16(text, item.Start, item.End - item.Start);
            buffer.Direction = item.Level % 2 == 1 ? Direction.RightToLeft : Direction.LeftToRight;
            buffer.Script = item.Script;
            buffer.ClusterLevel = ClusterLevel.MonotoneGraphemes;
            shaper.Font.Shape(buffer);
            var infos = buffer.GetGlyphInfoSpan();
            var places = buffer.GetGlyphPositionSpan();
            var glyphs = new ushort[infos.Length];
            var positions = new SKPoint[infos.Length];
            // Each cluster (the characters a glyph or glyphs stand for) from its left to its right.
            var from = new Dictionary<int, (double Left, double Right)>();
            var x = pen;
            for (var g = 0; g < infos.Length; g++)
            {
                glyphs[g] = (ushort)infos[g].Codepoint;
                positions[g] = new SKPoint((float)(x + places[g].XOffset * scale), (float)(-places[g].YOffset * scale));
                var advance = places[g].XAdvance * scale;
                var cluster = (int)infos[g].Cluster;
                from[cluster] = from.TryGetValue(cluster, out var was) ? (Math.Min(was.Left, x), Math.Max(was.Right, x + advance)) : (x, x + advance);
                x += advance;
            }
            Place(item, from, starts, left, right);
            pieces.Add(new Piece(item.Typeface, pen, x - pen) { Glyphs = glyphs, Positions = positions });
            pen = x;
        }
        var shaped = new ShapedText(text, pieces, pen, size, unhinted, shaped: true, rightToLeft: paragraph == 1, hasRightToLeft: levels.Any(l => l % 2 == 1))
        {
            left = left,
            right = right,
            backward = [.. levels.Select(l => l % 2 == 1)],
            stops = Stops(text, starts),
        };
        return shaped;
    }

    /// <summary>Where each character of <paramref name="item"/> is: a cluster's width shared by the characters that start a letter in it, in the item's direction.</summary>
    static void Place(Item item, Dictionary<int, (double Left, double Right)> from, bool[] starts, double[] left, double[] right)
    {
        var clusters = from.Keys.Order().ToList();
        var rtl = item.Level % 2 == 1;
        for (var c = 0; c < clusters.Count; c++)
        {
            var first = clusters[c];
            var end = c + 1 < clusters.Count ? clusters[c + 1] : item.End;
            var (l, r) = from[first];
            var letters = 0;
            for (var u = first; u < end; u++)
                if (starts[u])
                    letters++;
            var each = (r - l) / Math.Max(1, letters);
            var cursor = rtl ? r : l;
            for (var u = first; u < end; u++)
            {
                var wide = starts[u] || (letters == 0 && u == first) ? each : 0;
                if (rtl)
                {
                    right[u] = cursor;
                    left[u] = cursor -= wide;
                }
                else
                {
                    left[u] = cursor;
                    right[u] = cursor += wide;
                }
            }
        }
        // Characters before the first cluster (none, when HarfBuzz numbered them all) sit at the item's start.
        if (clusters.Count > 0)
            for (var u = item.Start; u < clusters[0]; u++)
                left[u] = right[u] = rtl ? from[clusters[0]].Right : from[clusters[0]].Left;
    }

    /// <summary>The line split where its level, its script or its font changes.</summary>
    static List<Item> Items(string text, IReadOnlyList<TextRun> runs, byte[] levels)
    {
        var fonts = new SKTypeface[text.Length];
        var at = 0;
        foreach (var run in runs)
            for (var i = 0; i < run.Text.Length && at < text.Length; i++)
                fonts[at++] = run.Typeface;
        for (; at < text.Length; at++)
            fonts[at] = runs.Count > 0 ? runs[^1].Typeface : SKTypeface.Default;

        // Each character's script; common ones (spaces, signs, marks) take their neighbour's.
        var scripts = new Script[text.Length];
        for (var i = 0; i < text.Length;)
        {
            System.Text.Rune.DecodeFromUtf16(text.AsSpan(i), out var rune, out var used);
            used = Math.Max(1, used);
            var script = UnicodeFunctions.Default.GetScript(rune.Value);
            for (var u = i; u < i + used; u++)
                scripts[u] = script;
            i += used;
        }
        static bool Shared(Script s) => s.Equals(Script.Common) || s.Equals(Script.Inherited) || s.Equals(Script.Unknown);
        var last = Script.Common;
        for (var i = 0; i < text.Length; i++)
            if (Shared(scripts[i]))
                scripts[i] = last;
            else
                last = scripts[i];
        last = Script.Common;
        for (var i = text.Length - 1; i >= 0; i--)
            if (Shared(scripts[i]))
                scripts[i] = last;
            else
                last = scripts[i];

        var items = new List<Item>();
        var start = 0;
        for (var i = 1; i <= text.Length; i++)
        {
            if (i < text.Length && levels[i] == levels[start] && scripts[i].Equals(scripts[start]) && fonts[i] == fonts[start])
                continue;
            // A surrogate pair stays whole.
            if (i < text.Length && char.IsLowSurrogate(text[i]))
                continue;
            items.Add(new Item(start, i, levels[start], scripts[start], fonts[start]));
            start = i;
        }
        return items;
    }

    /// <summary>Which characters start a letter as a person sees it (a grapheme): a caret goes before them.</summary>
    static bool[] Starts(string text)
    {
        var starts = new bool[text.Length];
        foreach (var i in StringInfo.ParseCombiningCharacters(text))
            starts[i] = true;
        return starts;
    }

    static bool[] Stops(string text, bool[] starts)
    {
        var stops = new bool[text.Length + 1];
        Array.Copy(starts, stops, text.Length);
        stops[text.Length] = true;
        return stops;
    }

    /// <summary>The boxes of plain text's characters, measured on first need: each character as wide as Skia measures it.</summary>
    void Measured()
    {
        if (left is not null)
            return;
        var l = new double[Text.Length];
        var r = new double[Text.Length];
        var at = 0;
        foreach (var piece in pieces)
        {
            using var font = FontOf(piece.Typeface, size, unhinted);
            var widths = font.GetGlyphWidths(piece.Text!, null);
            var x = piece.X;
            var g = 0;
            var text = piece.Text!;
            for (var i = 0; i < text.Length; i++)
            {
                // One glyph per character; the second half of a surrogate pair has none, a line break draws nothing.
                var w = char.IsLowSurrogate(text[i]) && i > 0 || g >= widths.Length ? 0 : widths[g++];
                if (text[i] == '\n')
                    w = 0;
                l[at] = x;
                r[at] = x += w;
                at++;
            }
        }
        backward = new bool[Text.Length];
        stops = Stops(Text, Starts(Text));
        right = r;
        left = l;
    }

    // -- Places -----------------------------------------------------------------

    /// <summary>The left edge of the character at <paramref name="index"/>, from the line's start.</summary>
    public double LeftOf(int index)
    {
        Measured();
        return left![index];
    }

    /// <summary>The right edge of the character at <paramref name="index"/>, from the line's start.</summary>
    public double RightOf(int index)
    {
        Measured();
        return right![index];
    }

    /// <summary>Whether the character at <paramref name="index"/> runs right to left.</summary>
    public bool Backward(int index)
    {
        Measured();
        return backward![index];
    }

    /// <summary>Whether a caret may stand at <paramref name="place"/>: not inside a letter made of several characters.</summary>
    public bool IsStop(int place)
    {
        Measured();
        return place >= 0 && place <= Text.Length && stops![place];
    }

    /// <summary>
    /// Where the caret stands at <paramref name="place"/> (0 is before the
    /// first character), from the line's start: at the leading edge of the
    /// character after it (its left in Latin, its right in Hebrew), at the
    /// end at the trailing edge of the last.
    /// </summary>
    public double CaretX(int place)
    {
        Measured();
        if (Text.Length == 0)
            return 0;
        if (place < Text.Length)
            return backward![place] ? right![place] : left![place];
        var last = Text.Length - 1;
        return backward![last] ? left![last] : right![last];
    }

    /// <summary>The place nearest <paramref name="x"/>, from the line's start.</summary>
    public int PlaceAt(double x)
    {
        Measured();
        var best = 0;
        var distance = double.MaxValue;
        for (var p = 0; p <= Text.Length; p++)
        {
            if (!stops![p])
                continue;
            var d = Math.Abs(CaretX(p) - x);
            if (d < distance - 1e-6)
            {
                best = p;
                distance = d;
            }
        }
        return best;
    }

    /// <summary>
    /// The place one step to the right (or left) of <paramref name="place"/>
    /// as the line shows: the arrow keys in text that runs both ways. The
    /// place itself at the line's edge.
    /// </summary>
    public int Step(int place, bool toRight)
    {
        Measured();
        var x = CaretX(place);
        var best = place;
        var bestX = toRight ? double.MaxValue : double.MinValue;
        for (var p = 0; p <= Text.Length; p++)
        {
            if (!stops![p] || p == place)
                continue;
            var px = CaretX(p);
            var beyond = toRight ? px > x + 0.01 : px < x - 0.01;
            if (!beyond)
                continue;
            var nearer = toRight ? px < bestX - 0.01 : px > bestX + 0.01;
            var asNear = Math.Abs(px - bestX) <= 0.01 && Math.Abs(p - place) < Math.Abs(best - place);
            if (nearer || asNear)
            {
                best = p;
                bestX = px;
            }
        }
        return best;
    }

    /// <summary>The stretches across the line the characters from <paramref name="start"/> to <paramref name="end"/> cover, from the left: one in text of one direction, more where directions mix.</summary>
    public IReadOnlyList<(double From, double To)> Spans(int start, int end)
    {
        Measured();
        start = Math.Clamp(start, 0, Text.Length);
        end = Math.Clamp(end, start, Text.Length);
        if (start == end)
            return [];
        if (!HasRightToLeft)
            return [(left![start], right![end - 1])];
        var boxes = new List<(double From, double To)>();
        for (var i = start; i < end; i++)
            if (right![i] > left![i])
                boxes.Add((left![i], right![i]));
        boxes.Sort();
        var spans = new List<(double From, double To)>();
        foreach (var box in boxes)
            if (spans.Count > 0 && box.From <= spans[^1].To + 0.5)
                spans[^1] = (spans[^1].From, Math.Max(spans[^1].To, box.To));
            else
                spans.Add(box);
        return spans;
    }

    // -- Drawing ----------------------------------------------------------------

    /// <summary>Draws the line from <paramref name="x"/> at <paramref name="baseline"/>; a slant (italic) when <paramref name="skewX"/> is not 0.</summary>
    public void Draw(SKCanvas canvas, double x, double baseline, SKPaint paint, float skewX = 0)
    {
        foreach (var piece in pieces)
        {
            using var font = FontOf(piece.Typeface, size, unhinted, skewX);
            if (piece.Text is { } text)
            {
                canvas.DrawText(text, (float)(x + piece.X), (float)baseline, SKTextAlign.Left, font, paint);
                continue;
            }
            if (piece.Glyphs is not { Length: > 0 } glyphs)
                continue;
            var blob = skewX == 0 ? piece.Blob : null;
            if (blob is null)
            {
                using var builder = new SKTextBlobBuilder();
                builder.AddPositionedRun(glyphs, font, piece.Positions!);
                blob = builder.Build();
                if (blob is null)
                    continue;
                if (skewX == 0)
                    piece.Blob = blob;
            }
            canvas.DrawText(blob, (float)x, (float)baseline, paint);
            if (skewX != 0)
                blob.Dispose();
        }
    }

    // -- HarfBuzz's fonts -------------------------------------------------------

    /// <summary>A typeface as HarfBuzz reads it, made once per typeface, at its own units per em.</summary>
    sealed class Shaper
    {
        static readonly ConditionalWeakTable<SKTypeface, Shaper> made = new();

        public required HarfBuzzSharp.Font Font { get; init; }
        public required int UnitsPerEm { get; init; }

        public static Shaper Of(SKTypeface typeface) => made.GetValue(typeface, t =>
        {
            // The blob takes the stream over, as SKShaper hands it.
            var stream = t.OpenStream(out var index) ?? throw new NotSupportedException($"the font {t.FamilyName} gives no data to shape with");
            using var blob = stream.ToHarfBuzzBlob();
            using var face = new Face(blob, index) { Index = index, UnitsPerEm = t.UnitsPerEm };
            var font = new HarfBuzzSharp.Font(face);
            font.SetFunctionsOpenType();
            var units = Math.Max(1, t.UnitsPerEm);
            font.SetScale(units, units);
            return new Shaper { Font = font, UnitsPerEm = units };
        });
    }
}
