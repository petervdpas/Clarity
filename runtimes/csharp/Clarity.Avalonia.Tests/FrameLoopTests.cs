// A canvas an engine beside the screen draws, on Avalonia (DESIGN 5.12):
// frames at the screen's pace, each worked off the screen's thread, one at
// a time; a failure ends the loop and the canvas says why
// (conformance/screen-engines.json, conformance/drawing.json).

using System.Text.Json;
using System.Text.Json.Nodes;
using Clarity.Avalonia;
using Clarity.Runtime;
using Xunit;

namespace Clarity.Avalonia.Tests;

public sealed class FrameLoopTests
{
    /// <summary>Frames the test answers: what each frame was given, and what it gives back.</summary>
    sealed class Frames : IFrames
    {
        public readonly List<double> Given = [];
        public readonly List<IReadOnlyList<ScreenInput>> Input = [];
        public Hears Hears { get; set; } = new(true, true);
        public Func<double, JsonArray> Answer = _ => [new JsonObject { ["op"] = "clear", ["color"] = "background" }];
        public bool Disposed;

        public List<ScreenTell> Tells = [];

        public Drawn Frame(double ms, IReadOnlyList<ScreenInput> input)
        {
            Given.Add(ms);
            Input.Add(input);
            return new Drawn(Answer(ms), Tells);
        }

        public void Dispose() => Disposed = true;
    }

    /// <summary>A loop whose screen frames, background work and screen thread the test steps.</summary>
    sealed class Harness
    {
        public readonly Frames Frames = new();
        public readonly List<Action> Background = [];
        public readonly List<IReadOnlyList<Shape>> Drawn = [];
        public readonly List<string> Stopped = [];
        public readonly List<ScreenTell> Told = [];
        public Action<TimeSpan>? Waiting;
        public readonly FrameLoop Loop;

        public Harness(Hears? hears = null)
        {
            if (hears is { } h)
                Frames.Hears = h;
            Loop = new FrameLoop(Frames, f => Waiting = f, Background.Add, ui => ui(), Drawn.Add, Stopped.Add, Told.Add);
            Loop.Start();
        }

        public void Step(double ms)
        {
            var f = Waiting;
            Waiting = null;
            f?.Invoke(TimeSpan.FromMilliseconds(ms));
        }

        public void Work()
        {
            var work = Background.ToList();
            Background.Clear();
            work.ForEach(w => w());
        }
    }

    [Fact]
    public void Each_frame_gives_the_time_since_the_last_and_draws_its_shapes()
    {
        var h = new Harness();
        h.Step(1000);
        Assert.Empty(h.Frames.Given);
        h.Work();
        h.Step(1016);
        h.Work();
        Assert.Equal([0, 16], h.Frames.Given);
        Assert.Equal(2, h.Drawn.Count);
        Assert.Equal(new Shape.Clear("background"), h.Drawn[0][0]);
    }

    [Fact]
    public void An_engine_gets_whole_milliseconds_their_fractions_carried()
    {
        var h = new Harness();
        h.Step(1000.4);
        h.Work();
        // A screen's frame clock has fractions of a millisecond; the engine takes whole ones.
        h.Step(1017.07);
        h.Work();
        h.Step(1033.74);
        h.Work();
        Assert.Equal([0, 16, 17], h.Frames.Given);
    }

    [Fact]
    public void A_frame_is_skipped_while_the_last_is_busy()
    {
        var h = new Harness();
        h.Step(1000);
        h.Step(1016);
        h.Step(1033);
        Assert.Single(h.Background);
        h.Work();
        h.Step(1050);
        h.Work();
        Assert.Equal([0, 50], h.Frames.Given);
    }

    [Fact]
    public void A_frame_that_fails_ends_the_loop_and_says_why()
    {
        var h = new Harness();
        h.Frames.Answer = _ => throw new EngineException("engine Screen failed in clarity_tick: a frame of more than ten seconds");
        h.Step(1000);
        h.Work();
        Assert.Equal(["engine Screen failed in clarity_tick: a frame of more than ten seconds"], h.Stopped);
        // No frame runs after it; the engine goes once its last frame is done.
        h.Step(1016);
        h.Work();
        Assert.Single(h.Frames.Given);
        Assert.True(h.Frames.Disposed);
    }

