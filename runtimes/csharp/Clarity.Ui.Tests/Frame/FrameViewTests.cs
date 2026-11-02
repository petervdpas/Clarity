// The frame around a window (runtimes/web/src/frame/FrameView.svelte): its
// regions read from the window's markup and its app's, each of the window's
// own replacing the app's, placed around the window's parts; the window's
// own buttons hear a key before the frame's commands do.

using System.Xml.Linq;

namespace Clarity.Ui.Tests;

/// <summary>A window with its frame on a screen of its own, as the frame's tests give it.</summary>
static class Framed
{
    public static (Screen Screen, WindowView View, TestHost Host) Window(string own, string? app = null, TestHost? host = null, double width = 1000, double height = 600, string name = "Main")
    {
        host ??= new TestHost();
        var spec = WindowMarkup.Read($"<Window name=\"{name}\">{own}</Window>");
        var view = new WindowView(spec, host);
        if (app is not null)
            view.AppFrame = WindowMarkup.ReadApp($"<App name=\"Shop\">{app}</App>");
        var screen = new Screen(view, ThemeLook.Clarity(dark: false)) { Keys = view.KeyPressed };
        screen.Resize(width, height, 1);
        return (screen, view, host);
    }

    public static void Click(Screen screen, Box box, int clicks = 1)
    {
        var (x, y) = (box.X + box.Width / 2, box.Y + box.Height / 2);
        screen.PointerPressed(x, y, PointerButton.Left, clicks);
        screen.PointerReleased(x, y, PointerButton.Left);
    }

    public static void Press(Screen screen, Key key, Modifiers held = Modifiers.None) => screen.KeyPressed(new KeyInput(key, held));

    /// <summary>Draws the screen, as the host does, so every region is placed and drawn.</summary>
    public static SKBitmap Draw(Screen screen) => Shot.Of(screen, (int)screen.Width, (int)screen.Height);
}

public class FrameViewTests
{
    const string App = """
        <Menu name="appMenu"><Item caption="App" onclick="fromApp" /></Menu>
        <Toolbar name="appTools"><Item caption="Print" onclick="print" /></Toolbar>
        <Footer status="Ready" columns="12" />
        """;

    [Fact]
    public void A_frame_is_read_from_its_regions()
    {
        var frame = FrameSpec.Read(XElement.Parse("""
            <App name="Shop">
              <Menu name="mainMenu"><Item caption="File"><Item caption="Open" key="Ctrl+O" onclick="open" /><Item kind="separator" /></Item></Menu>
              <Toolbar name="tools"><Item caption="Save" icon="save" onclick="save" /></Toolbar>
              <Ribbon name="mainRibbon" collapsible="true" collapsed="true">
                <Tab title="Home" name="home"><Group title="Clipboard" name="clip"><Item caption="Paste" large="true" onclick="paste" /></Group></Tab>
              </Ribbon>
              <Sidebar side="right" name="nav" width="280" min="200" max="480" resizable="true" collapsible="true" maximizable="true" columns="4">
                <Label at="1,1" span="4" caption="Filters" />
              </Sidebar>
              <Footer status="@ready" columns="6" />
            </App>
            """).Elements());
        Assert.Equal("mainMenu", frame.Menu!.Name);
        Assert.Equal("Ctrl+O", frame.Menu.Items[0].Items[0].Text("shortcut"));
        Assert.True(FrameRules.IsSeparator(frame.Menu.Items[0].Items[1]));
        Assert.Equal("save", frame.Toolbar!.Items[0].Text("icon"));
        Assert.Equal((true, true, "Home", "Clipboard"), (frame.Ribbon!.Collapsible, frame.Ribbon.Collapsed, frame.Ribbon.Tabs[0].Title, frame.Ribbon.Tabs[0].Groups[0].Title));
        Assert.True(frame.Ribbon.Tabs[0].Groups[0].Items[0].Flag("large"));
        Assert.Null(frame.Left);
        var nav = frame.Right!;
        Assert.Equal(("nav", 280.0, 200.0, 480.0, 4), (nav.Name, nav.Width, nav.Min, nav.Max, nav.Columns));
        Assert.True(nav.Resizable && nav.Collapsible && nav.Maximizable && !nav.Collapsed);
        Assert.Equal("label", nav.Parts[0].Kind);
        Assert.Equal(("@ready", 6), (frame.Footer!.Status, frame.Footer.Columns));
    }

