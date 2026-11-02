// A window's markup read as the shared screen reads it (DESIGN 7.8): each
// part its kind's defaults with what the markup changes, its handlers as
// written calls, its pages and menu; a kind Clarity.UI does not draw yet
// says so in its place.

namespace Clarity.Ui.Tests;

public class MarkupTests
{
    const string Window = """
        <?xml version="1.0" encoding="UTF-8"?>
        <Window name="Main">
          <Menu name="mainMenu"><Item caption="Open" name="openItem" key="Ctrl+O" opens="Orders" /></Menu>
          <Button at="3,2" span="4" caption="@saveCaption" name="save" key="Ctrl+S" onclick="digit(7, &quot;a, b&quot;)" />
          <Textbox at="1,1" span="6" rows="2" name="note" bind="order.note" none="placeholder" onchange="noteChanged" />
          <Tabs at="1,4" span="12" rows="4" name="pages">
            <Tab title="One"><Label at="1,1" span="2" caption="Hi" /></Tab>
            <Tab title="Two" span="6" />
          </Tabs>
          <Table at="1,9" span="12" name="grid" bind="orders" columns="number, status" />
        </Window>
        """;

    [Fact]
    public void A_part_is_its_kinds_defaults_with_what_the_markup_changes()
    {
        var spec = WindowMarkup.Read(Window);
        Assert.Equal(["Menu"], spec.Frame.Select(e => e.Name.LocalName));
        var save = spec.Parts[0];
        Assert.Equal("button", save.Kind);
        Assert.Equal("save", save.Name);
        Assert.Equal(new GridItem(3, 2, 4, 1, false), save.Place(false));
        Assert.Equal("@saveCaption", save.Text("caption"));
        Assert.Equal("Ctrl+S", save.Text("shortcut"));
        var call = Assert.Single(save.On("click"));
        Assert.Equal(("digit", 2), (call.Handler, call.Args.Count));
        Assert.Equal("\"a, b\"", call.Args[1]);
        var note = spec.Parts[1];
        Assert.Equal(("textbox", 2, "order.note"), (note.Kind, note.Place(false).Height, note.Text("bind")));
        Assert.Null(note.Text("placeholder"));
        Assert.Equal("noteChanged", Assert.Single(note.On("change")).Written);
        var tabs = spec.Parts[2];
        Assert.Equal(["One", "Two"], tabs.Pages.Select(p => p.Title));
        Assert.Equal((int?)6, tabs.Pages[1].Columns);
        Assert.Equal("label", Assert.Single(tabs.Pages[0].Children).Kind);
        Assert.Equal(["number", "status"], spec.Parts[3].Names("columns"));
    }

    [Fact]
    public void A_menu_item_reads_its_own_words()
    {
        var item = WindowMarkup.MenuOf(WindowMarkup.Read(Window).Frame[0]).Items[0];
        Assert.Equal(("Open", "Ctrl+O", "Orders"), (item.Text("caption"), item.Text("shortcut"), item.Text("onClick")));
    }

    [Fact]
    public void Every_kind_of_the_catalog_reads()
    {
        foreach (var kind in WindowMarkup.Kinds)
        {
            var tag = char.ToUpperInvariant(kind[0]) + kind[1..];
            Assert.Equal(kind, WindowMarkup.Read($"<Window><{tag} at=\"1,1\" /></Window>").Parts[0].Kind);
        }
    }

    [Fact]
    public void A_markup_with_an_unknown_part_is_refused()
    {
        Assert.Throws<FormatException>(() => WindowMarkup.Read("<Window><Gizmo /></Window>"));
    }

    [Fact]
    public void A_kind_not_drawn_yet_says_so_in_its_place()
    {
        var host = new TestHost();
        // A kind no class draws: made from its spec, as the markup would give it.
        var spec = new PartSpec("notDrawnYet", new System.Text.Json.Nodes.JsonObject { ["col"] = 1, ["row"] = 1, ["width"] = 4, ["height"] = 1 }, [], [], [], null);
        var missing = Assert.IsType<MissingPart>(PartKinds.Make(spec, host));
        Assert.Null(missing.Why);
        var grid = new GridPanel(columns: 12);
        grid.Add(missing, spec.Place(false));
        var screen = new Screen(grid, ThemeLook.Clarity(dark: false));
        using var pixels = Shot.Of(screen, 800, 400);
        Assert.NotEqual(screen.Look.Color("background"), pixels.PixelAt(0, 18));
    }

    [Fact]
    public void A_part_that_code_hides_takes_no_pointer()
    {
        var host = new TestHost();
        var (screen, _, parts) = Made.Window("""<Label at="1,1" span="12" name="cpu" caption="CPU" />""", host);
        Assert.Same(parts[0], screen.At(20, 10));
        host.State("cpu")!.IsVisible = false;
        Assert.NotSame(parts[0], screen.At(20, 10));
    }
}
