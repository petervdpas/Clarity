// Text shaped as Avalonia shaped it (HarfBuzz and the Unicode bidirectional
// algorithm): Arabic letters joined, Hebrew right to left, mixed Latin and
// Hebrew each in its own direction, the caret and the pointer on the shaped
// letters, an ellipsis at the end of a right-to-left label, and Latin drawn
// exactly as before. The test fonts are Inter (Clarity.UI's own) and Noto
// Sans Arabic and Noto Sans Hebrew, carried by this project (Fonts/, SIL
// Open Font License), so the results are the same on every machine.

namespace Clarity.Ui.Tests;

public class ShapingTests
{
    const string Salaam = "\u0633\u0644\u0627\u0645";          // Arabic "salaam", four letters that join
    const string Shalom = "\u05e9\u05dc\u05d5\u05dd";          // Hebrew "shalom"
    const string ShalomOlam = "\u05e9\u05dc\u05d5\u05dd \u05e2\u05d5\u05dc\u05dd"; // Hebrew "shalom olam"
    const double Size = 16;

    static readonly Lazy<bool> carried = new(() =>
    {
        foreach (var name in new[] { "NotoSansArabic-Regular.ttf", "NotoSansHebrew-Regular.ttf" })
            Assert.True(Fonts.AddFallback(File.ReadAllBytes(Path.Combine(AppContext.BaseDirectory, "Fonts", name))));
        return true;
    });

    /// <summary>A look whose text font is Inter, so the scripts it lacks come from the carried fallbacks.</summary>
    internal static ThemeLook Look()
    {
        _ = carried.Value;
        return ThemeLook.Over("""{ "fonts": { "text": "Inter" } }""", ThemeLook.Clarity(dark: false));
    }

    static Fonts NewFonts() => new(Look());

    // -- Bidi ---------------------------------------------------------------------

    [Fact]
    public void The_bidi_levels_follow_the_first_strong_letter()
    {
        Assert.Equal(0, Bidi.ParagraphLevel("abc " + Shalom));
        Assert.Equal(1, Bidi.ParagraphLevel("123 " + Shalom + " abc"));
        Assert.Equal([0, 0, 0, 1, 1, 1, 1, 0], Bidi.Levels("ab " + Shalom + " ").Select(l => (int)l));
        // A space between Hebrew words is Hebrew; the line's last space goes to the paragraph's level.
        Assert.Equal([1, 1, 1, 1, 1, 1, 1, 1, 1], Bidi.Levels(ShalomOlam + " ", 0).Take(9).Select(l => (int)l));
        Assert.Equal(0, Bidi.Levels(ShalomOlam + " ", 0)[^1]);
        // Numbers in right-to-left text run left to right (level 2).
        Assert.Equal([1, 1, 2, 2], Bidi.Levels(Shalom[..1] + " 12").Select(l => (int)l));
    }

    [Fact]
    public void Runs_of_a_higher_level_turn_around()
    {
        Assert.Equal([0, 3, 2, 1, 4], Bidi.VisualOrder([0, 1, 1, 1, 0]));
        Assert.Equal([2, 1, 0], Bidi.VisualOrder([1, 1, 1]));
        Assert.Equal([0, 4, 2, 3, 1], Bidi.VisualOrder([0, 1, 2, 2, 1]));
    }

    // -- Shaping ------------------------------------------------------------------

    [Fact]
    public void An_arabic_word_draws_joined()
    {
        var fonts = NewFonts();
        var word = fonts.Shape(Salaam, "text", Size, false);
        Assert.True(word.Shaped);
        // Each letter alone is its isolated form; joined, the letters take their initial, middle and final forms.
        var apart = Salaam.Select(c => fonts.Shape(c.ToString(), "text", Size, false)).ToList();
        var isolated = apart.SelectMany(s => s.Glyphs).ToList();
        Assert.Equal(4, isolated.Count);
        Assert.NotEqual(isolated.OrderBy(g => g), word.Glyphs.OrderBy(g => g));
        Assert.NotEqual(apart.Sum(s => s.Width), word.Width, 1);
    }

