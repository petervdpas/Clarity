// The small parts that show text: a heading, a text (a bound value on one
// line, or wrapped over several rows and scrolling), a separator and a badge.

using System.Globalization;
using Clarity.Runtime;

namespace Clarity.Ui.Tests.Parts;

enum SmallTextLevel
{
    Low,
    High,
}

public class SmallTextPartsTests
{
    [Fact]
    public void A_heading_draws_its_caption_in_the_heading_size()
    {
        var host = new TextFactsHost { Texts = { ["ordersTitle"] = "Orders" } };
        var (screen, _, parts) = Made.Window("""<Heading at="1,1" span="6" caption="@ordersTitle" />""", host);
        Assert.Equal("Orders", ((HeadingPart)parts[0]).Text);
        using var pixels = Shot.Of(screen, 800, 400);
        Assert.True(TextEditorTests.Inked(pixels, parts[0].Bounds, screen.Look.Color("text")));
    }

    [Fact]
    public void A_text_shows_its_value_as_the_person_reads_it()
    {
        var host = new TextFactsHost
        {
            Values = { ["total"] = 1250.5m, ["paid"] = true, ["level"] = SmallTextLevel.High, ["name"] = "Ada" },
            Offered = { ["level"] = [new Choice(SmallTextLevel.Low, "Low"), new Choice(SmallTextLevel.High, "Very high")] },
        };
        host.Formatted["total"] = new FieldFormat(FormatKind.Decimal, 2);
        var (_, _, parts) = Made.Window("""
            <Text at="1,1" span="3" bind="total" />
            <Text at="4,1" span="3" bind="paid" />
            <Text at="7,1" span="3" bind="level" />
            <Text at="10,1" span="3" bind="name" />
            <Text at="1,2" span="3" />
            """, host);
        Assert.Equal(["1,250.50", "Yes", "Very high", "Ada", ""], parts.Select(p => ((TextPart)p).Text));
    }

    [Fact]
    public void A_text_over_several_rows_wraps_and_scrolls_with_the_wheel()
    {
        var host = new TextFactsHost { Values = { ["story"] = string.Join(" ", Enumerable.Repeat("Once upon a time", 30)) + "\nThe end." } };
        var (screen, _, parts) = Made.Window("""<Text at="1,1" span="2" rows="2" bind="story" />""", host);
        var text = (TextPart)parts[0];
        using (Shot.Of(screen, 800, 400)) { }
        Assert.NotNull(text.Lines);
        Assert.False(text.Lines!.Focusable);
        Assert.Equal(host.Values["story"], text.Lines.Text);
        var b = text.Bounds;
        screen.Wheel(b.X + 5, b.Y + 5, 0, 40);
        Assert.Equal(40, text.Lines.ScrollY);
        // A value changed by code shows.
        host.Values["story"] = "Short.";
        using (Shot.Of(screen, 800, 400)) { }
        Assert.Equal("Short.", text.Lines.Text);
    }

    [Fact]
    public void A_long_text_on_one_line_is_cut_off_with_an_ellipsis()
    {
        var host = new TextFactsHost { Values = { ["note"] = new string('m', 200) } };
        var (screen, _, parts) = Made.Window("""<Text at="1,1" span="1" bind="note" />""", host);
        using (Shot.Of(screen, 800, 400)) { }
        using var surface = SKSurface.Create(new SKImageInfo(10, 10));
        var painter = new Painter(surface.Canvas, screen.Look, screen.Fonts);
        var fitted = PartTexts.Fit(painter, new string('m', 200), parts[0].Bounds.Width, 14);
        Assert.EndsWith("\u2026", fitted);
        Assert.True(screen.Fonts.Measure(fitted, "text", 14, false) <= parts[0].Bounds.Width);
    }

    [Fact]
    public void A_separator_is_a_line_across_or_down_in_the_strong_border_color()
    {
        var host = new TextFactsHost();
        var (screen, _, parts) = Made.Window("""
            <Separator at="1,1" span="12" />
            <Separator at="1,2" rows="3" orientation="vertical" />
            """, host);
        Assert.False(((SeparatorPart)parts[0]).Vertical);
        Assert.True(((SeparatorPart)parts[1]).Vertical);
        using var pixels = Shot.Of(screen, 800, 400);
        var across = parts[0].Bounds;
        var line = screen.Look.Color("strong_border");
        Assert.Equal(line, pixels.PixelAt((int)(across.X + across.Width / 2), (int)(across.Y + across.Height / 2)));
        var down = parts[1].Bounds;
        Assert.Equal(line, pixels.PixelAt((int)(down.X + down.Width / 2), (int)(down.Y + 10)));
        Assert.Equal(screen.Look.Color("background"), pixels.PixelAt((int)down.X + 1, (int)(down.Y + 10)));
    }

    [Fact]
    public void A_badge_shows_its_caption_or_its_value_in_the_accent_pill()
    {
        var host = new TextFactsHost { Values = { ["unread"] = 1200L } };
        var (screen, _, parts) = Made.Window("""
            <Badge at="1,1" span="2" />
            <Badge at="3,1" span="2" bind="unread" />
            """, host);
        Assert.Equal("New", ((BadgePart)parts[0]).Text);
        Assert.Equal("1,200", ((BadgePart)parts[1]).Text);
        using var pixels = Shot.Of(screen, 800, 400);
        var b = parts[0].Bounds;
        Assert.Equal(screen.Look.Color("accent"), pixels.PixelAt((int)b.X + 4, (int)(b.Y + b.Height / 2)));
        Assert.True(TextEditorTests.Inked(pixels, b, screen.Look.Color("on_accent")));
        Assert.Equal(screen.Look.Color("background"), pixels.PixelAt((int)b.Right - 2, (int)(b.Y + b.Height / 2)));
    }
}
