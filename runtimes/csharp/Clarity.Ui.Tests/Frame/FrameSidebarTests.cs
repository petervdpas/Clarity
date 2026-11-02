// A sidebar docked to a window's edge (conformance/sidebars.json): the
// person drags its inner edge between its min and max, folds it to a
// narrow strip with its button, and lets it fill the window; its part
// state carries whether it is folded and whether it fills the window, for
// the code. Its parts sit on its own grid and answer their keys.

using System.Text.Json;
using Clarity.Runtime;

namespace Clarity.Ui.Tests;

public class FrameSidebarTests
{
    static readonly JsonElement Cases = JsonDocument.Parse(File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "sidebars.json"))).RootElement;

    const string Nav = """
        <Sidebar side="left" name="nav" width="280" min="200" max="480" resizable="true" collapsible="true" maximizable="true" columns="4">
          <Button name="find" at="1,1" span="4" caption="Find" key="Ctrl+F" onclick="find" />
        </Sidebar>
        """;

    static double? Limit(JsonElement c, string key) => c.GetProperty(key).ValueKind == JsonValueKind.Null ? null : c.GetProperty(key).GetDouble();

    static bool? Given(JsonElement part, string key) => part.TryGetProperty(key, out var value) ? value.GetBoolean() : null;

    static SidebarSpec Spec(bool collapsed) => new("left", "nav", 240, null, null, false, true, collapsed, true, 3, []);

    [Fact]
    public void A_sidebar_is_dragged_within_its_limits_as_every_renderer_does()
    {
        foreach (var c in Cases.GetProperty("widths").EnumerateArray())
            Assert.Equal(c.GetProperty("width").GetDouble(), FrameRules.SidebarWidth(c.GetProperty("dragged").GetDouble(), Limit(c, "min"), Limit(c, "max")));
    }

    [Fact]
    public void A_sidebar_is_folded_and_fills_the_window_as_every_renderer_says()
    {
        foreach (var c in Cases.GetProperty("collapsed").EnumerateArray())
            Assert.Equal(c.GetProperty("collapsed").GetBoolean(), FrameRules.SidebarNow(Spec(c.GetProperty("design").GetBoolean()), Given(c.GetProperty("part"), "collapsed"), null).Collapsed);
        foreach (var c in Cases.GetProperty("maximized").EnumerateArray())
            Assert.Equal(c.GetProperty("maximized").GetBoolean(), FrameRules.SidebarNow(Spec(false), null, Given(c.GetProperty("part"), "maximized")).Maximized);
    }

    [Fact]
    public void Folding_and_filling_are_sent_as_its_part_state()
    {
        foreach (var c in Cases.GetProperty("set").EnumerateArray())
        {
            var (screen, view, host) = Framed.Window(Nav);
            Framed.Draw(screen);
            var nav = view.Frame.Left!;
            if (Given(c, "collapsed") is { } collapsed)
                nav.Fold(collapsed);
            if (Given(c, "maximized") is { } maximized)
                nav.Fill(maximized);
            var part = c.GetProperty("part");
            if (Given(part, "collapsed") is { } isCollapsed)
                Assert.Equal(isCollapsed, host.State("nav")!.IsCollapsed);
            if (Given(part, "maximized") is { } isMaximized)
                Assert.Equal(isMaximized, host.State("nav")!.IsMaximized);
        }
    }

    [Fact]
    public void A_sidebar_is_its_width_folds_to_a_strip_and_fills_the_window()
    {
        var (screen, view, host) = Framed.Window(Nav);
        Framed.Draw(screen);
        var nav = view.Frame.Left!;
        Assert.Equal(new Box(0, 0, 280, 600), nav.Bounds);
        Assert.Equal(new Box(280, 0, 720, 600), view.Frame.Body);
        Framed.Click(screen, nav.FoldButton!.Value);
        Assert.True(host.State("nav")!.IsCollapsed);
        Framed.Draw(screen);
        Assert.Equal(FrameRules.StripWidth, nav.Bounds.Width);
        Assert.False(nav.Parts[0].ShowsInTree);
        Framed.Click(screen, nav.FoldButton!.Value);
        Framed.Draw(screen);
        Framed.Click(screen, nav.FillButton!.Value);
        Assert.True(host.State("nav")!.IsMaximized);
        Framed.Draw(screen);
        // It takes the whole width; the window's own parts get none.
        Assert.Equal(1000, nav.Bounds.Width);
        Assert.Equal(0, view.Frame.Body.Width);
    }

    [Fact]
    public void Code_folds_it_through_its_part_state()
    {
        var (screen, view, host) = Framed.Window(Nav);
        host.State("nav")!.IsCollapsed = true;
        Framed.Draw(screen);
        Assert.Equal(FrameRules.StripWidth, view.Frame.Left!.Bounds.Width);
    }

    [Fact]
    public void An_unnamed_sidebar_starts_folded_as_designed()
    {
        var (screen, view, _) = Framed.Window("""<Sidebar side="right" width="300" collapsible="true" collapsed="true" />""");
        Framed.Draw(screen);
        var side = view.Frame.Right!;
        Assert.Equal(new Box(1000 - FrameRules.StripWidth, 0, FrameRules.StripWidth, 600), side.Bounds);
        Framed.Click(screen, side.FoldButton!.Value);
        Framed.Draw(screen);
        Assert.Equal(300, side.Bounds.Width);
    }

    [Fact]
    public void The_person_drags_its_edge_within_its_limits()
    {
        var (screen, view, _) = Framed.Window(Nav);
        Framed.Draw(screen);
        var nav = view.Frame.Left!;
        var edge = nav.Edge!.Value;
        var x = edge.X + edge.Width / 2;
        screen.PointerPressed(x, 300, PointerButton.Left);
        screen.PointerMoved(x + 60, 300);
        Framed.Draw(screen);
        Assert.Equal(340, nav.Bounds.Width);
        screen.PointerMoved(x + 900, 300);
        screen.PointerReleased(x + 900, 300, PointerButton.Left);
        Framed.Draw(screen);
        Assert.Equal(480, nav.Bounds.Width);
        Assert.Equal(480, view.Frame.Body.X);
    }

    [Fact]
    public void Its_parts_sit_on_its_grid_and_answer_their_keys()
    {
        var (screen, view, host) = Framed.Window(Nav);
        Framed.Draw(screen);
        var find = view.Frame.Left!.Parts[0];
        Assert.Equal(FrameSidebar.Padding, find.Bounds.X);
        Assert.Equal(280 - 2 * FrameSidebar.Padding, find.Bounds.Width);
        Framed.Click(screen, find.Bounds);
        Framed.Press(screen, Key.F, Modifiers.Control);
        Assert.Equal(["run find click find", "run find click find"], host.Log);
    }

    [Fact]
    public void A_hidden_sidebar_takes_no_room()
    {
        var (screen, view, host) = Framed.Window(Nav);
        host.State("nav")!.IsVisible = false;
        Framed.Draw(screen);
        Assert.Equal(new Box(0, 0, 1000, 600), view.Frame.Body);
        Framed.Press(screen, Key.F, Modifiers.Control);
        Assert.Empty(host.Log);
    }

    [Fact]
    public void A_sidebars_part_state_carries_whether_it_is_folded_and_fills_the_window()
    {
        var nav = new PartState("nav");
        var changed = new List<string?>();
        nav.PropertyChanged += (_, e) => changed.Add(e.PropertyName);
        nav.IsCollapsed = true;
        nav.IsMaximized = true;
        Assert.Equal(["IsCollapsed", "IsMaximized"], changed);
    }
}

