// A C# custom part through the library's callback table: it is made when its
// window shows, hears what it binds, draws on the library's canvas, takes a
// click, and its change reaches the view model as an edit. What it throws
// stays on the C# side.

using System.Text.Json.Nodes;
using Clarity.Runtime;

namespace Clarity.Ui.Native.Tests;

[PartKind("TestGauge")]
public sealed class TestGauge : CustomPart
{
    public static readonly List<string> Heard = [];
    public static int Drawn;

    long value;

    protected override void Set(string name, object? given)
    {
        lock (Heard)
            Heard.Add($"{name}={given}");
        if (name == "value" && given is long n)
            value = n;
    }

    protected override void Hear(string hear, object?[] values)
    {
    }

    public override void Draw(Painter painter)
    {
        Drawn++;
        painter.Fill(Bounds, painter.Look.Color("accent"), 4);
        painter.Text($"{value}", Bounds.Inset(4), painter.Look.Color("text"), align: TextAlign.Center);
    }

    public override void OnClick() => ChangeValue(value + 1);
}

[PartKind("BrokenGauge")]
public sealed class BrokenGauge : CustomPart
{
    protected override void Set(string name, object? value) => throw new InvalidOperationException("broken gauge");

    protected override void Hear(string hear, object?[] values) => throw new InvalidOperationException("broken gauge");

    public override void Draw(Painter painter) => throw new InvalidOperationException("broken gauge");
}

public sealed class GaugeViewModel() : WindowViewModel(Db.Empty())
{
    public const string Markup = """
        <Window name="Gauge">
          <Custom name="speedo" part="TestGauge" bind="speed" at="1,1" span="4" rows="2" undoScope="speedo" />
          <Custom name="broken" part="BrokenGauge" bind="speed" at="5,1" span="4" rows="2" />
        </Window>
        """;

    public long Speed { get; set; } = 41;
}

public sealed class CustomPartTests
{
    [Fact]
    public void A_csharp_custom_part_hears_its_value_draws_and_its_change_reaches_the_view_model()
    {
        CustomParts.Add("TestGauge", () => new TestGauge());
        CustomParts.Add("BrokenGauge", () => new BrokenGauge());
        var gauge = new GaugeViewModel();
        var wire = new ProgramWire(typeof(GaugeViewModel).Assembly, gauge, n => n == "Gauge" ? GaugeViewModel.Markup : null);
        var files = new NativeFiles();
        files.Window("Gauge", GaugeViewModel.Markup);
        using var driven = new Driven(wire.Answer, files);
        var read = driven.Step(new JsonObject { ["do"] = "read" });
        Assert.True(read["ok"]!.GetValue<bool>(), read.ToJsonString());
        lock (TestGauge.Heard)
            Assert.Contains("value=41", TestGauge.Heard);
        // A shot draws the screen: the part draws on the library's canvas, and the broken one beside it does not stop it.
        var shot = Path.Combine(Path.GetTempPath(), $"gauge-{Guid.NewGuid():N}.png");
        try
        {
            Assert.True(driven.Step(new JsonObject { ["do"] = "shot", ["path"] = shot })["ok"]!.GetValue<bool>());
        }
        finally
        {
            File.Delete(shot);
        }
        Assert.True(TestGauge.Drawn > 0);
        driven.Step(new JsonObject { ["do"] = "click", ["part"] = "speedo" });
        driven.Step(new JsonObject { ["do"] = "settle", ["ms"] = 50 });
        Assert.Equal(42, gauge.Speed);
    }
}