    [Fact]
    public void Hebrew_draws_right_to_left()
    {
        var fonts = NewFonts();
        var word = fonts.Shape(Shalom, "text", Size, false);
        Assert.True(word.RightToLeft);
        // The first letter is at the right, the last at the left.
        Assert.True(word.LeftOf(0) > word.LeftOf(1));
        Assert.True(word.LeftOf(1) > word.LeftOf(2));
        Assert.True(word.LeftOf(2) > word.LeftOf(3));
        Assert.Equal(word.Width, word.RightOf(0), 3);
        Assert.Equal(0, word.LeftOf(3), 3);
        // The glyphs drawn from the left are the letters' own, last first.
        var letters = Shalom.Select(c => fonts.Shape(c.ToString(), "text", Size, false).Glyphs.Single()).Reverse();
        Assert.Equal(letters, word.Glyphs);
    }

    [Fact]
    public void Hebrew_draws_its_letters_where_it_measures_them()
    {
        var fonts = NewFonts();
        var word = fonts.Shape(Shalom, "text", 40, false);
        using var shot = new SKBitmap(200, 60);
        using var canvas = new SKCanvas(shot);
        canvas.Clear(SKColors.White);
        using var paint = new SKPaint { Color = SKColors.Black, IsAntialias = true };
        Assert.Equal(word.Width, fonts.Draw(canvas, Shalom, 10, 45, "text", 40, false, paint));
        // Ink inside the measured width, none past it.
        bool Inked(int x0, int x1)
        {
            for (var x = x0; x < x1; x++)
                for (var y = 0; y < 60; y++)
                    if (shot.PixelAt(x, y).Red < 128)
                        return true;
            return false;
        }
        Assert.True(Inked(10, (int)(10 + word.Width)));
        Assert.False(Inked((int)Math.Ceiling(10 + word.Width) + 2, 200));
        // Each letter's ink is in its own box: the first letter's at the right.
        Assert.True(Inked((int)(10 + word.LeftOf(0)) + 2, (int)(10 + word.RightOf(0)) - 2));
    }

    [Fact]
    public void Mixed_latin_and_hebrew_keep_their_own_directions()
    {
        var fonts = NewFonts();
        // A Latin paragraph: "abc" from the left, then the Hebrew word right to left.
        var latinFirst = fonts.Shape("abc " + Shalom, "text", Size, false);
        Assert.False(latinFirst.RightToLeft);
        Assert.Equal(0, latinFirst.LeftOf(0), 3);
        Assert.True(latinFirst.LeftOf(1) > latinFirst.LeftOf(0));
        Assert.True(latinFirst.LeftOf(4) > latinFirst.LeftOf(7));
        Assert.True(latinFirst.LeftOf(7) >= latinFirst.RightOf(3) - 0.01);
        // A Hebrew paragraph: the Hebrew word at the right, "abc" at the left still left to right.
        var hebrewFirst = fonts.Shape(Shalom + " abc", "text", Size, false);
        Assert.True(hebrewFirst.RightToLeft);
        Assert.Equal(hebrewFirst.Width, hebrewFirst.RightOf(0), 3);
        Assert.Equal(0, hebrewFirst.LeftOf(5), 3);
        Assert.True(hebrewFirst.LeftOf(6) > hebrewFirst.LeftOf(5));
        Assert.True(hebrewFirst.LeftOf(7) > hebrewFirst.LeftOf(6));
    }

