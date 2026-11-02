// What a table's rows do under the pointer, beside its cases: a double
// click on a row opens it (not on the header), a line runs under every row,
// a press on a toggle or an input starts no drag, a row moved off its page
// shows on its new page, and a list the host keeps as a plain list shows its
// new order. And an edited color cell offers the palette the color part has.

using Clarity.Runtime;

namespace Clarity.Ui.Tests;

public class TableRowTests
{
    static (Screen Screen, TablePart Table, DataHost Host) Folders(string markup, int count = 3)
    {
        var host = new DataHost();
        host.Lists["folders"] = [.. Enumerable.Range(1, count).Select(i => (object)new Folder { Id = i, Name = $"Folder {i}", Position = i })];
        var (screen, _, parts) = Made.Window(markup, host);
        return (screen, Assert.IsType<TablePart>(parts[0]), host);
    }

    [Fact]
    public void A_double_click_on_a_row_opens_it_and_on_the_header_does_not()
    {
        var (screen, table, host) = Folders("""<Table at="1,1" span="12" rows="6" name="folders" bind="folders" columns="name, id" onopen="opened" onselect="picked" />""");
        Cases.Click(screen, table.HeaderBox("name")!.Value);
        Cases.Click(screen, table.HeaderBox("name")!.Value, 2);
        Assert.Empty(host.Events);
        var cell = table.CellBox(1, "name")!.Value;
        Cases.Click(screen, cell);
        Cases.Click(screen, cell, 2);
        Assert.Equal(["select", "open"], host.Events);
    }

    [Fact]
    public void A_right_click_chooses_the_row_under_the_pointer()
    {
        var (screen, table, host) = Folders("""<Table at="1,1" span="12" rows="6" name="folders" bind="folders" columns="name, id" onselect="picked" />""");
        var cell = table.CellBox(1, "name")!.Value;
        screen.PointerPressed(cell.X + cell.Width / 2, cell.Y + cell.Height / 2, PointerButton.Right);
        screen.PointerReleased(cell.X + cell.Width / 2, cell.Y + cell.Height / 2, PointerButton.Right);
        Assert.Same(host.Lists["folders"][1], table.Chosen);
        Assert.Equal(["select"], host.Events);
    }

    [Fact]
    public void A_line_runs_under_every_row()
    {
        var (screen, table, _) = Folders("""<Table at="1,1" span="12" rows="6" name="folders" bind="folders" columns="name, id" />""");
        using var pixels = Shot.Of(screen, 800, 400);
        foreach (var place in new[] { 0, 1, 2 })
        {
            var row = table.RowBox(place)!.Value;
            Assert.Equal(screen.Look.Color("border"), pixels.PixelAt((int)(row.X + row.Width / 2), (int)Math.Ceiling(row.Bottom - screen.Look.Border)));
            Assert.NotEqual(screen.Look.Color("border"), pixels.PixelAt((int)(row.X + row.Width / 2), (int)(row.Y + 3)));
        }
    }

    [Fact]
    public void A_press_on_a_toggle_or_an_input_starts_no_drag()
    {
        var host = new DataHost();
        host.Lists["folders"] = [new Folder { Id = 1, Name = "Documents", Position = 1 }, new Folder { Id = 2, Name = "Invoices", Parent = 1, Position = 2 }, new Folder { Id = 3, Name = "Music", Position = 3 }];
        var (screen, _, parts) = Made.Window("""<TreeTable at="1,1" span="12" rows="8" name="folders" bind="folders" columns="name, position" edit="position" reorder="position" />""", host);
        var tree = Assert.IsType<TablePart>(parts[0]);
        var before = host.Lists["folders"].ToList();
        void DragFrom(Box from)
        {
            var onto = tree.RowBox(2)!.Value;
            screen.PointerPressed(from.X + from.Width / 2, from.Y + from.Height / 2, PointerButton.Left);
            screen.PointerMoved(from.X + from.Width / 2, from.Y + from.Height / 2 + 10);
            screen.PointerMoved(onto.X + 300, onto.Bottom - 3);
            Assert.Null(tree.Drop);
            screen.PointerReleased(onto.X + 300, onto.Bottom - 3, PointerButton.Left);
        }
        DragFrom(tree.ToggleBox(0)!.Value);
        tree.Toggle(0);
        // The number typed into is no row to drag either.
        DragFrom(tree.CellBox(0, "position")!.Value);
        Assert.Equal(before, host.Lists["folders"]);
        Assert.DoesNotContain(host.Log, l => l.Contains("reorder"));
    }

