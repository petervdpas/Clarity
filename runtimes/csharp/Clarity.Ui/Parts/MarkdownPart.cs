// The markdown part: Markdown read into blocks the same way on every stack
// (MarkdownBlocks, conformance/markdown.json), shown as headings, paragraphs,
// list items and code, wrapped to its width and scrolling when longer than
// its place. Its text is a Markdown file of the application's resources (its
// `source`, by name, as the host gives the file), the text a key gives in
// the language shown (its `caption`), or the text it binds. A link opens
// through the host, in the person's browser or mail program.

using System.Text;
using System.Text.RegularExpressions;
using Clarity.Runtime;
using SkiaSharp;

namespace Clarity.Ui;

/// <summary>
/// A piece of text as a markdown page places it: a word with the space after
/// it, or a list item's marker. Its place is in the page, from its top left.
/// </summary>
public sealed record MarkdownRun(string Text, Box Box, double Size, string Font, bool Bold = false, bool Italic = false, bool Code = false, string? Link = null);

/// <summary>A page of Markdown laid out at one width: its runs, the places of its code blocks, and how high it is.</summary>
public sealed partial record MarkdownLayout(IReadOnlyList<MarkdownRun> Runs, IReadOnlyList<Box> CodeBoxes, double Height)
{
    public static readonly MarkdownLayout Empty = new([], [], 0);

    /// <summary>Space between blocks.</summary>
    const double Gap = 6;

    /// <summary>The padding of a code block: across, then down.</summary>
    const double CodeAcross = 8, CodeDown = 6;

    static double LineHeight(double size) => Math.Ceiling(size * 1.45);

    static double HeadingScale(int level) => level switch { 1 => 1.4, 2 => 1.2, 3 => 1.05, _ => 1.0 };

    /// <summary>Words with the space after them; a piece of a word ends with no space.</summary>
    [GeneratedRegex(@"\S*\s+|\S+")]
    private static partial Regex Pieces();

    /// <summary>The blocks of <paramref name="markdown"/> laid out to <paramref name="width"/>, in the look's text size and fonts.</summary>
    public static MarkdownLayout Of(string markdown, double width, Fonts fonts, ThemeLook look)
    {
        var runs = new List<MarkdownRun>();
        var codeBoxes = new List<Box>();
        var size = look.Size("text");
        var y = 0.0;
        var first = true;
        foreach (var block in MarkdownBlocks.Read(markdown))
        {
            if (!first)
                y += Gap;
            first = false;
            switch (block.Kind)
            {
                case "heading":
                    var heading = size * HeadingScale(block.Level);
                    y = Flow(runs, block.Spans, 0, width, y, heading, "text", bold: true, fonts);
                    break;
                case "item":
                    var indent = block.Depth * 1.25 * size;
                    var marker = block.Number is int n ? $"{n}." : "\u2022";
                    var markerWidth = Math.Max(size, fonts.Measure(marker, "text", size, false));
                    var markerShown = fonts.Measure(marker, "text", size, false);
                    runs.Add(new MarkdownRun(marker, new Box(indent + markerWidth - markerShown, y, markerShown, LineHeight(size)), size, "text"));
                    y = Flow(runs, block.Spans, indent + markerWidth + size * 0.5, width, y, size, "text", bold: false, fonts);
                    break;
                case "code":
                    var top = y;
                    var lines = block.Text.Split('\n');
                    y += CodeDown;
                    foreach (var line in lines)
                        y = Flow(runs, [new MarkdownSpan(line, Code: true)], CodeAcross, width - CodeAcross, y, size, "mono", bold: false, fonts, codeBlock: true);
                    y += CodeDown;
                    codeBoxes.Add(new Box(0, top, width, y - top));
                    break;
                default:
                    y = Flow(runs, block.Spans, 0, width, y, size, "text", bold: false, fonts);
                    break;
            }
        }
        return new MarkdownLayout(runs, codeBoxes, Math.Ceiling(y));
    }

