// Canvas modes (DESIGN 5.31) against the cases every runtime shares
// (conformance/canvas-modes.json): a drawing's SVG for its shapes and back,
// and what each action on a canvas makes of the window's data.

using System.Text.Json.Nodes;
using Clarity.Avalonia;
using Xunit;

namespace Clarity.Avalonia.Tests;

public class CanvasModeTests
{
    static readonly JsonObject File = JsonNode.Parse(System.IO.File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "canvas-modes.json")))!.AsObject();

    static int[] Units(JsonNode? units) => units is JsonArray a ? [.. a.Select(n => (int)n!)] : [1000, 1000];

    static JsonArray Shapes(IEnumerable<JsonObject> shapes) => [.. shapes.Select(s => (JsonNode)s.DeepClone())];

    [Fact]
    public void A_drawing_is_kept_as_svg_one_element_on_a_line()
    {
        foreach (var c in File["svg"]!.AsArray())
        {
            var name = (string?)c!["name"];
            var units = Units(c["units"]);
            var shapes = c["shapes"]!.AsArray().Select(s => s!.AsObject()).ToList();
            Assert.True((string?)c["text"] == CanvasModes.DrawingText(units, shapes), name);
            var (read, back) = CanvasModes.DrawingShapes((string)c["text"]!, [1000, 1000]);
            Assert.True(units.SequenceEqual(read), name);
            Assert.True(JsonNode.DeepEquals(c["shapes"], Shapes(back)), $"{name}: {Shapes(back).ToJsonString()}");
        }
        foreach (var c in File["read"]!.AsArray())
        {
            var name = (string?)c!["name"];
            var (units, shapes) = CanvasModes.DrawingShapes((string)c["text"]!, [1000, 1000]);
            if (c["units"] is JsonArray)
                Assert.True(Units(c["units"]).SequenceEqual(units), name);
            Assert.True(JsonNode.DeepEquals(c["shapes"], Shapes(shapes)), name);
        }
    }

    static CanvasArgs Args(JsonNode c) => new((string)c["mode"]!)
    {
        List = (string?)c["list"] ?? "",
        At = c["at"] is JsonArray at ? [(string)at[0]!, (string)at[1]!] : ["x", "y"],
        Path = (string?)c["path"] ?? "",
        Units = c["units"] is JsonArray units ? [.. units.Select(n => (int)n!)] : null,
        Grid = (long?)c["grid"] ?? 0,
        Snap = (bool?)c["snap"] ?? false,
        Readonly = (bool?)c["readonly"] ?? false,
    };

    [Fact]
    public void An_action_on_a_canvas_changes_the_data_as_every_runtime_does()
    {
        foreach (var c in File["actions"]!.AsArray())
        {
            var name = (string?)c!["name"];
            var data = c["data"]!.DeepClone().AsObject();
            var selected = new JsonObject();
            var changed = CanvasModes.Event(data, selected, "canvas", Args(c), c["action"]!.AsObject());
            var refused = (bool?)c["refused"] ?? false;
            Assert.True(changed != refused, $"{name}: changed {changed}");
            if (refused)
            {
                Assert.True(JsonNode.DeepEquals(c["data"], data), $"{name}: a refused action changed the data");
                continue;
            }
            Assert.True(JsonNode.DeepEquals(c["after"], data), $"{name}: {data.ToJsonString()}");
            var want = CanvasModes.Number(c["selected"]);
            Assert.True(want == CanvasModes.Number(selected["canvas"]), $"{name}: selected {selected["canvas"]?.ToJsonString()}");
        }
    }
}
