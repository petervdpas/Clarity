// What a screen draws: the theme's background, a plate at its place in the
// grid, a line of text inside its box only, and the same at twice the scale.

namespace Clarity.Ui.Tests;

public class DrawingTests
{
    static readonly ThemeLook Look = ThemeLook.Clarity(dark: false);

    [Fact]
    public void A_window_is_the_theme_s_background_with_its_parts_at_their_places()
    {
        var grid = new GridPanel(columns: 2);
        grid.Add(new Plate("accent"), new GridItem(2, 2, 1, 1, false));
        var screen = new Screen(grid, Look);
        using var pixels = Shot.Of(screen, 400, 200);
        // Column 2 starts at 204; row 2 at 44 and is 36 high.
        Assert.Equal(Look.Color("accent"), pixels.GetPixel(300, 60));
        Assert.Equal(Look.Color("background"), pixels.GetPixel(100, 60));
        Assert.Equal(Look.Color("background"), pixels.GetPixel(300, 20));
        Assert.Equal(Look.Color("background"), pixels.GetPixel(300, 90));
    }

    [Fact]
    public void Twice_the_scale_draws_the_same_window_in_twice_the_pixels()
    {
        var grid = new GridPanel(columns: 2);
        grid.Add(new Plate("accent"), new GridItem(2, 2, 1, 1, false));
        using var pixels = Shot.Of(new Screen(grid, Look), 800, 400, scale: 2);
        Assert.Equal(Look.Color("accent"), pixels.GetPixel(600, 120));
        Assert.Equal(Look.Color("background"), pixels.GetPixel(600, 40));
    }

    [Fact]
    public void Text_is_drawn_in_its_box_and_nowhere_else()
    {
        var grid = new GridPanel(columns: 1);
        grid.Add(new TextLine("Hello, Clarity"), new GridItem(1, 1, 1, 1, false));
        using var pixels = Shot.Of(new Screen(grid, Look), 300, 120);
        var background = Look.Color("background");
        var inked = 0;
        for (var y = 0; y < 120; y++)
            for (var x = 0; x < 300; x++)
                if (pixels.GetPixel(x, y) != background)
                {
                    Assert.True(y < 36, $"ink at {x},{y}, below the text's row");
                    inked++;
                }
        Assert.True(inked > 50, "the text drew nothing");
    }

    [Fact]
    public void Text_is_measured_with_the_theme_s_font_size()
    {
        var fonts = new Fonts(Look);
        var small = fonts.Measure("Clarity", "text", size: 12, bold: false);
        var large = fonts.Measure("Clarity", "text", size: 24, bold: false);
        Assert.True(small > 10);
        Assert.True(large > small * 1.8);
    }

    [Fact]
    public void Inter_is_carried_for_where_there_are_no_fonts_of_the_system()
    {
        Assert.Equal("Inter", Fonts.Carried(bold: false).FamilyName);
        Assert.Equal("Inter", Fonts.Carried(bold: true).FamilyName);
        Assert.True(Fonts.Carried(bold: true).FontWeight > Fonts.Carried(bold: false).FontWeight);
    }
}
