// Custom parts (DESIGN 5.19, 7.9): a class of the program's library that
// derives from CustomPart, found by its declared name, drawn in its place.
// Values come in (what it binds, its properties, enabled), code's hears
// come in, the person's change goes out and runs `on change`, and its tells
// run the handlers wired to them, on a host that runs tells and on a
// generated view model's Told_ method. The Gauge here is written as the
// generator writes the half of a part and the developer the adapter.

using System.Text.Json;
using Clarity.Runtime;

namespace Clarity.Ui.Tests;

/// <summary>Custom part Gauge as a library brings it: what it was set and told, and a click that changes its value.</summary>
[PartKind("Gauge")]
public sealed class Gauge : CustomPart
{
    public readonly List<string> Got = [];

    protected override void Set(string name, object? value) => Got.Add($"set {name} {value}");

    protected override void Hear(string hear, object?[] values) => Got.Add($"hear {hear} {string.Join(",", values)}");

    public override bool Focusable => true;

    public override void Draw(Painter painter) => painter.Fill(Bounds, painter.Look.Color("accent"));

    /// <summary>A click is the person's change of its value, and its tell `reached`.</summary>
    public override void OnClick()
    {
        ChangeValue(9L);
        Tell("reached", 3L, "high");
    }

    public void TellUnwired() => Tell("dropped", 1L);

    public static long Level(object? value) => As<long>(value);
}

public class CustomPartTests
{
    const string Line = """<Custom name="gauge" at="1,1" span="4" rows="3" part="Gauge" bind="speed" onchange="gaugeChanged" tells='[{"tell":"reached","handler":"gaugeReached"}]' />""";

    /// <summary>A host that runs tells, keeping each.</summary>
    sealed class TellingHost : TestHost
    {
        public override void Told(string part, string tell, JsonElement[] values) =>
            Log.Add($"told {part} {tell} {string.Join(",", values.Select(v => v.GetRawText()))}");
    }

    static (Screen Screen, CustomPlace Place, Gauge Gauge) Window(TestHost host)
    {
        var (screen, _, parts) = Made.Window(Line, host);
        var place = Assert.IsType<CustomPlace>(parts[0]);
        return (screen, place, Assert.IsType<Gauge>(place.Inner));
    }

    [Fact]
    public void It_is_the_library_s_class_found_by_its_declared_name_in_its_place()
    {
        var (screen, place, gauge) = Window(new TestHost());
        Assert.Equal(place.Bounds, gauge.Bounds);
        Assert.True(gauge.Bounds.Width > 0);
        Assert.Same(gauge, screen.At(gauge.Bounds.X + 4, gauge.Bounds.Y + 4));
        // It draws itself.
        using var shot = Shot.Of(screen, 800, 400);
        Assert.Equal(screen.Look.Color("accent"), shot.GetPixel((int)gauge.Bounds.X + 10, (int)gauge.Bounds.Y + 10));
    }

    [Fact]
    public void Its_values_come_in_from_its_line_its_binding_and_code()
    {
        var host = new TestHost();
        host.Values["speed"] = 3L;
        var state = host.State("gauge")!.With("title", "Speed");
        state.Hear("reset", 1L);
        var (screen, _, gauge) = Window(host);
        Assert.Equal(["set title Speed", "set value 3", "set enabled True", "hear reset 1"], gauge.Got);
        gauge.Got.Clear();
        state.SetValue("speed", 7L);
        state.Hear("reset", 2L);
        host.Values["speed"] = 4L;
        state.IsEnabled = false;
        screen.Resize(800, 400, 1);
        Assert.Equal(["set speed 7", "hear reset 2", "set value 4", "set enabled False"], gauge.Got);
        // Laid out again with nothing changed: nothing is given again.
        gauge.Got.Clear();
        screen.Resize(800, 400, 1);
        Assert.Empty(gauge.Got);
    }

    [Fact]
    public void The_person_s_change_goes_to_what_it_binds_and_its_tells_run_their_handlers()
    {
        var host = new TellingHost();
        var (screen, _, gauge) = Window(host);
        screen.PointerPressed(gauge.Bounds.X + 4, gauge.Bounds.Y + 4, PointerButton.Left);
        screen.PointerReleased(gauge.Bounds.X + 4, gauge.Bounds.Y + 4, PointerButton.Left);
        Assert.Equal(["write gauge speed 9", "run gauge change gaugeChanged", "told gauge reached 3,\"high\""], host.Log);
        // A tell no handler is wired to runs nothing.
        gauge.TellUnwired();
        Assert.Equal(3, host.Log.Count);
        // Its own change is not given back to it as a new value.
        gauge.Got.Clear();
        screen.Resize(800, 400, 1);
        Assert.Empty(gauge.Got);
    }

    /// <summary>A view model with what the generator writes for a part with wired tells.</summary>
    public sealed class GaugeViewModel : WindowViewModel
    {
        public GaugeViewModel() : base(null) { }

        public override string Title => "Gauges";

        public long Speed { get; set; }

        public PartState Gauge { get; } = new("gauge");

        public readonly List<string> Reached = [];

        public void Told_Gauge(string tell, JsonElement[] values)
        {
            switch (tell)
            {
                case "reached":
                    Handle(() => Reached.Add($"{PartValues.Read<long>(values, 0)} {PartValues.Read<string>(values, 1)}"), [tell, values]);
                    break;
            }
        }
    }

    [Fact]
    public void On_a_generated_window_a_tell_runs_its_Told_method()
    {
        var model = new GaugeViewModel();
        var host = new ViewModelHost(model);
        var (_, _, parts) = Made.Window(Line.Replace("onchange=\"gaugeChanged\" ", ""), host);
        var gauge = (Gauge)((CustomPlace)parts[0]).Inner;
        gauge.OnClick();
        Assert.Equal(9L, model.Speed);
        Assert.Equal(["3 high"], model.Reached);
    }

    [Fact]
    public void A_part_no_library_brings_says_so_in_its_place()
    {
        var host = new TestHost();
        var (_, _, parts) = Made.Window("""<Custom name="map" at="1,1" span="4" part="Map" />""", host);
        var missing = Assert.IsType<MissingPart>(((CustomPlace)parts[0]).Inner);
        Assert.Contains("Map", missing.Why);
    }

    [Fact]
    public void A_part_added_by_hand_is_found_too()
    {
        CustomParts.Add("HandGauge", () => new Gauge());
        var (_, _, parts) = Made.Window("""<Custom name="hand" at="1,1" span="4" part="HandGauge" />""", new TestHost());
        Assert.IsType<Gauge>(((CustomPlace)parts[0]).Inner);
    }

    [Fact]
    public void Values_are_read_as_the_types_the_declaration_gives()
    {
        Assert.Equal(5L, Gauge.Level(5L));
        Assert.Equal(5L, Gauge.Level(5));
        Assert.Equal(5L, Gauge.Level("5"));
    }
}