    /// <summary>
    /// Places the spans from <paramref name="left"/> to <paramref name="right"/>,
    /// from <paramref name="y"/> down, a word on the next line when it does not
    /// fit, and a word wider than a line cut where the line ends. Gives where
    /// the next block starts.
    /// </summary>
    static double Flow(List<MarkdownRun> runs, IReadOnlyList<MarkdownSpan> spans, double left, double right, double y, double size, string font, bool bold, Fonts fonts, bool codeBlock = false)
    {
        var height = LineHeight(size);
        var x = left;
        var first = runs.Count;
        // A word may be made of pieces of spans that look different ("then" and "."): it moves to a new line as a whole.
        var word = new List<(string Text, MarkdownSpan Span, string Font, bool Bold, double Width)>();
        void Place()
        {
            if (word.Count == 0)
                return;
            var shown = word.Sum(p => p.Width) - TrailingSpace(word[^1]);
            if (x > left && x + shown > right)
            {
                x = left;
                y += height;
            }
            foreach (var piece in word)
            {
                var (text, pieceWidth) = (piece.Text, piece.Width);
                // A piece wider than what is left of the line is cut where it ends, character by character.
                while (x + pieceWidth - TrailingSpace(piece with { Text = text }) > right && text.Length > 1)
                {
                    var fits = Fits(text, right - x, piece.Font, size, piece.Bold, fonts);
                    if (fits == 0 && x > left)
                    {
                        x = left;
                        y += height;
                        continue;
                    }
                    fits = Math.Max(1, fits);
                    var cut = text[..fits];
                    var cutWidth = fonts.Measure(cut, piece.Font, size, piece.Bold);
                    runs.Add(Run(cut, new Box(x, y, cutWidth, height), piece.Span, piece.Font, piece.Bold, size, codeBlock));
                    text = text[fits..];
                    pieceWidth = fonts.Measure(text, piece.Font, size, piece.Bold);
                    x = left;
                    y += height;
                }
                runs.Add(Run(text, new Box(x, y, pieceWidth, height), piece.Span, piece.Font, piece.Bold, size, codeBlock));
                x += pieceWidth;
            }
            word.Clear();
        }
        double TrailingSpace((string Text, MarkdownSpan Span, string Font, bool Bold, double Width) piece)
        {
            var trimmed = piece.Text.TrimEnd();
            return trimmed.Length == piece.Text.Length ? 0 : fonts.Measure(piece.Text[trimmed.Length..], piece.Font, size, piece.Bold);
        }
        foreach (var span in spans)
        {
            var spanFont = span.Code ? "mono" : font;
            var spanBold = bold || span.Bold;
            foreach (Match match in Pieces().Matches(span.Text))
            {
                var text = match.Value;
                word.Add((text, span, spanFont, spanBold, fonts.Measure(text, spanFont, size, spanBold)));
                if (char.IsWhiteSpace(text[^1]))
                    Place();
            }
        }
        Place();
        if (!codeBlock)
            Reorder(runs, first, left, right, fonts);
        return y + height;
    }

    /// <summary>
    /// A paragraph with Hebrew or Arabic in it, its words placed as they are
    /// read: on each line the words in visual order (Bidi), a word that runs
    /// right to left with its space on its left, and a paragraph that starts
    /// right to left set against the right edge.
    /// </summary>
    static void Reorder(List<MarkdownRun> runs, int first, double left, double right, Fonts fonts)
    {
        if (first >= runs.Count)
            return;
        var block = string.Concat(runs.Skip(first).Select(r => r.Text));
        if (!Bidi.HasRightToLeft(block))
            return;
        var paragraph = Bidi.ParagraphLevel(block);
        var lines = Enumerable.Range(first, runs.Count - first).GroupBy(i => runs[i].Box.Y).ToList();
        foreach (var line in lines)
        {
            var members = line.ToList();
            var text = string.Concat(members.Select(i => runs[i].Text));
            var levels = Bidi.Levels(text, paragraph);
            var runLevels = new List<int>(members.Count);
            var at = 0;
            foreach (var i in members)
            {
                var piece = runs[i].Text;
                var k = 0;
                while (k < piece.Length && char.IsWhiteSpace(piece[k]))
                    k++;
                runLevels.Add(k < piece.Length ? levels[at + k] : paragraph);
                at += piece.Length;
            }
            var order = Bidi.VisualOrder(runLevels);
            var width = members.Sum(i => runs[i].Box.Width);
            var x = paragraph == 1 ? Math.Max(left, right - width) : left;
            foreach (var o in order)
            {
                var run = runs[members[o]];
                var box = run.Box with { X = x };
                if (runLevels[o] % 2 == 1)
                {
                    var trimmed = run.Text.TrimEnd();
                    var space = trimmed.Length == run.Text.Length ? 0 : run.Box.Width - fonts.Measure(trimmed, run.Font, run.Size, run.Bold);
                    box = box with { X = x + space, Width = run.Box.Width - space };
                }
                runs[members[o]] = run with { Box = box };
                x += run.Box.Width;
            }
        }
    }

    static int Fits(string text, double width, string font, double size, bool bold, Fonts fonts)
    {
        var count = 0;
        while (count < text.Length && fonts.Measure(text[..(count + 1)], font, size, bold) <= width)
            count++;
        return count;
    }

    static MarkdownRun Run(string text, Box box, MarkdownSpan span, string font, bool bold, double size, bool codeBlock) =>
        new(text, box, size, font, bold, span.Italic, span.Code || codeBlock, span.Link);
}

/// <summary>The blocks of a markdown part, laid out to its width; it opens a link pressed and released on.</summary>
public sealed class MarkdownPage(MarkdownPart part) : Element
{
    MarkdownLayout laid = MarkdownLayout.Empty;
    (string Text, double Width, ThemeLook Look)? laidFor;
    string? pressedLink;

