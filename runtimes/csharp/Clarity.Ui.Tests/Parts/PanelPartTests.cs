// A panel and a form: a frame with its caption in the theme's header, its
// parts in a grid of its own as wide as it spans; a form without a caption
// has no frame; a foldable one folds to its header (conformance/folding.json);
// it grows when a part in it does, and tells a list in it that it is inside.

using System.Text.Json;

namespace Clarity.Ui.Tests.Containers;

static class Conformance
{
    public static JsonElement Read(string name) =>
        JsonDocument.Parse(File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", name))).RootElement;
}

public class PanelPartTests
{
    const string Panel = """
        <Panel name="box" at="7,1" span="6" rows="4" caption="Customer">
          <Label name="inner" at="4,1" span="3" />
        </Panel>
        """;

    [Fact]
    public void A_panels_parts_are_in_a_grid_as_wide_as_it_spans_under_its_header()
    {
        var (_, _, parts) = Made.Window(Panel, new TestHost(), width: 808);
        var panel = Assert.IsType<PanelPart>(parts[0]);
        // Twelve columns of 60 with 8 between: the panel is the right half.
        Assert.Equal(new Box(408, 0, 400, 168), panel.Bounds);
        var inner = Assert.IsAssignableFrom<Part>(Assert.Single(panel.Inner.Children));
        // Its own six columns are the window's: the part at its fourth starts at the window's tenth.
        Assert.Equal(612, inner.Bounds.X);
        Assert.Equal(196, inner.Bounds.Width);
        Assert.True(inner.Bounds.Y > panel.Bounds.Y + 20);
    }

    [Fact]
    public void A_panel_shows_its_caption_in_the_themes_header()
    {
        var (screen, _, parts) = Made.Window(Panel, new TestHost(), width: 808);
        using var pixels = Shot.Of(screen, 808, 400);
        var header = screen.Look.Part("header", null).Background!.Value;
        Assert.Equal(header, pixels.GetPixel(800, 6));
        // Its frame, in the strong border, down its side.
        Assert.Equal(screen.Look.Color("strong_border"), pixels.GetPixel(405, 100));
    }

    [Fact]
    public void A_form_without_a_caption_has_no_frame_and_its_grid_lines_up_with_the_windows()
    {
        var (_, _, parts) = Made.Window("""<Form at="1,2" span="12" rows="3"><Label at="1,1" span="2" /></Form>""", new TestHost(), width: 808);
        var form = Assert.IsType<PanelPart>(parts[0]);
        Assert.True(form.Bare);
        Assert.Equal(form.Bounds.Y, form.Inner.Children[0].Bounds.Y);
        Assert.Equal(new Box(0, 44, 128, 36), form.Inner.Children[0].Bounds);
    }

    [Fact]
    public void A_container_grows_when_a_part_in_it_does()
    {
        var (_, _, parts) = Made.Window("""
            <Panel at="1,1" span="6" rows="3"><Image at="1,1" span="3" rows="2" /></Panel>
            <Panel at="7,1" span="6" rows="3"><Label at="1,1" span="3" /></Panel>
            <Tabs at="1,4" span="6" rows="3"><Tab title="A" /><Tab title="B"><Image at="1,1" /></Tab></Tabs>
            """, new TestHost());
        Assert.True(parts[0].Grows);
        Assert.False(parts[1].Grows);
        Assert.True(parts[2].Grows);
    }

    [Fact]
    public void A_part_in_a_container_is_inside_it_and_one_on_the_window_is_not()
    {
        var (_, _, parts) = Made.Window(Panel + """<Label at="1,6" span="3" />""", new TestHost(), width: 808);
        var panel = (PanelPart)parts[0];
        Assert.True(ContainerPart.IsInside(panel.Inner.Children[0]));
        Assert.False(ContainerPart.IsInside(parts[1]));
        Assert.True(panel.FramesParts);
    }

    [Fact]
    public void A_foldable_panel_folds_to_its_header_when_the_person_presses_it()
    {
        var host = new TestHost();
        var (screen, _, parts) = Made.Window(Panel.Replace("caption=", "foldable=\"true\" caption="), host, width: 808);
        var panel = (PanelPart)parts[0];
        var inner = panel.Inner.Children[0];
        Assert.Same(inner, screen.At(620, inner.Bounds.Y + 5));
        screen.PointerPressed(500, 8, PointerButton.Left);
        screen.PointerReleased(500, 8, PointerButton.Left);
        Assert.True(host.State("box")!.IsFolded);
        Assert.True(panel.IsFolded);
        screen.Resize(808, 400, 1);
        // Folded, its parts take no pointer and its place below the header is the window's.
        Assert.NotSame(inner, screen.At(620, inner.Bounds.Y + 5));
        Assert.NotSame(panel, screen.At(620, 100));
        Assert.True(panel.Bounds.Height < 40);
        // A handler unfolds it by its state.
        host.State("box")!.IsFolded = false;
        screen.Resize(808, 400, 1);
        Assert.Same(inner, screen.At(620, inner.Bounds.Y + 5));
        // A press in its body is no press on its header.
        screen.PointerPressed(450, 150, PointerButton.Left);
        screen.PointerReleased(450, 150, PointerButton.Left);
        Assert.False(host.State("box")!.IsFolded);
    }

    [Fact]
    public void A_foldable_part_is_folded_as_its_state_says_else_as_designed()
    {
        foreach (var c in Conformance.Read("folding.json").GetProperty("cases").EnumerateArray())
        {
            var host = new TestHost();
            var design = c.GetProperty("design").GetBoolean() ? " folded=\"true\"" : "";
            var named = c.GetProperty("part").TryGetProperty("folded", out var folded);
            var markup = $"""<Panel{(named ? " name=\"box\"" : "")} at="1,1" span="6" rows="3" foldable="true"{design} />""";
            if (named)
                host.State("box")!.IsFolded = folded.GetBoolean();
            var (_, _, parts) = Made.Window(markup, host);
            Assert.Equal(c.GetProperty("folded").GetBoolean(), ((PanelPart)parts[0]).IsFolded);
        }
    }

    [Fact]
    public void A_panel_code_hides_draws_nothing()
    {
        var host = new TestHost();
        var (screen, _, _) = Made.Window(Panel, host, width: 808);
        host.State("box")!.IsVisible = false;
        using var pixels = Shot.Of(screen, 808, 400);
        Assert.Equal(screen.Look.Color("background"), pixels.GetPixel(800, 6));
    }
}