    [Fact]
    public void Stopping_the_loop_ends_the_engine()
    {
        var h = new Harness();
        h.Step(1000);
        h.Loop.Dispose();
        h.Work();
        Assert.Empty(h.Drawn);
        Assert.True(h.Frames.Disposed);
        h.Step(1016);
        Assert.Empty(h.Background);
    }

    [Fact]
    public void Pointer_and_keys_wait_for_the_next_frame_and_go_with_it()
    {
        var h = new Harness();
        h.Loop.Pointer("down", 4, 5);
        h.Loop.Key("down", "a");
        h.Step(1000);
        // While the frame is busy, input waits for the next one.
        h.Loop.Pointer("move", 6, 7);
        h.Loop.Pointer("move", 8, 9);
        h.Work();
        h.Step(1016);
        h.Work();
        Assert.Equal([new ScreenInput.Pointer("down", 4, 5), new ScreenInput.Key("down", "a")], h.Frames.Input[0]);
        Assert.Equal([new ScreenInput.Pointer("move", 8, 9)], h.Frames.Input[1]);
    }

    [Fact]
    public void Hears_wait_for_the_next_frame_and_tells_come_once_it_is_drawn()
    {
        var h = new Harness(new Hears(false, false));
        h.Loop.Hear("start", [2L]);
        h.Frames.Tells = [new ScreenTell("lap", [JsonDocument.Parse("1").RootElement])];
        h.Step(1000);
        Assert.Empty(h.Told);
        h.Work();
        var hear = Assert.IsType<ScreenInput.Hear>(Assert.Single(h.Frames.Input[0]));
        Assert.Equal(("start", 2L), (hear.Name, hear.Values[0]));
        Assert.Equal("lap", Assert.Single(h.Told).Tell);
        // A loop that ended hears nothing more.
        h.Loop.Dispose();
        h.Loop.Hear("start", [3L]);
        h.Step(1016);
        Assert.Single(h.Frames.Input);
    }

    [Fact]
    public void Input_an_engine_does_not_hear_is_not_sent()
    {
        var h = new Harness(new Hears(false, true));
        h.Loop.Pointer("down", 1, 1);
        h.Loop.Key("down", "a");
        h.Step(1000);
        h.Work();
        Assert.Equal([new ScreenInput.Key("down", "a")], h.Frames.Input[0]);
        Assert.False(h.Loop.Hears.Pointer);
    }

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

    /// <summary>An engine sends its shapes as a draw handler's (conformance/drawing.json).</summary>
    [Fact]
    public void Shapes_are_read_as_a_draw_handler_sends_them()
    {
        var file = JsonDocument.Parse(File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "drawing.json")));
        Shape[] expected =
        [
            new Shape.Clear("#101810"),
            new Shape.Line(10, 20, 30, 40, "accent", 3),
            new Shape.Line(10, 20, 30, 40, "accent", 1),
            new Shape.Rect(5, 5, 20, 10, "text"),
            new Shape.Circle(50, 50, 40, "accent"),
            new Shape.Ring(50, 50, 40, "border", 2),
            new Shape.Ring(50, 50, 40, "border", 1),
        ];
        var sent = file.RootElement.GetProperty("shapes").EnumerateArray().Select(s => Shape.Of(JsonNode.Parse(s.GetProperty("sends").GetRawText())!)).ToList();
        Assert.Equal(expected, sent.Take(expected.Length));
        var words = Assert.IsType<Shape.Words>(sent[^3]);
        // A picture of the resources folder, by its name.
        Assert.Equal(new Shape.Image(10, 20, 30, 15, "pill.svg"), sent[^1]);
        // A number may come as its text, as an engine gives a decimal.
        Assert.Equal(new Shape.Circle(1.5, 2, 3, "text"), Shape.Of(JsonNode.Parse("""{"op": "circle", "x": "1.5", "y": 2, "r": 3}""")!));
        Assert.Equal((50.0, 90.0, "km/h", "muted", 8.0), (words.X, words.Y, words.Text.Show(), words.Color, words.Size));
        Assert.Throws<EngineException>(() => Shape.Of(new JsonObject { ["op"] = "star" }));
    }
}
