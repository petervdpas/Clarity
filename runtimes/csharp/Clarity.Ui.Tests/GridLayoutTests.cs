// The grid against the cases every version of Clarity.UI shares
// (conformance/grid-layout.json).

using System.Text.Json;

namespace Clarity.Ui.Tests;

public class GridLayoutTests
{
    static readonly JsonElement File = JsonDocument.Parse(System.IO.File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "grid-layout.json"))).RootElement;

    [Fact]
    public void A_grid_places_its_parts_as_every_version_does()
    {
        Assert.Equal(GridLayout.RowHeight, File.GetProperty("row_height").GetDouble());
        foreach (var c in File.GetProperty("cases").EnumerateArray())
        {
            var what = c.GetProperty("what").GetString()!;
            GridItem[] parts = [.. c.GetProperty("parts").EnumerateArray().Select(p => new GridItem(
                p.GetProperty("col").GetInt32(), p.GetProperty("row").GetInt32(),
                p.GetProperty("width").GetInt32(), p.GetProperty("height").GetInt32(), p.GetProperty("grows").GetBoolean()))];
            var placed = GridLayout.Place(parts, c.GetProperty("columns").GetInt32(), c.GetProperty("width").GetDouble(), c.GetProperty("height").GetDouble(), c.GetProperty("gap").GetDouble());
            var gives = c.GetProperty("gives");
            Assert.True(Math.Abs(gives.GetProperty("content").GetDouble() - placed.Content) < 0.001, $"{what}: content {placed.Content}");
            var boxes = gives.GetProperty("boxes").EnumerateArray().ToArray();
            Assert.Equal(boxes.Length, placed.Boxes.Count);
            for (var i = 0; i < boxes.Length; i++)
            {
                double[] want = [.. boxes[i].EnumerateArray().Select(v => v.GetDouble())];
                var got = placed.Boxes[i];
                double[] have = [got.X, got.Y, got.Width, got.Height];
                for (var k = 0; k < 4; k++)
                    Assert.True(Math.Abs(want[k] - have[k]) < 0.001, $"{what}: part {i + 1} is at {got}, not [{string.Join(", ", want)}]");
            }
        }
    }
}