    /// <summary>The runs it shows, as last laid out.</summary>
    public IReadOnlyList<MarkdownRun> Runs => laid.Runs;

    /// <summary>The places of its code blocks, as last laid out.</summary>
    public IReadOnlyList<Box> CodeBoxes => laid.CodeBoxes;

    /// <summary>Its blocks laid out to <paramref name="width"/>, again only when its text, width or look changed.</summary>
    public MarkdownLayout LayOut(double width)
    {
        var text = part.Markdown;
        var look = LookNow;
        if (laidFor is not { } was || was.Text != text || was.Width != width || was.Look != look)
        {
            laid = MarkdownLayout.Of(text, width, FontsNow, look);
            laidFor = (text, width, look);
        }
        return laid;
    }

    /// <summary>The address of the link at a point of the page, from its top left; none off a link.</summary>
    public string? LinkAt(double x, double y) => laid.Runs.FirstOrDefault(r => r.Link is not null && r.Box.Contains(x, y))?.Link;

    public override void OnPointerPressed(PointerInput at) => pressedLink = LinkAt(at.X, at.Y);

    /// <summary>The hand over a link, as Avalonia's Markdown view showed it.</summary>
    public override Cursor? CursorAt(double x, double y) => part.Enabled && LinkAt(x - Bounds.X, y - Bounds.Y) is not null ? Cursor.Hand : null;

    public override void OnPointerReleased(PointerInput at)
    {
        var link = pressedLink;
        pressedLink = null;
        if (link is not null && part.Enabled && LinkAt(at.X, at.Y) == link)
            part.Host.OpenAddress(link);
    }

    public override void Draw(Painter painter)
    {
        var look = painter.Look;
        var shade = look.Color("muted").WithAlpha(31);
        var radius = look.Radius("input");
        foreach (var box in laid.CodeBoxes)
            painter.Fill(box.Offset(Bounds.X, Bounds.Y), shade, radius);
        foreach (var run in laid.Runs)
        {
            var box = run.Box.Offset(Bounds.X, Bounds.Y);
            using var font = painter.Fonts.Font(run.Font, run.Size, run.Bold);
            var color = run.Link is not null ? look.Color("accent") : look.Color("text");
            using var paint = new SKPaint { Color = color, IsAntialias = true };
            var metrics = font.Metrics;
            var baseline = box.Y + (box.Height - (metrics.Descent - metrics.Ascent)) / 2 - metrics.Ascent;
            var text = run.Text.TrimEnd();
            // Shaped as every text is: Arabic joined, Hebrew right to left.
            var width = painter.Fonts.Measure(text, run.Font, run.Size, run.Bold);
            if (run.Code && laid.CodeBoxes.All(c => !c.Contains(run.Box.X, run.Box.Y + 1)) && text.Length > 0)
                painter.Fill(new Box(box.X - 1, box.Y + 1, width + 2, box.Height - 2), shade, 3);
            painter.Fonts.Draw(painter.Canvas, text, box.X, baseline, run.Font, run.Size, run.Bold, paint, run.Italic ? -0.2f : 0);
            if (run.Link is not null && text.Length > 0)
                painter.Fill(new Box(box.X, Math.Round(baseline + 2), width, 1), color);
        }
    }
}

[PartKind("markdown")]
public sealed class MarkdownPart : Part
{
    string? resourceName;
    string resourceText = "";
    MarkdownPage page = null!;
    ScrollView scroll = null!;

    public override bool Grows => true;

    /// <summary>The page of its blocks.</summary>
    public MarkdownPage Page => page;

    /// <summary>Where its page scrolls.</summary>
    public ScrollView Scroll => scroll;

    /// <summary>The Markdown it shows: its resource file, its key's text, or what it binds.</summary>
    public string Markdown
    {
        get
        {
            if (Spec.Text("source") is { Length: > 0 } source)
                return Resource(source);
            if (Spec.Text("caption") is { Length: > 0 } caption)
                return Host.Text(caption);
            return Bound is { } path && Host.Read(path) is string text ? text : "";
        }
    }

    /// <summary>The text of a resource file, read once per name; "" when the application carries none.</summary>
    string Resource(string name)
    {
        if (name != resourceName)
        {
            resourceName = name;
            var bytes = Host.Resource(name);
            resourceText = bytes is null ? "" : new UTF8Encoding(false).GetString(bytes).TrimStart('\uFEFF');
        }
        return resourceText;
    }

    protected internal override void Made()
    {
        page = new MarkdownPage(this);
        scroll = new ScrollView(page) { Measure = (width, _) => (width, page.LayOut(Math.Max(0, width - ScrollView.BarThickness)).Height) };
        AddChild(scroll);
        Host.Changed += Invalidate;
    }

    public override void Arrange(Box box)
    {
        Bounds = box;
        scroll.Arrange(box);
    }

    public override void Draw(Painter painter)
    {
        if (!IsShown)
            return;
        base.Draw(painter);
    }
}
