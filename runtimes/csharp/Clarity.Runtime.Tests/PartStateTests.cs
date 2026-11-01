// Custom parts (DESIGN 5.19 and 6.5): what a window's code sets and tells a
// part, kept in its PartState until the part that shows it hears it, and the
// values crossing to and from it as JSON.

using System.Text.Json;
using Xunit;

namespace Clarity.Runtime.Tests;

public class PartStateTests
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
        arena.Listen((name, value) => { }, (hear, values) => heard.Add($"{hear} {PartValues.Write(values)}"));
        arena.Hear("pause");
        Assert.Equal(["restart [1]", "say [\"Well done\",true]", "pause []"], heard);
    }

    [Fact]
    public void ValuesCrossAsEveryStackWritesThem()
    {
        Assert.Equal("7", PartValues.Write(7L));
        Assert.Equal("\"2.50\"", PartValues.Write(2.50m));
        Assert.Equal("\"Level one\"", PartValues.Write("Level one"));
        Assert.Equal("true", PartValues.Write(true));
        Assert.Equal("\"2026-10-23\"", PartValues.Write(new DateOnly(2026, 10, 23)));
        Assert.Equal("null", PartValues.Write(null));
    }

    [Fact]
    public void ATellsValuesReadAsTheHandlersTypes()
    {
        var values = JsonDocument.Parse("[42, \"Ann\", true, \"9.95\"]").RootElement.EnumerateArray().ToArray();
        Assert.Equal(42L, PartValues.Read<long>(values, 0));
        Assert.Equal("Ann", PartValues.Read<string>(values, 1));
        Assert.True(PartValues.Read<bool>(values, 2));
        Assert.Equal(9.95m, PartValues.Read<decimal>(values, 3));
        Assert.Equal(0L, PartValues.Read<long>(values, 9));
    }
}

/// A part from a .NET library as a generated half and an adapter make one:
/// what code sets and tells it arrives typed, and what it tells goes out.
