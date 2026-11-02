// The link against the cases every renderer shares (conformance/small-parts.json):
// what a bound link opens for its value; a link opens its address, the one
// its value holds, or runs its click when it has neither.

using System.Text.Json;

namespace Clarity.Ui.Tests;

public class LinkPartTests
{
    static readonly JsonElement File = JsonDocument.Parse(System.IO.File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "small-parts.json"))).RootElement;

    static void Click(Screen screen, Element part)
    {
        screen.PointerPressed(part.Bounds.X + 4, part.Bounds.Y + 18, PointerButton.Left);
        screen.PointerReleased(part.Bounds.X + 4, part.Bounds.Y + 18, PointerButton.Left);
    }

    [Fact]
    public void A_bound_value_opens_as_on_every_renderer()
    {
        foreach (var c in File.GetProperty("address").EnumerateArray())
        {
            var value = c.GetProperty("value");
            var opens = c.GetProperty("opens");
            Assert.Equal(opens.ValueKind == JsonValueKind.Null ? null : opens.GetString(), LinkPart.Address(value.ValueKind == JsonValueKind.Null ? null : value.GetString()));
        }
    }

    [Fact]
    public void It_opens_its_address_its_values_or_runs_its_click()
    {
        var host = new TestHost();
        host.Values["shop.mail"] = "ada@example.org";
        host.Values["shop.site"] = "lucide.dev";
        var (screen, _, parts) = Made.Window("""
            <Link at="1,1" span="3" name="docs" to="https://example.org/docs" onclick="never" />
            <Link at="1,2" span="3" name="mail" bind="shop.mail" />
            <Link at="1,3" span="3" name="site" bind="shop.site" onclick="never" />
            <Link at="1,4" span="3" name="more" caption="More" onclick="showMore" />
            """, host);
        foreach (var part in parts)
            Click(screen, part);
        Assert.Equal(["https://example.org/docs", "mailto:ada@example.org"], host.Opened);
        Assert.Equal(["run more click showMore"], host.Log);
        Assert.Equal("ada@example.org", ((LinkPart)parts[1]).Shown);
        Assert.Equal("More", ((LinkPart)parts[3]).Shown);
    }

    [Fact]
    public void A_disabled_link_does_nothing()
    {
        var host = new TestHost();
        var (screen, _, parts) = Made.Window("""<Link at="1,1" span="3" name="more" onclick="showMore" />""", host);
        host.State("more")!.IsEnabled = false;
        Click(screen, parts[0]);
        Assert.Empty(host.Log);
    }
}
