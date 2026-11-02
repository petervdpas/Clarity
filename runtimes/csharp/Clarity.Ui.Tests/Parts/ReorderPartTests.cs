// Moving rows by hand (DESIGN 5.30) against the cases every stack shares
// (conformance/reorder.json), through the parts as the person does it: a
// row dragged onto another with the pointer, the chosen row moved with Alt
// and an arrow key. The list takes its new order and each record its number
// (and in a tree its parent) as one edit, the moved row is chosen and
// `reorder` runs; a move that changes nothing does nothing.

using System.Text.Json;

namespace Clarity.Ui.Tests;

public class ReorderPartTests
{
    static readonly JsonElement File = Cases.Of("reorder.json");

    static List<object> Folders(JsonElement rows) => [.. rows.EnumerateArray().Select((r, i) => (object)new Folder
    {
        Id = r.GetProperty("id").GetInt64(),
        Name = $"Folder {r.GetProperty("id").GetInt64()}",
        Parent = r.TryGetProperty("parent", out var p) && p.ValueKind != JsonValueKind.Null ? p.GetInt64() : null,
        Position = i + 1,
    })];

    static void Run(JsonElement @case, bool tree)
    {
        var name = @case.GetProperty("name").GetString();
        var move = @case.GetProperty("move");
        // The pointer drops a row of a list before or after another, never into it.
        if (!tree && move.TryGetProperty("where", out var w) && w.GetString() == "into")
            return;
        var host = new DataHost();
        var folders = Folders(@case.GetProperty("rows"));
        host.Lists["folders"] = [.. folders];
        var markup = tree
            ? """<TreeTable at="1,1" span="12" rows="10" name="folders" bind="folders" columns="name" reorder="position" onreorder="moved" onselect="picked" />"""
            : """<List at="1,1" span="12" rows="10" name="folders" bind="folders" show="name" reorder="position" onreorder="moved" onselect="picked" />""";
        var (screen, _, parts) = Made.Window(markup, host, 800, 600);
        var part = (RowsPart)parts[0];
        Box? RowBox(int place) => part is TablePart t ? t.RowBox(place) : ((ListPart)part).RowBox(place);
        if (part is TablePart table)
            for (var i = 0; i < folders.Count; i++)
                if (!table.IsOpen(i))
                    table.Toggle(i);
        var row = move.GetProperty("row").GetInt32();
        if (move.TryGetProperty("key", out var key))
        {
            host.State("folders")!.SelectedItem = folders[row];
            screen.Focus(part);
            var arrow = key.GetString() switch { "up" => Key.Up, "down" => Key.Down, "left" => Key.Left, _ => Key.Right };
            screen.KeyPressed(new KeyInput(arrow, Modifiers.Alt));
        }
        else
        {
            var from = RowBox(row) ?? throw new InvalidOperationException($"{name}: row {row} does not show");
            var onto = RowBox(move.GetProperty("target").GetInt32()) ?? throw new InvalidOperationException($"{name}: the target does not show");
            var y = move.GetProperty("where").GetString() switch
            {
                "before" => onto.Y + 3,
                "after" => onto.Bottom - 3,
                _ => onto.Y + onto.Height / 2,
            };
            screen.PointerPressed(from.X + 40, from.Y + from.Height / 2, PointerButton.Left);
            screen.PointerMoved(from.X + 40, from.Y + from.Height / 2 + 6);
            screen.PointerMoved(onto.X + 40, y);
            // A mark shows where it lands, unless that is the row itself.
            Assert.Equal(move.GetProperty("target").GetInt32() != row, part.Drop is not null);
            using (Shot.Of(screen, 800, 600)) { }
            screen.PointerReleased(onto.X + 40, y, PointerButton.Left);
            Assert.Null(part.Drop);
        }
        var events = host.Log.Where(l => l.StartsWith("run ")).ToList();
        if (@case.TryGetProperty("refused", out var refused) && refused.GetBoolean())
        {
            Assert.True(events.All(e => !e.Contains("reorder")) && host.Log.All(l => !l.StartsWith("write")), $"{name}: was not refused [{string.Join(", ", host.Log)}]");
            Assert.True(folders.SequenceEqual(host.Lists["folders"]), $"{name}: the order changed");
            return;
        }
        var now = host.Lists["folders"];
        var order = now.Select(f => folders.IndexOf(f)).ToList();
        Assert.True(@case.GetProperty("order").EnumerateArray().Select(o => o.GetInt32()).SequenceEqual(order), $"{name}: order [{string.Join(", ", order)}]");
        var numbers = now.Select(f => ((Folder)f).Position).ToList();
        Assert.True(@case.GetProperty("numbers").EnumerateArray().Select(o => o.GetInt64()).SequenceEqual(numbers), $"{name}: numbers [{string.Join(", ", numbers)}]");
        if (@case.TryGetProperty("parents", out var parents))
        {
            var got = now.Select(f => ((Folder)f).Parent).ToList();
            Assert.True(parents.EnumerateArray().Select(p => p.ValueKind == JsonValueKind.Null ? (long?)null : p.GetInt64()).SequenceEqual(got), $"{name}: parents [{string.Join(", ", got)}]");
        }
        // One edit, the moved row chosen, then reorder.
        Assert.Single(host.Log, l => l.StartsWith("write"));
        Assert.Same(folders[row], host.State("folders")!.SelectedItem);
        Assert.Equal("run folders reorder moved", events[^1]);
    }

    [Fact]
    public void Rows_of_a_list_move_as_every_stack_moves_them()
    {
        foreach (var @case in File.GetProperty("lists").EnumerateArray())
            Run(@case, tree: false);
    }

    [Fact]
    public void Rows_of_a_tree_move_as_every_stack_moves_them()
    {
        foreach (var @case in File.GetProperty("trees").EnumerateArray())
            Run(@case, tree: true);
    }

    [Fact]
    public void Without_reorder_rows_stay_put()
    {
        var host = new DataHost();
        host.Lists["folders"] = [new Folder { Id = 1 }, new Folder { Id = 2 }];
        var (screen, _, parts) = Made.Window("""<List at="1,1" span="12" rows="6" name="folders" bind="folders" show="name" />""", host);
        var list = (ListPart)parts[0];
        host.State("folders")!.SelectedItem = host.Lists["folders"][0];
        screen.Focus(list);
        screen.KeyPressed(new KeyInput(Key.Down, Modifiers.Alt));
        var from = list.RowBox(0)!.Value;
        var onto = list.RowBox(1)!.Value;
        screen.PointerPressed(from.X + 20, from.Y + 5, PointerButton.Left);
        screen.PointerMoved(onto.X + 20, onto.Bottom - 3);
        screen.PointerReleased(onto.X + 20, onto.Bottom - 3, PointerButton.Left);
        Assert.DoesNotContain(host.Log, l => l.StartsWith("write"));
        Assert.Equal(1L, ((Folder)host.Lists["folders"][0]).Id);
    }
}
