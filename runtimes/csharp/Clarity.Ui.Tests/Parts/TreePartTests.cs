// A tree and a tree table (U1c), a table in tree mode, against the cases
// every stack shares (conformance/trees.json): rows under their parents,
// indented, the open ones showing theirs; a toggle opens and closes a row,
// and a row selected by code opens the rows above it.

using System.Text.Json;

namespace Clarity.Ui.Tests;

public class TreePartTests
{
    static readonly JsonElement Trees = Cases.Of("trees.json");

    static List<object> Folders(JsonElement rows) => [.. rows.EnumerateArray().Select(r => (object)new Folder
    {
        Id = r.GetProperty("id").GetInt64(),
        Name = $"Folder {r.GetProperty("id").GetInt64()}",
        Parent = r.GetProperty("parent").ValueKind == JsonValueKind.Null ? null : r.GetProperty("parent").GetInt64(),
    })];

    static (Screen Screen, TablePart Tree, DataHost Host) Made(List<object> folders, string kind = "Tree", string fields = "show=\"name\"")
    {
        var host = new DataHost();
        host.Lists["folders"] = folders;
        var (screen, _, parts) = Clarity.Ui.Tests.Made.Window($"""<{kind} at="1,1" span="12" rows="8" name="folders" bind="folders" {fields} onselect="picked" />""", host);
        return (screen, Assert.IsType<TablePart>(parts[0]), host);
    }

    [Fact]
    public void A_tree_shows_its_rows_as_every_stack_does()
    {
        foreach (var c in Trees.GetProperty("cases").EnumerateArray())
        {
            var folders = Folders(c.GetProperty("rows"));
            var (_, tree, _) = Made(folders);
            foreach (var id in c.GetProperty("open").EnumerateArray().Select(o => o.GetInt64()))
                tree.Toggle(folders.FindIndex(f => ((Folder)f).Id == id));
            var shown = tree.ShownPlaces.Select(p => $"{p},{tree.TreePlace(p)!.Value.Depth},{tree.TreePlace(p)!.Value.Children}").ToList();
            var expect = c.GetProperty("shows").EnumerateArray().Select(s => $"{s[0].GetInt32()},{s[1].GetInt32()},{s[2].GetBoolean()}").ToList();
            Assert.True(expect.SequenceEqual(shown), $"{c.GetProperty("name").GetString()}: [{string.Join(" ", shown)}]");
        }
        foreach (var c in Trees.GetProperty("reveal").EnumerateArray())
        {
            var folders = Folders(c.GetProperty("rows"));
            var (_, tree, host) = Made(folders);
            host.State("folders")!.SelectedItem = folders[c.GetProperty("at").GetInt32()];
            var opens = c.GetProperty("opens").EnumerateArray().Select(o => o.GetInt64()).ToList();
            Assert.Equal(opens, folders.Select((f, i) => (f, i)).Where(x => tree.IsOpen(x.i)).Select(x => ((Folder)x.f).Id));
        }
    }

    static List<object> Documents() =>
    [
        new Folder { Id = 1, Name = "Documents" },
        new Folder { Id = 2, Name = "Invoices", Parent = 1 },
        new Folder { Id = 3, Name = "2026", Parent = 2 },
        new Folder { Id = 4, Name = "Music" },
    ];

    [Fact]
    public void A_toggle_shows_the_rows_under_a_row_and_a_selected_row_opens_its_parents()
    {
        var (screen, tree, host) = Made(Documents());
        var folders = host.Lists["folders"];
        List<string> Shown() => [.. tree.ShownPlaces.Select(p => ((Folder)folders[p]).Name)];
        Assert.Equal(["Documents", "Music"], Shown());
        Assert.Null(tree.HeaderBox("name"));
        Assert.Equal("Expand", tree.ToggleWord(0));
        // A press on the toggle opens the row and chooses none.
        Cases.Click(screen, tree.ToggleBox(0)!.Value);
        Assert.Equal(["Documents", "Invoices", "Music"], Shown());
        Assert.Equal("Collapse", tree.ToggleWord(0));
        Assert.Empty(host.Events);
        // The rows under a row are indented.
        Assert.True(tree.ToggleBox(1)!.Value.X > tree.ToggleBox(0)!.Value.X);
        host.State("folders")!.SelectedItem = folders[2];
        Assert.Equal(["Documents", "Invoices", "2026", "Music"], Shown());
        Assert.Equal(new Clarity.Runtime.TreeShown(2, 2, false), tree.TreePlace(2));
        Cases.Click(screen, tree.ToggleBox(0)!.Value);
        Assert.Equal(["Documents", "Music"], Shown());
        // A click on a row chooses it.
        Cases.Click(screen, tree.CellBox(3, "name")!.Value);
        Assert.Equal(["select"], host.Events);
        using var pixels = Shot.Of(screen, 800, 400);
    }

    [Fact]
    public void Right_and_left_open_and_close_the_chosen_row()
    {
        var (screen, tree, host) = Made(Documents());
        host.State("folders")!.SelectedItem = host.Lists["folders"][0];
        screen.Focus(tree);
        screen.KeyPressed(new KeyInput(Key.Right, Modifiers.None));
        Assert.True(tree.IsOpen(0));
        screen.KeyPressed(new KeyInput(Key.Down, Modifiers.None));
        Assert.Equal(1, tree.ChosenPlace);
        screen.KeyPressed(new KeyInput(Key.Up, Modifiers.None));
        screen.KeyPressed(new KeyInput(Key.Left, Modifiers.None));
        Assert.False(tree.IsOpen(0));
    }

    [Fact]
    public void A_tree_table_is_a_table_in_tree_mode_with_its_header()
    {
        var (_, tree, _) = Made(Documents(), "TreeTable", "columns=\"name, id\"");
        Assert.True(tree.IsTree);
        Assert.True(tree.Grows);
        Assert.NotNull(tree.HeaderBox("name"));
        Assert.Equal(["name", "id"], tree.Columns.Select(c => c.Field));
        Assert.Equal([0, 3], tree.ShownPlaces);
        Assert.Equal("1", tree.Shown(0, "id"));
    }
}
