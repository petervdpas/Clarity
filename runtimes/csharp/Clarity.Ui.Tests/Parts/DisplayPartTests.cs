// The display part's text and segments against the cases every renderer
// shares (conformance/display.json), and the digits drawn in its place.

namespace Clarity.Ui.Tests.Containers;

public class DisplayPartTests
{
    static readonly System.Text.Json.JsonElement File = Conformance.Read("display.json");

    [Fact]
    public void A_value_shows_rounded_to_fit_its_digits_or_as_E()
    {
        foreach (var c in File.GetProperty("shows").EnumerateArray())
            Assert.Equal(c.GetProperty("text").GetString(), DisplayPart.ShownText(c.GetProperty("value").GetString()!, c.GetProperty("digits").GetInt32()));
    }

    [Fact]
    public void The_text_fills_the_cells_from_the_right_a_point_lit_on_its_digit()
    {
        foreach (var c in File.GetProperty("cells").EnumerateArray())
        {
            var expected = c.GetProperty("cells").EnumerateArray().Select(x => (x[0].GetString()![0], x[1].GetBoolean())).ToArray();
            Assert.Equal(expected, DisplayPart.Cells(c.GetProperty("text").GetString()!, c.GetProperty("digits").GetInt32()));
        }
    }

    [Fact]
    public void Each_character_lights_its_segments()
    {
        foreach (var lit in File.GetProperty("segments").EnumerateObject())
            Assert.Equal(lit.Value.GetString(), DisplayPart.Segments[lit.Name[0]]);
    }

    [Fact]
    public void A_display_shows_what_it_binds_in_its_place()
    {
        var host = new TestHost();
        host.Values["total"] = 12.5m;
        var (screen, _, parts) = Made.Window("""<Display name="shown" at="1,1" span="4" rows="2" bind="total" digits="4" />""", host, width: 808);
        var display = Assert.IsType<DisplayPart>(parts[0]);
        Assert.Equal("12.5", display.Text);
        host.Values["total"] = 123456m;
        Assert.Equal("E", display.Text);
        host.Values["total"] = null;
        Assert.Equal("0", display.Text);
        host.Values["total"] = 8m;
        using var pixels = Shot.Of(screen, 808, 400);
        // The last cell's middle segment, lit in the text's color, on the right of its place.
        var (left, top, scale) = DisplayPart.Fit(display.Bounds, 5);
        var middle = pixels.PixelAt((int)(left + (6 + 4 * 13) * scale), (int)(top + 10 * scale));
        Assert.Equal(screen.Look.Color("text"), middle);
        // The first cell is dark: only faintly drawn.
        var dark = pixels.PixelAt((int)(left + 6 * scale), (int)(top + 10 * scale));
        Assert.NotEqual(screen.Look.Color("text"), dark);
    }
}
