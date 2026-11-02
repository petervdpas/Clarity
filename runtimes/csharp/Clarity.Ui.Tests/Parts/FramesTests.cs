// A container frames the data parts in it (DESIGN 7.8): a list, table or tree
// inside a panel, a form, a tabs page, a step, a section, a pane, a scroll box
// or a popover shows no border of its own unless it has `frame`; on the window
// it keeps one. The rule is read as the part draws, from the parts around it.

namespace Clarity.Ui.Tests;

public class FramesTests
{
    const string List = """<List name="folders" at="1,1" span="3" rows="3" bind="folders" show="name" EXTRA />""";
    const string Table = """<Table name="folders" at="1,1" span="3" rows="3" bind="folders" columns="name" EXTRA />""";
    const string Tree = """<Tree name="folders" at="1,1" span="3" rows="3" bind="folders" show="name" EXTRA />""";

    public static TheoryData<string> Containers =>
    [
        """<Panel name="box" at="1,1" span="6" rows="5" caption="Box">PART</Panel>""",
        """<Form at="1,1" span="6" rows="5">PART</Form>""",
        """<Tabs name="pages" at="1,1" span="6" rows="5"><Tab title="One">PART</Tab></Tabs>""",
        """<Steps name="wizard" at="1,1" span="6" rows="6"><Step title="One">PART</Step></Steps>""",
        """<Accordion name="faq" at="1,1" span="6" rows="6"><Section title="One">PART</Section></Accordion>""",
        """<Splitter name="split" at="1,1" span="12" rows="5"><Pane span="6">PART</Pane><Pane /></Splitter>""",
        """<Scroll at="1,1" span="6" rows="5">PART</Scroll>""",
        """<Trend name="save" at="1,1" span="3" /><Popover name="tip" for="save" at="1,2" span="4" rows="4">PART</Popover>""",
    ];

    static RowsPart Data(string markup)
    {
        var host = new DataHost();
        host.Lists["folders"] = [new Folder { Id = 1, Name = "One", Position = 1 }];
        host.State("tip")!.IsVisible = true;
        var (screen, _, parts) = Made.Window(markup, host, width: 808);
        var found = parts.Cast<Element>().Concat(screen.Overlays.Select(o => o.Content)).SelectMany(Below).OfType<RowsPart>();
        return Assert.Single(found);
    }

    static IEnumerable<Element> Below(Element e) => e.Children.SelectMany(Below).Prepend(e);

    [Theory]
    [MemberData(nameof(Containers))]
    public void A_list_table_or_tree_in_a_container_has_no_border_unless_it_has_frame(string container)
    {
        foreach (var part in new[] { List, Table, Tree })
        {
            Assert.False(Data(container.Replace("PART", part.Replace("EXTRA", ""))).Framed);
            Assert.True(Data(container.Replace("PART", part.Replace("EXTRA", "frame=\"true\""))).Framed);
        }
    }

    [Fact]
    public void A_list_table_or_tree_on_the_window_has_its_border()
    {
        foreach (var part in new[] { List, Table, Tree })
            Assert.True(Data(part.Replace("EXTRA", "")).Framed);
    }

    [Fact]
    public void A_list_in_a_panel_draws_no_border_line()
    {
        var host = new DataHost();
        host.Lists["folders"] = [new Folder { Id = 1, Name = "One", Position = 1 }];
        var (screen, _, parts) = Made.Window("""<Panel name="box" at="1,1" span="6" rows="5" caption="Box">""" + List.Replace("EXTRA", "") + "</Panel>", host, width: 808);
        var list = Assert.Single(Below(parts[0]).OfType<ListPart>());
        using var pixels = Shot.Of(screen, 808, 400);
        Assert.NotEqual(screen.Look.Part("list", null).Border, pixels.GetPixel((int)list.Bounds.X + 30, (int)list.Bounds.Y));
    }
}
