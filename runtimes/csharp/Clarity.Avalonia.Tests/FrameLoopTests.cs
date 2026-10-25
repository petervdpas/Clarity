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
        public Func<double, JsonArray> Answer = _ => [new JsonObject { ["op"] = "clear", ["color"] = "background" }];
        public bool Disposed;

        public JsonArray Frame(double ms)
        {
            Given.Add(ms);
            return Answer(ms);
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
        public Action<TimeSpan>? Waiting;
        public readonly FrameLoop Loop;

        public Harness()
        {
            Loop = new FrameLoop(Frames, f => Waiting = f, Background.Add, ui => ui(), Drawn.Add, Stopped.Add);
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
        var words = Assert.IsType<Shape.Words>(sent[^2]);
        Assert.Equal((50.0, 90.0, "km/h", "muted", 8.0), (words.X, words.Y, words.Text.Show(), words.Color, words.Size));
        Assert.Throws<EngineException>(() => Shape.Of(new JsonObject { ["op"] = "star" }));
    }
}
