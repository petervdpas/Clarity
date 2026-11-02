// Tabs: a strip of tabs over the open tab's page, a grid of its parts. The
// open tab is the part's state's Tab, from 1; a tab whose parts are all
// hidden gives way to the first that shows (conformance/ribbons.json).

namespace Clarity.Ui.Tests.Containers;

public class TabsPartTests
{
    const string Tabs = """
        <Tabs name="pages" at="1,1" span="12" rows="6">
          <Tab title="One"><Trend name="first" at="1,1" span="3" /></Tab>
          <Tab title="Two"><Trend name="second" at="1,1" span="3" /></Tab>
          <Tab title="Three" span="6"><Trend name="third" at="4,1" span="3" /></Tab>
        </Tabs>
        """;

    [Fact]
    public void The_first_tab_is_open_and_only_its_page_shows()
    {
        var host = new TestHost();
        var (_, _, parts) = Made.Window(Tabs, host, width: 808);
        var tabs = Assert.IsType<TabsPart>(parts[0]);
        Assert.Equal(1, tabs.Open);
        Assert.Contains(tabs.Views[0], tabs.Children);
        Assert.DoesNotContain(tabs.Views[1], tabs.Children);
        // The page is under the strip.
        Assert.True(tabs.Pages[0].Bounds.Y > TabsPart.StripHeight);
    }

    [Fact]
    public void A_press_on_a_tab_opens_it_in_the_parts_state()
    {
        var host = new TestHost();
        var (screen, _, parts) = Made.Window(Tabs, host, width: 808);
        var tabs = (TabsPart)parts[0];
        var second = tabs.Tabs[1].Bounds;
        screen.PointerPressed(second.X + 5, second.Y + 5, PointerButton.Left);
        screen.PointerReleased(second.X + 5, second.Y + 5, PointerButton.Left);
        Assert.Equal(2, host.State("pages")!.Tab);
        screen.Resize(808, 400, 1);
        Assert.Contains(tabs.Views[1], tabs.Children);
        Assert.DoesNotContain(tabs.Views[0], tabs.Children);
        // A handler opens another by its state.
        host.State("pages")!.Tab = 3;
        screen.Resize(808, 400, 1);
        Assert.Equal(3, tabs.Open);
        Assert.Contains(tabs.Views[2], tabs.Children);
    }

    [Fact]
    public void Every_page_has_the_parts_columns_as_on_the_web()
    {
        var host = new TestHost();
        host.State("pages")!.Tab = 3;
        var (_, _, parts) = Made.Window(Tabs, host, width: 808);
        var tabs = (TabsPart)parts[0];
        Assert.Equal(12, tabs.Pages[2].Columns);
        Assert.Equal(12, tabs.Pages[0].Columns);
    }

    [Fact]
    public void The_tab_shown_is_the_one_asked_for_else_the_first_that_shows()
    {
        foreach (var c in Conformance.Read("ribbons.json").GetProperty("shown").EnumerateArray())
        {
            var visible = c.GetProperty("visible").EnumerateArray().Select(v => v.GetBoolean()).ToList();
            var shown = c.GetProperty("shown");
            long? expected = shown.ValueKind == System.Text.Json.JsonValueKind.Null ? null : shown.GetInt64();
            Assert.Equal(expected, TabsPart.ShownTab(visible, c.GetProperty("tab").GetInt64()));
        }
    }

    [Fact]
    public void A_tab_whose_parts_are_all_hidden_gives_way()
    {
        var host = new TestHost();
        var (screen, _, parts) = Made.Window(Tabs, host, width: 808);
        var tabs = (TabsPart)parts[0];
        host.State("pages")!.Tab = 2;
        host.State("second")!.IsVisible = false;
        screen.Resize(808, 400, 1);
        Assert.Equal(1, tabs.Open);
        Assert.False(tabs.Tabs[1].IsShown);
        Assert.Contains(tabs.Views[0], tabs.Children);
    }

    [Fact]
    public void Arrow_keys_move_between_tabs()
    {
        var host = new TestHost();
        var (screen, _, parts) = Made.Window(Tabs, host, width: 808);
        var tabs = (TabsPart)parts[0];
        screen.Focus(tabs.Tabs[0]);
        screen.KeyPressed(new KeyInput(Key.Right, Modifiers.None));
        Assert.Equal(2, host.State("pages")!.Tab);
        Assert.Same(tabs.Tabs[1], screen.Focused);
    }

    [Fact]
    public void A_page_whose_parts_go_further_than_its_room_scrolls()
    {
        var rows = string.Concat(Enumerable.Range(1, 8).Select(r => $"""<Label at="1,{r}" span="4" />"""));
        var (screen, _, parts) = Made.Window($"""<Tabs name="pages" at="1,1" span="12" rows="3"><Tab title="One">{rows}</Tab></Tabs>""", new TestHost(), width: 808);
        var tabs = (TabsPart)parts[0];
        // A place lower than the tabs ask for, as in a window too low for them.
        tabs.Arrange(new Box(0, 0, 808, 124));
        var view = tabs.Views[0];
        Assert.True(view.MaxScrollY > 0);
        var last = tabs.Pages[0].Children[^1];
        screen.Wheel(view.Bounds.X + 20, view.Bounds.Y + 20, 0, 1000);
        Assert.Equal(view.MaxScrollY, view.ScrollY);
        Assert.True(last.Bounds.Bottom <= view.Bounds.Bottom + 0.5);
    }

    [Fact]
    public void Tabs_ask_room_for_their_strip_so_a_page_of_their_own_rows_needs_no_scrolling()
    {
        var rows = string.Concat(Enumerable.Range(1, 3).Select(r => $"""<Label at="1,{r}" span="4" />"""));
        var (_, _, parts) = Made.Window($"""<Tabs name="pages" at="1,1" span="12" rows="3"><Tab title="One">{rows}</Tab></Tabs>""", new TestHost(), width: 808);
        var tabs = (TabsPart)parts[0];
        Assert.True(tabs.Around >= TabsPart.StripHeight);
        Assert.Equal(0, tabs.Views[0].MaxScrollY);
    }
}
