// Custom parts drawn by native libraries (DESIGN 5.19 and 6.5): what a
// window's code sets and tells a part, kept in its PartState until the
// NativePart that shows it hears it, and the values crossing to the part
// host as JSON, as every stack writes them.

using System.Text.Json;
using Avalonia.Headless.XUnit;
using Clarity.Avalonia;
using Xunit;

namespace Clarity.Avalonia.Tests;

public class NativePartTests
{
    [Fact]
    public void APropertyReadsAsCodeSetItElseItsFallback()
    {
        var arena = new PartState("arena").With("speed", 5L);
        Assert.Equal(5L, arena.ValueAs<long>("speed", "3"));
        Assert.Equal("Level one", arena.ValueAs<string>("title", "\"Level one\""));
        arena.SetValue("speed", 8L);
        Assert.Equal(8L, arena.ValueAs<long>("speed", "3"));
        Assert.Equal(2.5m, arena.ValueAs<decimal>("size", "\"2.5\""));
    }

    [Fact]
    public void HearsWaitForThePartAndComeInOrder()
    {
        var arena = new PartState("arena");
        arena.Hear("restart", 1L);
        arena.Hear("say", "Well done", true);
        var heard = new List<string>();
        arena.Listen((name, value) => { }, (hear, values) => heard.Add($"{hear} {NativePart.Json(values)}"));
        arena.Hear("pause");
        Assert.Equal(["restart [1]", "say [\"Well done\",true]", "pause []"], heard);
    }

    [Fact]
    public void ValuesCrossAsEveryStackWritesThem()
    {
        Assert.Equal("7", NativePart.Json(7L));
        Assert.Equal("\"2.50\"", NativePart.Json(2.50m));
        Assert.Equal("\"Level one\"", NativePart.Json("Level one"));
        Assert.Equal("true", NativePart.Json(true));
        Assert.Equal("\"2026-10-23\"", NativePart.Json(new DateOnly(2026, 10, 23)));
        Assert.Equal("null", NativePart.Json(null));
    }

    [Fact]
    public void ATellsValuesReadAsTheHandlersTypes()
    {
        var values = JsonDocument.Parse("[42, \"Ann\", true, \"9.95\"]").RootElement.EnumerateArray().ToArray();
        Assert.Equal(42L, NativePart.Read<long>(values, 0));
        Assert.Equal("Ann", NativePart.Read<string>(values, 1));
        Assert.True(NativePart.Read<bool>(values, 2));
        Assert.Equal(9.95m, NativePart.Read<decimal>(values, 3));
        Assert.Equal(0L, NativePart.Read<long>(values, 9));
    }
}

/// A part from a .NET library as a generated half and an adapter make one:
/// what code sets and tells it arrives typed, and what it tells goes out.
sealed partial class Gauge : DotNetPart
{
    public long Speed;
    public List<string> Heard = [];
    protected override void Set(string name, object? value)
    {
        if (name == "speed")
            Speed = As<long>(value);
    }

    protected override void Hear(string hear, object?[] values) => Heard.Add($"{hear} {As<long>(values[0])}");

    public void Press(long points) => Tell("scored", points);
}

public class DotNetPartTests
{
    [AvaloniaFact]
    public void AStateReachesThePartAndItsTellsComeBack()
    {
        var state = new PartState("gauge").With("speed", 5L);
        state.Hear("restart", 2L);
        var gauge = new Gauge { State = state };
        Assert.Equal(5L, gauge.Speed);
        Assert.Equal(["restart 2"], gauge.Heard);
        state.SetValue("speed", 8L);
        Assert.Equal(8L, gauge.Speed);
        var told = new List<string>();
        gauge.Told += (tell, values) => told.Add($"{tell} {values[0].GetInt64()}");
        gauge.Press(7);
        Assert.Equal(["scored 7"], told);
        // The person changing the value sets what it binds and says so.
        var changed = 0;
        gauge.ValueChanged += (_, _) => changed++;
        gauge.ChangeValue(12L);
        Assert.Equal(12L, gauge.Value);
        Assert.Equal(1, changed);
    }
}