    [Fact]
    public void A_sidebar_without_its_settings_is_a_new_sidebars()
    {
        var frame = FrameSpec.Read(XElement.Parse("""<Window><Sidebar side="left" /></Window>""").Elements());
        Assert.Equal((240.0, 3, (double?)null), (frame.Left!.Width, frame.Left.Columns, frame.Left.Min));
    }

    [Fact]
    public void A_windows_own_region_replaces_the_apps()
    {
        var app = FrameSpec.Read(XElement.Parse($"<App>{App}</App>").Elements());
        var own = FrameSpec.Read(XElement.Parse("""<Window><Menu name="mine"><Item caption="Mine" /></Menu></Window>""").Elements());
        var shown = ShownFrame.Of(app, own);
        Assert.Equal(("mine", false), (shown.Menu!.Value.Name, shown.Menu.Inherited));
        Assert.Equal(("appTools", true), (shown.Toolbar!.Value.Name, shown.Toolbar.Inherited));
        Assert.True(shown.Footer!.Inherited);
        Assert.Null(shown.Ribbon);
        // A dialog shows no app frame.
        Assert.Equal("mine", ShownFrame.Of(null, own).Menu!.Value.Name);
        Assert.Null(ShownFrame.Of(null, own).Toolbar);
    }

    [Fact]
    public void The_windows_parts_sit_between_the_regions()
    {
        var (screen, view, _) = Framed.Window("""<Button name="save" at="1,1" span="3" caption="Save" />""", App);
        Framed.Draw(screen);
        var frame = view.Frame;
        Assert.Equal(new Box(0, 0, 1000, FrameMenuBar.Height), frame.MenuBar!.Bounds);
        Assert.Equal(new Box(0, FrameMenuBar.Height, 1000, FrameToolbar.Height), frame.Toolbar!.Bounds);
        Assert.Equal(600 - FrameFooter.StatusHeight, frame.Footer!.Bounds.Y);
        Assert.Equal(new Box(0, FrameMenuBar.Height + FrameToolbar.Height, 1000, 600 - FrameMenuBar.Height - FrameToolbar.Height - FrameFooter.StatusHeight), frame.Body);
        // The window's padding is inside the frame.
        Assert.Equal(frame.Body.Y + WindowView.ContentPadding, view.Parts[0].Bounds.Y);
    }

    [Fact]
    public void A_window_without_a_frame_keeps_its_whole_place()
    {
        var (screen, view, _) = Framed.Window("""<Button name="save" at="1,1" span="3" caption="Save" />""");
        Framed.Draw(screen);
        Assert.Empty(view.Frame.Regions);
        Assert.Equal(new Box(0, 0, 1000, 600), view.Frame.Body);
        Assert.Equal(WindowView.ContentPadding, view.Parts[0].Bounds.Y);
    }

    [Fact]
    public void The_windows_own_button_hears_its_key_before_the_frame()
    {
        var (screen, _, host) = Framed.Window(
            """<Button name="mine" at="1,1" span="3" caption="Mine" key="Ctrl+N" onclick="windowNew" />""",
            """<Toolbar><Item caption="New" key="Ctrl+N" onclick="frameNew" /></Toolbar>""");
        Framed.Press(screen, Key.N, Modifiers.Control);
        Assert.Equal(["run mine click windowNew"], host.Log);
    }

    [Fact]
    public void A_frame_draws_in_light_and_dark()
    {
        var (screen, _, host) = Framed.Window(
            """<Sidebar side="left" name="nav" collapsible="true" maximizable="true"><Label at="1,1" span="3" caption="Nav" /></Sidebar>""",
            App + """<Ribbon name="r"><Tab title="Home"><Group title="Edit"><Item caption="Cut" large="true" icon="scissors" onclick="cut" /></Group></Tab></Ribbon>""");
        host.Icons["scissors"] = "M6 6l12 12";
        using var light = Framed.Draw(screen);
        Assert.Equal(screen.Look.Color("frame"), light.GetPixel(2, 2));
        screen.Show(ThemeLook.Clarity(dark: true));
        using var dark = Framed.Draw(screen);
        Assert.Equal(screen.Look.Color("frame"), dark.GetPixel(2, 2));
    }

    [Fact]
    public void The_app_frame_is_given_after_the_window_is_made()
    {
        var (screen, view, host) = Framed.Window("");
        Assert.Empty(view.Frame.Regions);
        view.AppFrame = WindowMarkup.ReadApp($"<App name=\"Shop\">{App}</App>");
        Framed.Draw(screen);
        Assert.Equal(3, view.Frame.Regions.Count);
        Framed.Click(screen, view.Frame.Toolbar!.Placed[0].Box);
        Assert.Equal(["run  click print"], host.Log);
    }
}