    [Fact]
    public void On_a_paged_table_a_row_moved_off_its_page_shows_on_the_next()
    {
        var (screen, table, host) = Folders("""<Table at="1,1" span="12" rows="6" name="folders" bind="folders" columns="name" page="2" reorder="position" />""");
        var folders = host.Lists["folders"].ToList();
        host.State("folders")!.SelectedItem = folders[1];
        screen.Focus(table);
        screen.KeyPressed(new KeyInput(Key.Down, Modifiers.Alt));
        Assert.Equal(["Folder 1:1", "Folder 3:2", "Folder 2:3"], host.Lists["folders"].Cast<Folder>().Select(f => $"{f.Name}:{f.Position}"));
        Assert.Equal(2, table.Paged!.Page);
        Assert.Same(folders[1], table.Chosen);
        Assert.NotNull(table.RowBox(table.ChosenPlace));
    }

    /// <summary>A host that keeps a list as a plain list, changed in place, and tells nobody it changed.</summary>
    sealed class PlainHost : DataHost
    {
        public override void Write(string part, string path, object? value)
        {
            if (value is IEnumerable<object> items && Lists.TryGetValue(path, out var list))
            {
                var now = items.ToList();
                list.Clear();
                list.AddRange(now);
                return;
            }
            base.Write(part, path, value);
        }
    }

    [Fact]
    public void A_list_of_a_plain_list_shows_its_new_order()
    {
        var host = new PlainHost();
        host.Lists["folders"] = [.. Enumerable.Range(1, 3).Select(i => (object)new Folder { Id = i, Name = $"Folder {i}", Position = i })];
        var (screen, _, parts) = Made.Window("""<List at="1,1" span="12" rows="8" name="folders" bind="folders" show="name" reorder="position" />""", host);
        var list = Assert.IsType<ListPart>(parts[0]);
        host.State("folders")!.SelectedItem = host.Lists["folders"][0];
        screen.Focus(list);
        screen.KeyPressed(new KeyInput(Key.Down, Modifiers.Alt));
        Assert.Equal(["Folder 2", "Folder 1", "Folder 3"], list.Records.Cast<Folder>().Select(f => f.Name));
        Assert.Equal(1, list.ChosenPlace);
    }

    [Fact]
    public void An_edited_color_cell_offers_the_palette_of_the_color_part()
    {
        var host = new DataHost();
        host.Lists["people"] = [new Dictionary<string, object?> { ["name"] = "Ada", ["color"] = "#ff0000" }];
        host.Fields["people.color"] = new FieldFacts("Color", Color: true);
        var (screen, _, parts) = Made.Window("""<Table at="1,1" span="12" rows="6" name="people" bind="people" columns="name, color" edit="color" onselect="picked" onchange="changed" />""", host);
        var table = Assert.IsType<TablePart>(parts[0]);
        Cases.Click(screen, table.SwatchBox(0, "color")!.Value);
        var palette = Assert.IsType<PaletteGrid>(Assert.Single(screen.Overlays).Content);
        Assert.Equal("#ff0000", palette.Chosen);
        Assert.Null(table.CellEditor);
        var color = palette.Colors[3];
        Cases.Click(screen, palette.Place(color));
        Assert.Empty(screen.Overlays);
        Assert.Equal(color, ((Dictionary<string, object?>)host.Lists["people"][0])["color"]);
        Assert.Equal(["select", "change"], host.Events);
        // Alt+Down in its typed text opens the palette too.
        Cases.Click(screen, table.CellBox(0, "color")!.Value);
        Assert.NotNull(table.CellEditor);
        screen.KeyPressed(new KeyInput(Key.Down, Modifiers.Alt));
        Assert.IsType<PaletteGrid>(Assert.Single(screen.Overlays).Content);
    }
}
