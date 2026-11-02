// The small parts that show: a progress bar's fill (conformance/small-parts.json
// `progress`), a busy indicator with its text, an icon of Lucide's path data,
// and an image of the resources by its name, fitted in proportion.

namespace Clarity.Ui.Tests.Containers;

public class SmallDisplayTests
{
    [Fact]
    public void A_progress_bar_is_as_full_as_on_every_renderer()
    {
        foreach (var c in Conformance.Read("small-parts.json").GetProperty("progress").EnumerateArray())
        {
            var value = c.GetProperty("value");
            object? v = value.ValueKind == System.Text.Json.JsonValueKind.Null ? null : value.GetDouble();
            var max = c.GetProperty("max");
            Assert.Equal(c.GetProperty("full").GetDouble(), ProgressPart.Full(v, max.ValueKind == System.Text.Json.JsonValueKind.Null ? null : max.GetString()), 6);
        }
    }

    [Fact]
    public void A_progress_bar_fills_in_the_accent_as_far_as_its_value()
    {
        var host = new TestHost();
        host.Values["done"] = 125L;
        var (screen, _, parts) = Made.Window("""<Progress at="1,1" span="12" bind="done" max="250" />""", host, width: 808);
        Assert.Equal(0.5, ((ProgressPart)parts[0]).Shown);
        using var pixels = Shot.Of(screen, 808, 400);
        Assert.Equal(screen.Look.Color("accent"), pixels.GetPixel(200, 18));
        Assert.Equal(screen.Look.Color("raised"), pixels.GetPixel(600, 18));
    }

    [Fact]
    public void A_busy_indicator_says_what_it_binds_else_its_caption()
    {
        var host = new TestHost();
        host.Texts["loading"] = "Loading orders";
        var (screen, _, parts) = Made.Window("""
            <Busy at="1,1" span="4" caption="@loading" />
            <Busy at="1,2" span="4" bind="status" />
            """, host, width: 808);
        Assert.Equal("Loading orders", ((BusyPart)parts[0]).Text);
        host.Values["status"] = "Saving";
        Assert.Equal("Saving", ((BusyPart)parts[1]).Text);
        // Its indicator turns by its clock.
        var busy = (BusyPart)parts[0];
        busy.Clock = () => 0;
        using var pixels = Shot.Of(screen, 808, 400);
        // At the start of a turn its arc starts at the top, in the accent.
        var top = pixels.GetPixel(8, 11);
        Assert.True(top.Blue > top.Red + 50, $"the top of the indicator is {top}");
    }

    [Fact]
    public void An_icon_is_found_by_its_name_as_the_generated_icons_class_names_it()
    {
        Assert.Equal("TriangleAlert", IconPart.Member("triangle-alert"));
        Assert.Equal("ArrowUp10", IconPart.Member("arrow-up-1-0"));
        Assert.Equal("_1", IconPart.Member("1"));
    }

    [Fact]
    public void An_icon_draws_its_path_in_the_texts_color()
    {
        var host = new TestHost { Icons = { ["minus"] = "M 2 12 L 22 12" } };
        var (screen, _, parts) = Made.Window("""<Icon at="1,1" span="1" icon="minus" />""", host, width: 808);
        using var pixels = Shot.Of(screen, 808, 400);
        var box = parts[0].Bounds;
        // A line across the middle of a 20 high icon in the middle of its place.
        Assert.True(pixels.GetPixel((int)(box.X + box.Width / 2), (int)(box.Y + box.Height / 2)).Red < 100);
        Assert.Equal(screen.Look.Color("background"), pixels.GetPixel((int)(box.X + box.Width / 2), (int)(box.Y + 4)));
    }

    [Fact]
    public void An_image_shows_its_picture_fitted_in_proportion()
    {
        using var red = new SKBitmap(200, 100);
        red.Erase(SKColors.Red);
        using var encoded = red.Encode(SKEncodedImageFormat.Png, 100);
        var host = new TestHost { Resources = { ["small-display-red.png"] = encoded.ToArray() } };
        var (screen, _, parts) = Made.Window("""
            <Image at="1,1" span="6" rows="3" source="small-display-red.png" />
            <Image at="7,1" span="6" rows="3" source="small-display-not-there.png" />
            """, host, width: 808, height: 124);
        var box = parts[0].Bounds;
        Assert.Equal(new Box(0, 0, 400, 124), box);
        var fitted = ImagePart.Fit(box, 200, 100);
        Assert.Equal(new Box(76, 0, 248, 124), fitted);
        using var pixels = Shot.Of(screen, 808, 124);
        Assert.Equal(SKColors.Red, pixels.GetPixel(200, 62));
        Assert.Equal(screen.Look.Color("background"), pixels.GetPixel(20, 62));
        // One that is not there shows its name in a dashed frame.
        Assert.Null(((ImagePart)parts[1]).Shown);
        Assert.True(parts[1].Grows);
    }
}