    [Fact]
    public void Latin_text_measures_and_draws_as_before()
    {
        var fonts = NewFonts();
        var line = fonts.Shape("Clarity draws text", "text", Size, false);
        Assert.False(line.Shaped);
        using var font = new SKFont(Fonts.Carried(bold: false), (float)Size) { Subpixel = true, Edging = SKFontEdging.SubpixelAntialias };
        Assert.Equal(font.MeasureText("Clarity draws text"), line.Width, 3);
        // The caret places add up each letter's width, as the text box measured them.
        Assert.Equal(font.MeasureText("C"), line.CaretX(1), 3);
        Assert.Equal(font.MeasureText("C") + font.MeasureText("l"), line.CaretX(2), 3);

        // Drawn pixel for pixel as Skia draws it.
        using var shaped = new SKBitmap(240, 30);
        using var plain = new SKBitmap(240, 30);
        using var paint = new SKPaint { Color = SKColors.Black, IsAntialias = true };
        using (var canvas = new SKCanvas(shaped))
        {
            canvas.Clear(SKColors.White);
            fonts.Draw(canvas, "Clarity draws text", 4, 20, "text", Size, false, paint);
        }
        using (var canvas = new SKCanvas(plain))
        {
            canvas.Clear(SKColors.White);
            canvas.DrawText("Clarity draws text", 4, 20, SKTextAlign.Left, font, paint);
        }
        Assert.Equal(plain.Bytes, shaped.Bytes);
    }

    [Fact]
    public void A_line_is_shaped_once_for_its_text_font_and_size()
    {
        var fonts = NewFonts();
        Assert.Same(fonts.Shape(Salaam, "text", Size, false), fonts.Shape(Salaam, "text", Size, false));
        Assert.NotSame(fonts.Shape(Salaam, "text", Size, false), fonts.Shape(Salaam, "text", Size + 1, false));
    }

    [Fact]
    public void Marks_sit_on_their_letter()
    {
        var fonts = NewFonts();
        // A Hebrew letter with a vowel point is one letter: no caret between them, no width of its own.
        var pointed = fonts.Shape("\u05e9\u05b8", "text", Size, false);
        Assert.True(pointed.Shaped);
        Assert.False(pointed.IsStop(1));
        Assert.Equal(fonts.Shape("\u05e9", "text", Size, false).Width, pointed.Width, 1);
    }

    [Fact]
    public void Devanagari_letters_form_their_conjuncts()
    {
        // Clarity carries no Devanagari font: the system's fallback shapes it, where the system has one.
        const string ksha = "\u0915\u094d\u0937";
        var fonts = NewFonts();
        var runs = fonts.Runs(ksha, "text", false);
        if (!runs.All(r => Fonts.Has(r.Typeface, r.Text)))
            Assert.Skip("this system has no Devanagari font");
        var shaped = fonts.Shape(ksha, "text", Size, false);
        Assert.True(shaped.Shaped);
        // Ka, virama and ssa are three characters; the font draws them as fewer glyphs (the conjunct ksha, or half ka and ssa).
        Assert.True(shaped.Glyphs.Count < 3);
        Assert.False(shaped.IsStop(1));
    }

    // -- The caret ----------------------------------------------------------------

    static (Screen Screen, TextEditor Editor) Editor(string text)
    {
        var editor = new TextEditor(false) { Text = text, Framed = false };
        var grid = new GridPanel(columns: 1);
        grid.Add(editor, new GridItem(1, 1, 1, 1, false));
        var screen = new Screen(grid, Look());
        screen.Resize(300, 100, 1);
        screen.Focus(editor);
        return (screen, editor);
    }

