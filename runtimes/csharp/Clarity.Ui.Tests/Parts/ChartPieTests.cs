// A pie of sums by a reference that may be empty (a `ref` field is a long?
// in the generated record), as examples/agents/drawing draws it: its slices
// show, in the palette's colors.

using Clarity.Runtime;

namespace Clarity.Ui.Tests;

public class ChartPieTests
{
    public sealed class Item
    {
        public long? Owner { get; set; }
        public decimal Price { get; set; }
    }

    [Fact]
    public void A_pie_of_sums_by_a_reference_draws_its_slices()
    {
        var host = new TestHost();
        host.Lists["items"] = [new Item { Owner = 1, Price = 12 }, new Item { Owner = 2, Price = 36 }, new Item { Owner = 1, Price = 24 }];
        host.Offered["items.owner"] = [new(1L, "Ada"), new(2L, "Bo")];
        var (screen, _, parts) = Made.Window("""<Chart name="byOwner" at="1,1" span="6" rows="6" bind="items" chart="pie" by="owner" measure="{&quot;field&quot;:&quot;price&quot;,&quot;kind&quot;:&quot;sum&quot;}" />""", host);
        var chart = Assert.IsType<ChartPart>(parts[0]);
        Assert.Equal(["Ada", "Bo"], chart.Groups().Select(g => g.Label));
        Assert.Equal([36.0, 36.0], chart.Groups().Select(g => g.Value));
        using var shot = Shot.Of(screen, 800, 400);
        var slices = chart.Marks(chart.Bounds);
        Assert.Equal(2, slices.Count);
        var (x, y) = slices[0].Center;
        Assert.Equal(SKColor.Parse(slices[0].Color!), shot.PixelAt((int)x, (int)y));
    }

    [Fact]
    public void A_reference_its_choices_do_not_name_yet_still_counts()
    {
        var host = new TestHost();
        host.Lists["items"] = [new Item { Owner = 1, Price = 12 }, new Item { Owner = 3, Price = 5 }, new Item { Owner = null, Price = 1 }];
        // Read before the records were added: only the empty choice and the first.
        host.Offered["items.owner"] = [new(null, ""), new(1L, "Ada")];
        var (_, _, parts) = Made.Window("""<Chart name="byOwner" at="1,1" span="6" rows="6" bind="items" chart="pie" by="owner" measure="{&quot;field&quot;:&quot;price&quot;,&quot;kind&quot;:&quot;sum&quot;}" />""", host);
        var chart = Assert.IsType<ChartPart>(parts[0]);
        Assert.Equal(["3", "Ada"], chart.Groups().Select(g => g.Label));
    }
}
