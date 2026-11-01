// Custom parts on Avalonia (DESIGN 5.19 and 6.5): what a window's code sets
// and tells a part, kept in its PartState until the part that shows it
// hears it, and the values crossing to and from it as JSON.

using System.Text.Json;
using Avalonia.Headless.XUnit;
using Clarity.Avalonia;
using Xunit;

namespace Clarity.Avalonia.Tests;

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
