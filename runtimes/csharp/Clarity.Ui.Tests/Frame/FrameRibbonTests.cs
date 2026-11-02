// A ribbon (conformance/ribbons.json): its open tab, counted from 1, and
// whether it is folded follow its part state. A hidden tab gives way to the
// first visible one; folded, only the tab strip shows, and opening a tab
// unfolds it; a double click on the tabs folds it again. A button runs its
// handler, and its key runs it whichever tab is open, unless code hid or
// disabled it.

using System.Text.Json;
using Clarity.Runtime;

namespace Clarity.Ui.Tests;

public class FrameRibbonTests
{
    static readonly JsonElement Cases = JsonDocument.Parse(File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "ribbons.json"))).RootElement;

    const string Ribbon = """
        <Ribbon name="mainRibbon" collapsible="true">
          <Tab title="Home" name="home">
            <Group title="Clipboard" name="clip">
              <Item name="paste" caption="Paste" large="true" icon="clipboard" key="Ctrl+V" onclick="paste" />
              <Item caption="Cut" onclick="cut" />
            </Group>
            <Group title="Save" name="saving">
              <Item name="save" caption="Save" key="Ctrl+S" onclick="save" />
            </Group>
          </Tab>
          <Tab title="Insert" name="insert">
            <Group title="Pictures"><Item caption="Picture" key="Ctrl+P" onclick="picture" /></Group>
          </Tab>
        </Ribbon>
        """;

    static (Screen Screen, WindowView View, TestHost Host, FrameRibbon Ribbon) Shown()
    {
        var (screen, view, host) = Framed.Window(Ribbon);
        Framed.Draw(screen);
        return (screen, view, host, view.Frame.Ribbon!);
    }

    [Fact]
    public void A_ribbon_shows_the_tab_as_every_renderer_does()
    {
        foreach (var c in Cases.GetProperty("shown").EnumerateArray())
        {
            var visible = c.GetProperty("visible").EnumerateArray().Select(v => v.GetBoolean()).ToArray();
            var shown = c.GetProperty("shown");
            Assert.Equal(shown.ValueKind == JsonValueKind.Null ? null : shown.GetInt32(), FrameRules.ShownTab(visible, c.GetProperty("tab").GetInt64()));
        }
    }

    [Fact]
    public void Its_part_state_carries_the_open_tab()
    {
        foreach (var c in Cases.GetProperty("tab").EnumerateArray())
        {
            var state = new PartState("mainRibbon");
            if (c.GetProperty("part").TryGetProperty("tab", out var tab))
                state.Tab = tab.GetInt64();
            Assert.Equal(c.GetProperty("tab").GetInt64(), state.Tab);
        }
        foreach (var c in Cases.GetProperty("set").EnumerateArray())
        {
            var state = new PartState("mainRibbon") { Tab = c.GetProperty("tab").GetInt64() };
            Assert.Equal(c.GetProperty("part").GetProperty("tab").GetInt64(), state.Tab);
        }
        // Opening a tab sends it as the part's tab.
        var (screen, _, host, ribbon) = Shown();
        Framed.Click(screen, ribbon.Tabs[1].Box);
        Assert.Equal(2, host.State("mainRibbon")!.Tab);
    }

    [Fact]
    public void A_tab_opens_its_groups_and_code_opens_another()
    {
        var (screen, _, host, ribbon) = Shown();
        Assert.Equal(1, ribbon.ShownTab);
        Assert.Equal(["Paste", "Cut", "Save"], ribbon.Buttons.Select(b => b.Item.Text("caption")));
        // A large button is higher, its icon above its caption.
        Assert.True(ribbon.Buttons[0].Box.Height > ribbon.Buttons[1].Box.Height);
        host.State("mainRibbon")!.Tab = 2;
        Framed.Draw(screen);
        Assert.Equal(["Picture"], ribbon.Buttons.Select(b => b.Item.Text("caption")));
    }

    [Fact]
    public void A_hidden_tab_gives_way_and_a_hidden_group_leaves()
    {
        var (screen, _, host, ribbon) = Shown();
        host.State("saving")!.IsVisible = false;
        Framed.Draw(screen);
        Assert.Equal(["Paste", "Cut"], ribbon.Buttons.Select(b => b.Item.Text("caption")));
        host.State("home")!.IsVisible = false;
        Framed.Draw(screen);
        Assert.Equal(2, ribbon.ShownTab);
        Assert.Single(ribbon.Tabs);
    }

    [Fact]
    public void Folded_only_the_tabs_show_and_opening_a_tab_unfolds_it()
    {
        var (screen, view, host, ribbon) = Shown();
        var unfolded = view.Frame.Body.Y;
        Framed.Click(screen, ribbon.Tabs[0].Box, clicks: 2);
        Assert.True(host.State("mainRibbon")!.IsCollapsed);
        Framed.Draw(screen);
        Assert.Equal(FrameRibbon.TabsHeight, ribbon.Bounds.Height);
        Assert.Empty(ribbon.Buttons);
        Assert.True(view.Frame.Body.Y < unfolded);
        Framed.Click(screen, ribbon.Tabs[1].Box);
        Assert.Equal((false, 2L), (host.State("mainRibbon")!.IsCollapsed, host.State("mainRibbon")!.Tab));
    }

    [Fact]
    public void A_ribbon_that_does_not_fold_stays_open()
    {
        var (screen, view, host) = Framed.Window("""<Ribbon><Tab title="Home"><Group title="G"><Item caption="A" onclick="a" /></Group></Tab></Ribbon>""");
        Framed.Draw(screen);
        var ribbon = view.Frame.Ribbon!;
        Framed.Click(screen, ribbon.Tabs[0].Box, clicks: 2);
        Framed.Draw(screen);
        Assert.False(ribbon.Collapsed);
        Framed.Click(screen, ribbon.Buttons[0].Box);
        Assert.Equal(["run  click a"], host.Log);
    }

    [Fact]
    public void A_button_runs_its_handler()
    {
        var (screen, _, host, ribbon) = Shown();
        Framed.Click(screen, ribbon.Buttons[0].Box);
        Assert.Equal(["run paste click paste"], host.Log);
    }

    [Fact]
    public void A_buttons_key_runs_it_on_any_tab()
    {
        var (screen, _, host, _) = Shown();
        Framed.Press(screen, Key.S, Modifiers.Control);
        // On the Insert tab, which is not open.
        Framed.Press(screen, Key.P, Modifiers.Control);
        Assert.Equal(["run save click save", "run  click picture"], host.Log);
    }

    [Fact]
    public void What_code_hid_or_disabled_does_not_answer()
    {
        var (screen, _, host, ribbon) = Shown();
        host.State("save")!.IsEnabled = false;
        host.State("insert")!.IsVisible = false;
        host.State("paste")!.IsEnabled = false;
        Framed.Draw(screen);
        Framed.Press(screen, Key.S, Modifiers.Control);
        Framed.Press(screen, Key.P, Modifiers.Control);
        Framed.Press(screen, Key.V, Modifiers.Control);
        Framed.Click(screen, ribbon.Buttons[0].Box);
        Assert.Empty(host.Log);
    }
}
