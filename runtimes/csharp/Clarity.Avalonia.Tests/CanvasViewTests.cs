// An engine's canvas on Avalonia (DESIGN 5.12): where a pointer is in the
// drawing, Avalonia's keys by Clarity's names, and where words start
// (conformance/screen-engines.json). The frame loop itself is Clarity.Ui's.

using System.Text.Json;
using Clarity.Avalonia;
using Xunit;

namespace Clarity.Avalonia.Tests;

public sealed class CanvasViewTests
{
    static readonly JsonElement Screen = JsonDocument.Parse(File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "screen-engines.json"))).RootElement;

    [Fact]
    public void A_pointer_that_is_down_is_at_the_drawing_s_nearest_edge_beside_it()
    {
        var units = new global::Avalonia.Size(Screen.GetProperty("units").GetProperty("width").GetDouble(), Screen.GetProperty("units").GetProperty("height").GetDouble());
        foreach (var c in Screen.GetProperty("near").EnumerateArray())
        {
            var part = new global::Avalonia.Size(c.GetProperty("part").GetProperty("width").GetDouble(), c.GetProperty("part").GetProperty("height").GetDouble());
            var at = new global::Avalonia.Point(c.GetProperty("at").GetProperty("x").GetDouble(), c.GetProperty("at").GetProperty("y").GetDouble());
            var got = CanvasView.PointNear(units, part, at);
            var expected = c.GetProperty("x").ValueKind == JsonValueKind.Null ? null : new Pointer(c.GetProperty("x").GetInt64(), c.GetProperty("y").GetInt64());
            Assert.Equal(expected, got);
        }
    }

    [Fact]
    public void Keys_go_by_Clarity_s_names_once_down_and_once_up()
    {
        // Every name an engine gets has a key on Avalonia, and only those.
        var named = Enum.GetValues<global::Avalonia.Input.Key>().Select(ScreenKeys.NameOf).OfType<string>().ToHashSet();
        Assert.Equal(Screen.GetProperty("keys").EnumerateArray().Select(k => k.GetString()!).ToHashSet(), named);
        Assert.Equal("a", ScreenKeys.NameOf(global::Avalonia.Input.Key.A));
        Assert.Equal("7", ScreenKeys.NameOf(global::Avalonia.Input.Key.NumPad7));
        Assert.Equal("Enter", ScreenKeys.NameOf(global::Avalonia.Input.Key.Return));
        Assert.Equal("Shift", ScreenKeys.NameOf(global::Avalonia.Input.Key.RightShift));
        Assert.Null(ScreenKeys.NameOf(global::Avalonia.Input.Key.Tab));
        var held = new HeldKeys();
        Assert.True(held.Down("a"));
        Assert.False(held.Down("a"));
        Assert.True(held.Down("ArrowLeft"));
        Assert.True(held.Up("a"));
        Assert.False(held.Up("a"));
        Assert.Equal(["ArrowLeft"], held.All());
        Assert.Empty(held.All());
    }

    [Fact]
    public void Words_start_by_their_alignment()
    {
        Assert.Equal((0.0, 40.0, 20.0), (CanvasView.TextLeft("left", 40), CanvasView.TextLeft("right", 40), CanvasView.TextLeft("center", 40)));
    }
}
