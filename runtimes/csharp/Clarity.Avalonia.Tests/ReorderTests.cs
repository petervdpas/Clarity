// Reordering against the cases every renderer shares (conformance/reorder.json).

using System.Text.Json;
using Clarity.Avalonia;
using Xunit;

namespace Clarity.Avalonia.Tests;

public class ReorderTests
{
    static readonly JsonElement File = JsonDocument.Parse(System.IO.File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "reorder.json"))).RootElement;

    static object? Parent(JsonElement row) =>
        row.TryGetProperty("parent", out var p) && p.ValueKind != JsonValueKind.Null ? p.GetInt64() : null;

    static Reordered? Moved(JsonElement c, bool tree)
    {
        TreeRow[] rows = [.. c.GetProperty("rows").EnumerateArray().Select(r => new TreeRow(r.GetProperty("id").GetInt64(), Parent(r)))];
        var move = c.GetProperty("move");
        var row = move.GetProperty("row").GetInt32();
        if (move.TryGetProperty("key", out var key))
            return Reorder.Key(rows, row, Enum.Parse<MoveKey>(key.GetString()!, ignoreCase: true), tree);
        return Reorder.Drag(rows, row, Enum.Parse<DropAt>(move.GetProperty("where").GetString()!, ignoreCase: true), move.GetProperty("target").GetInt32(), tree);
    }

    static int[] Ints(JsonElement c, string name) => [.. c.GetProperty(name).EnumerateArray().Select(n => n.GetInt32())];

    [Fact]
    public void A_lists_rows_move_as_every_renderer_moves_them()
    {
        foreach (var c in File.GetProperty("lists").EnumerateArray())
        {
            var name = c.GetProperty("name").GetString();
            var moved = Moved(c, tree: false);
            if (c.TryGetProperty("refused", out _))
            {
                Assert.True(moved is null, name);
                continue;
            }
            Assert.True(moved is not null, name);
            Assert.True(Ints(c, "order").SequenceEqual(moved!.Order), name);
            Assert.True(Ints(c, "numbers").SequenceEqual(moved.Numbers), name);
        }
    }

    [Fact]
    public void A_trees_rows_move_as_every_renderer_moves_them()
    {
        foreach (var c in File.GetProperty("trees").EnumerateArray())
        {
            var name = c.GetProperty("name").GetString();
            var moved = Moved(c, tree: true);
            if (c.TryGetProperty("refused", out _))
            {
                Assert.True(moved is null, name);
                continue;
            }
            Assert.True(moved is not null, name);
            Assert.True(Ints(c, "order").SequenceEqual(moved!.Order), name);
            Assert.True(Ints(c, "numbers").SequenceEqual(moved.Numbers), name);
            var parents = c.GetProperty("parents").EnumerateArray().Select(p => p.ValueKind == JsonValueKind.Null ? null : (object?)p.GetInt64());
            Assert.True(parents.SequenceEqual(moved.Parents), name);
        }
    }
}
