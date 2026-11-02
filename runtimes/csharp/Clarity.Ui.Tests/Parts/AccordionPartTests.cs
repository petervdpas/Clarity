// An accordion against the cases every renderer shares
// (conformance/accordions.json): its headers pressed, the open section
// shown, the part's state from 1 and 0 for none.

using Clarity.Runtime;

namespace Clarity.Ui.Tests.Containers;

public class AccordionPartTests
{
    static readonly System.Text.Json.JsonElement File = Conformance.Read("accordions.json");

    static (Screen Screen, AccordionPart Accordion, TestHost Host) Accordion(int sections, long open)
    {
        var host = new TestHost();
        host.State("faq")!.OpenSection = open;
        var markup = """<Accordion name="faq" at="1,1" span="6" rows="8" onchange="opened">"""
            + string.Concat(Enumerable.Range(1, sections).Select(i => $"""<Section title="Section {i}"><Trend at="1,1" span="3" /></Section>"""))
            + "</Accordion>";
        var (screen, _, parts) = Made.Window(markup, host);
        return (screen, Assert.IsType<AccordionPart>(parts[0]), host);
    }

    [Fact]
    public void The_open_section_shows_as_on_every_renderer()
    {
        foreach (var c in File.GetProperty("shown").EnumerateArray())
        {
            var (_, accordion, _) = Accordion(c.GetProperty("sections").GetInt32(), c.GetProperty("open").GetInt64());
            var shown = c.GetProperty("shown").GetInt64();
            Assert.Equal(shown, AccordionPart.Shown(accordion.Open, accordion.Pages.Count));
            for (var i = 0; i < accordion.Pages.Count; i++)
            {
                Assert.Equal(i + 1 == shown, accordion.Children.Contains(accordion.Pages[i]));
                Assert.Equal(i + 1 == shown, accordion.Headers[i].Chevron);
            }
        }
    }

    [Fact]
    public void A_press_on_a_header_opens_or_folds_as_on_every_renderer()
    {
        foreach (var c in File.GetProperty("press").EnumerateArray())
        {
            var (screen, accordion, host) = Accordion(c.GetProperty("sections").GetInt32(), c.GetProperty("open").GetInt64());
            var header = accordion.Headers[c.GetProperty("press").GetInt32() - 1].Bounds;
            screen.PointerPressed(header.X + 20, header.Y + 10, PointerButton.Left);
            screen.PointerReleased(header.X + 20, header.Y + 10, PointerButton.Left);
            Assert.Equal(c.GetProperty("open after").GetInt64(), host.State("faq")!.OpenSection);
            Assert.Equal(["run faq change opened"], host.Log);
        }
    }

    [Fact]
    public void An_accordions_part_state_opens_the_first_section()
    {
        foreach (var c in File.GetProperty("open").EnumerateArray())
        {
            var part = new PartState("faq");
            if (c.GetProperty("part").TryGetProperty("open", out var open))
                part.OpenSection = open.GetInt64();
            Assert.Equal(c.GetProperty("open").GetInt64(), part.OpenSection);
        }
        foreach (var c in File.GetProperty("set").EnumerateArray())
        {
            var part = new PartState("faq") { OpenSection = c.GetProperty("open").GetInt64() };
            Assert.Equal(c.GetProperty("part").GetProperty("open").GetInt64(), part.OpenSection);
        }
    }

    [Fact]
    public void The_open_sections_parts_fill_the_room_under_its_header_and_the_rest_follow()
    {
        var (_, accordion, _) = Accordion(3, 2);
        var headers = accordion.Headers.Select(h => h.Bounds).ToList();
        var page = accordion.Pages[1].Bounds;
        Assert.True(page.Y >= headers[1].Bottom);
        Assert.True(headers[2].Y >= page.Bottom);
        Assert.True(headers[2].Bottom <= accordion.Bounds.Bottom + 0.001);
    }
}