public class SidebarCatalogTests
{
    [Fact]
    public void The_fill_mark_is_drawn_as_lines_its_way_for_filling_and_giving_back()
    {
        using var bitmap = new SKBitmap(40, 40);
        using var canvas = new SKCanvas(bitmap);
        var look = ThemeLook.Clarity(dark: false);
        var painter = new Painter(canvas, look, new Fonts(look));
        // To fill: from the bottom left to the top right.
        canvas.Clear(SKColors.White);
        FrameSidebar.FillMark(painter, new Box(0, 0, 40, 40), filling: false, SKColors.Black);
        Assert.NotEqual(SKColors.White, bitmap.PixelAt(29, 11));
        Assert.Equal(SKColors.White, bitmap.PixelAt(11, 11));
        // While it fills: from the top left to the bottom right.
        canvas.Clear(SKColors.White);
        FrameSidebar.FillMark(painter, new Box(0, 0, 40, 40), filling: true, SKColors.Black);
        Assert.NotEqual(SKColors.White, bitmap.PixelAt(11, 11));
        Assert.Equal(SKColors.White, bitmap.PixelAt(29, 11));
    }

    [Fact]
    public void A_sidebar_takes_what_its_markup_leaves_out_from_the_catalog()
    {
        var defaults = WindowMarkup.SidebarDefaults("right");
        var spec = FrameSpec.Read([System.Xml.Linq.XElement.Parse("""<Sidebar side="right" name="info" min="120" />""")]).Right!;
        Assert.Equal(defaults["width"]!.GetValue<double>(), spec.Width);
        Assert.Equal(defaults["columns"]!.GetValue<double>(), spec.Columns);
        Assert.Equal(120, spec.Min);
        Assert.Null(spec.Max);
        Assert.False(spec.Resizable);
        var written = FrameSpec.Read([System.Xml.Linq.XElement.Parse("""<Sidebar side="left" width="300" columns="2" resizable="true" />""")]).Left!;
        Assert.Equal((300.0, 2, true), (written.Width, written.Columns, written.Resizable));
    }
}