    [Fact]
    public void The_caret_in_right_to_left_text_moves_the_way_the_arrow_points()
    {
        var (screen, editor) = Editor(Shalom);
        screen.KeyPressed(new KeyInput(Key.Home, Modifiers.None));
        Assert.Equal(0, editor.Caret);
        // The start of Hebrew is at the right: Left goes on into the word.
        screen.KeyPressed(new KeyInput(Key.Left, Modifiers.None));
        Assert.Equal(1, editor.Caret);
        screen.KeyPressed(new KeyInput(Key.Left, Modifiers.None));
        Assert.Equal(2, editor.Caret);
        screen.KeyPressed(new KeyInput(Key.Right, Modifiers.None));
        Assert.Equal(1, editor.Caret);
        screen.KeyPressed(new KeyInput(Key.End, Modifiers.None));
        screen.KeyPressed(new KeyInput(Key.Right, Modifiers.None));
        Assert.Equal(3, editor.Caret);
        // Shift with Left selects the letter the caret passes.
        screen.KeyPressed(new KeyInput(Key.Home, Modifiers.None));
        screen.KeyPressed(new KeyInput(Key.Left, Modifiers.Shift));
        Assert.Equal(Shalom[..1], editor.SelectedText);
    }

    [Fact]
    public void The_caret_in_mixed_text_moves_right_across_the_line()
    {
        var text = "ab " + Shalom;
        var (screen, editor) = Editor(text);
        var shape = NewFonts().Shape(text, "text", Look().Size("text"), false);
        // Rightward from the left edge every step lands further right, through both directions.
        screen.KeyPressed(new KeyInput(Key.Home, Modifiers.None));
        var seen = new List<double> { shape.CaretX(editor.Caret) };
        for (var i = 0; i < text.Length; i++)
        {
            screen.KeyPressed(new KeyInput(Key.Right, Modifiers.None));
            seen.Add(shape.CaretX(editor.Caret));
        }
        Assert.Equal(seen.Order(), seen);
        Assert.Equal(text.Length + 1, seen.Distinct().Count());
    }

    [Fact]
    public void A_pointer_in_right_to_left_text_lands_between_the_right_letters()
    {
        var (_, editor) = Editor(Shalom);
        var shape = NewFonts().Shape(Shalom, "text", Look().Size("text"), false);
        // From the editor's left; it has no frame, so its text starts there.
        const double x0 = 0;
        // At the right edge is the start, at the left the end.
        Assert.Equal(0, editor.PlaceAt(x0 + shape.Width - 0.5, 5));
        Assert.Equal(4, editor.PlaceAt(x0 + 0.5, 5));
        // Just right of the line between the first two letters is the place between them.
        Assert.Equal(1, editor.PlaceAt(x0 + shape.LeftOf(0) + 1, 5));
        Assert.Equal(2, editor.PlaceAt(x0 + shape.LeftOf(1) + 1, 5));
    }

    // -- Labels and pages -----------------------------------------------------------

    [Fact]
    public void An_ellipsis_cuts_a_right_to_left_label_at_its_end_on_the_left()
    {
        var look = Look();
        var fonts = new Fonts(look);
        using var shot = new SKBitmap(100, 30);
        using var canvas = new SKCanvas(shot);
        var painter = new Painter(canvas, look, fonts);
        var whole = fonts.Measure(ShalomOlam, "text", Size, false);
        var cut = PartTexts.Fit(painter, ShalomOlam, whole * 0.7, Size);
        Assert.EndsWith("\u2026", cut);
        Assert.True(cut.Length < ShalomOlam.Length);
        var shape = fonts.Shape(cut, "text", Size, false);
        // The dots are the leftmost thing drawn; the first letter the rightmost.
        Assert.Equal(0, shape.LeftOf(cut.Length - 1), 3);
        Assert.Equal(shape.Width, shape.RightOf(0), 3);
    }

    [Fact]
    public void A_right_to_left_markdown_paragraph_reads_from_the_right()
    {
        var look = Look();
        var laid = MarkdownLayout.Of(ShalomOlam, 400, new Fonts(look), look);
        var first = laid.Runs.Single(r => r.Text.StartsWith(Shalom[0]));
        var second = laid.Runs.Single(r => r.Text.StartsWith('\u05e2'));
        Assert.True(first.Box.X > second.Box.X);
        Assert.True(first.Box.Right > 380);
    }
}
