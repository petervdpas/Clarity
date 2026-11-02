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

    /// <summary>The <c>made</c> cases: each part made from its markup says itself whether it grows (a custom part by its <c>grows</c>, as its declaration says).</summary>
    [Fact]
    public void Parts_made_from_their_markup_grow_as_every_version_does()
    {
        var cases = File.GetProperty("made").EnumerateArray().ToArray();
        Assert.NotEmpty(cases);
        foreach (var c in cases)
        {
            var what = c.GetProperty("what").GetString()!;
            var host = new TestHost();
            GridItem[] parts = [.. c.GetProperty("parts").EnumerateArray().Select(p =>
            {
                var spec = WindowMarkup.Read($"<Window name=\"Test\">{p.GetString()}</Window>").Parts[0];
                return spec.Place(PartKinds.Make(spec, host).Grows);
            })];
            var placed = GridLayout.Place(parts, c.GetProperty("columns").GetInt32(), c.GetProperty("width").GetDouble(), c.GetProperty("height").GetDouble(), c.GetProperty("gap").GetDouble());
            var gives = c.GetProperty("gives");
            Assert.True(Math.Abs(gives.GetProperty("content").GetDouble() - placed.Content) < 0.001, $"{what}: content {placed.Content}");
            var boxes = gives.GetProperty("boxes").EnumerateArray().ToArray();
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
