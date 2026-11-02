// A canvas an engine beside the screen draws (conformance/screen-engines.json):
// a frame at the screen's pace, the milliseconds since the last; pointer and
// keys wait for the next frame; what handlers tell it goes with it; what the
// engine tells runs the window's handler; an engine that fails says why in
// its place.

using System.Text.Json;
using System.Text.Json.Nodes;
using Clarity.Runtime;

namespace Clarity.Ui.Tests;

public class EngineCanvasPartTests
{
    static readonly JsonElement Engines = JsonDocument.Parse(File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "screen-engines.json"))).RootElement;

    /// <summary>An engine that logs each frame's milliseconds and input, and draws a dot where the pointer went last.</summary>
    sealed class Engine(Hears hears) : IFrames
    {
        public readonly List<(double Ms, List<ScreenInput> Input)> Frames = [];
        public string? FailWith;
        public bool Disposed;
        public List<ScreenTell> Telling = [];

        public Hears Hears => hears;

        public Drawn Frame(double ms, IReadOnlyList<ScreenInput> input)
        {
            if (FailWith is { } why)
                throw new EngineException(why);
            Frames.Add((ms, [.. input]));
            var shapes = new JsonArray { new JsonObject { ["op"] = "rect", ["x"] = 0.0, ["y"] = 0.0, ["width"] = 10.0, ["height"] = 10.0, ["color"] = "accent" } };
            var told = Telling;
            Telling = [];
            return new Drawn(shapes, told);
        }

        public void Dispose() => Disposed = true;
    }

    sealed class Run
    {
        public required CanvasPartHost Host;
        public required Screen Screen;
        public required EngineCanvasPart Part;
        public required Engine Engine;
        public TimeSpan Now;

        /// <summary>The screen draws a frame <paramref name="ms"/> milliseconds after the last.</summary>
        public void Frame(double ms)
        {
            Now += TimeSpan.FromMilliseconds(ms);
            using var shot = Shot.Of(Screen, 320, 200);
        }
    }

    static Run Running(Hears hears = default)
    {
        var host = new CanvasPartHost();
        var engine = new Engine(hears);
        host.Engines["sky"] = () => engine;
        var (screen, part) = CanvasPartTests.Alone("""<Canvas name="pad" runs="sky" units="{&quot;width&quot;:160,&quot;height&quot;:100}" />""", host, 320, 200);
        var run = new Run { Host = host, Screen = screen, Part = ((CanvasPart)part).Engine!, Engine = engine };
        run.Part.Clock = () => run.Now;
        // The engine's work runs at once, as a test waits for nothing.
        run.Part.Background = work => work();
        return run;
    }

    [Fact]
    public void A_canvas_that_runs_an_engine_is_drawn_by_it_frame_by_frame()
    {
        var run = Running();
        run.Frame(0);
        run.Frame(16);
        run.Frame(17);
        Assert.Equal([0.0, 16, 17], run.Engine.Frames.Select(f => f.Ms));
        Assert.IsType<Shape.Rect>(Assert.Single(run.Part.Drawing));
        // It asks the screen for every frame while it runs.
        Assert.Equal(1, run.Screen.Animating);
        using var shot = Shot.Of(run.Screen, 320, 200);
        Assert.Equal(ThemeLook.Clarity(dark: false).Color("accent"), shot.GetPixel(5, 5));
    }

    [Fact]
    public void An_engine_that_fails_says_why_in_its_place_and_runs_no_more()
    {
        var run = Running();
        run.Frame(0);
        run.Engine.FailWith = "out of time";
        run.Frame(16);
        Assert.Equal("out of time", run.Part.Stopped);
        Assert.True(run.Engine.Disposed);
        Assert.Contains("out of time", run.Part.StoppedText);
        Assert.Equal(0, run.Screen.Animating);
    }

    [Fact]
    public void A_pointer_that_is_down_is_at_the_drawing_s_nearest_edge_beside_it()
    {
        var units = Engines.GetProperty("units");
        var (uw, uh) = (units.GetProperty("width").GetDouble(), units.GetProperty("height").GetDouble());
        foreach (var c in Engines.GetProperty("near").EnumerateArray())
        {
            var part = c.GetProperty("part");
            var at = c.GetProperty("at");
            var got = CanvasPart.PointNear(uw, uh, part.GetProperty("width").GetDouble(), part.GetProperty("height").GetDouble(), at.GetProperty("x").GetDouble(), at.GetProperty("y").GetDouble());
            var expected = c.GetProperty("x").ValueKind == JsonValueKind.Null ? null : new Pointer(c.GetProperty("x").GetInt64(), c.GetProperty("y").GetInt64());
            Assert.Equal(expected, got);
        }
    }

    [Fact]
    public void The_pointer_goes_to_an_engine_that_takes_it_with_the_next_frame()
    {
        var run = Running(new Hears(Pointer: true, Keys: false));
        run.Frame(0);
        // 160 x 100 units in 320 x 200 pixels: 2 pixels a unit.
        run.Screen.PointerPressed(20, 40, PointerButton.Left);
        run.Screen.PointerMoved(30, 40);
        run.Screen.PointerMoved(40, 40);
        run.Screen.PointerReleased(400, 40, PointerButton.Left);
        run.Frame(16);
        Assert.Equal(
            // The pointer comes over the canvas before it goes down there.
            [new ScreenInput.Pointer("move", 10, 20), new ScreenInput.Pointer("down", 10, 20), new ScreenInput.Pointer("move", 20, 20), new ScreenInput.Pointer("up", 159, 20)],
            run.Engine.Frames[^1].Input);
    }

    [Fact]
    public void An_engine_that_takes_no_pointer_gets_none()
    {
        var run = Running();
        run.Frame(0);
        run.Screen.PointerPressed(20, 40, PointerButton.Left);
        run.Screen.PointerReleased(20, 40, PointerButton.Left);
        run.Frame(16);
        Assert.Empty(run.Engine.Frames[^1].Input);
    }

    [Fact]
    public void Keys_go_to_an_engine_that_takes_them_by_Clarity_s_names()
    {
        var run = Running(new Hears(Pointer: false, Keys: true));
        run.Frame(0);
        run.Screen.PointerPressed(20, 40, PointerButton.Left);
        run.Screen.PointerReleased(20, 40, PointerButton.Left);
        Assert.Equal(run.Part, run.Screen.Focused);
        run.Screen.KeyPressed(new KeyInput(Key.Left, Modifiers.None));
        // A key held repeats, and goes down once; it goes up when it is let go.
        run.Screen.KeyPressed(new KeyInput(Key.Left, Modifiers.None));
        // A shortcut with Control is the window's; Control itself is a key held.
        run.Screen.KeyPressed(new KeyInput(Key.S, Modifiers.Control));
        run.Screen.KeyPressed(new KeyInput(Key.Shift, Modifiers.Shift));
        run.Frame(16);
        var keys = run.Engine.Frames[^1].Input.OfType<ScreenInput.Key>().ToList();
        Assert.Equal([new ScreenInput.Key("down", "ArrowLeft"), new ScreenInput.Key("down", "Shift")], keys);
        run.Screen.KeyReleased(new KeyInput(Key.Left, Modifiers.Shift));
        // Leaving the canvas lets go of every key still down.
        run.Screen.Focus(null);
        run.Frame(16);
        keys = run.Engine.Frames[^1].Input.OfType<ScreenInput.Key>().ToList();
        Assert.Equal([new ScreenInput.Key("up", "ArrowLeft"), new ScreenInput.Key("up", "Shift")], keys);
    }

    [Fact]
    public void A_canvas_that_leaves_the_screen_ends_its_engine_and_one_that_joins_again_starts_a_new_one()
    {
        var host = new CanvasPartHost();
        var made = 0;
        Engine? last = null;
        host.Engines["sky"] = () =>
        {
            made++;
            return last = new Engine(default);
        };
        var root = new Hooked();
        var screen = new Screen(root, ThemeLook.Clarity(dark: false));
        var spec = WindowMarkup.Read("""<Window name="W"><Canvas name="pad" runs="sky" /></Window>""").Parts[0];
        var canvas = (CanvasPart)PartKinds.Make(spec, host);
        canvas.Engine!.Background = work => work();
        root.Add(canvas);
        Shot.Of(screen, 320, 200);
        Assert.Equal(1, made);
        var first = last!;
        root.Remove(canvas);
        Assert.True(first.Disposed);
        Assert.Equal(0, screen.Animating);
        root.Add(canvas);
        Shot.Of(screen, 320, 200);
        Assert.Equal(2, made);
        Assert.False(last!.Disposed);
    }

    [Fact]
    public void Every_key_Clarity_UI_has_goes_by_the_name_every_renderer_gives_it()
    {
        var names = Engines.GetProperty("keys").EnumerateArray().Select(k => k.GetString()!).ToHashSet();
        var named = Enum.GetValues<Key>().Select(ScreenKeys.NameOf).OfType<string>().ToHashSet();
        Assert.Subset(names, named);
        Assert.Equal("a", ScreenKeys.NameOf(Key.A));
        Assert.Equal("7", ScreenKeys.NameOf(Key.D7));
        Assert.Equal("Enter", ScreenKeys.NameOf(Key.Enter));
        Assert.Equal("F12", ScreenKeys.NameOf(Key.F12));
        Assert.Null(ScreenKeys.NameOf(Key.Tab));
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
    public void What_a_handler_tells_it_goes_with_the_next_frame_and_what_it_tells_runs_the_window_s_handler()
    {
        var run = Running();
        // Told before it showed, it waits for it.
        run.Host.State("pad")!.Hear("start", 2L);
        run.Frame(0);
        var hear = Assert.IsType<ScreenInput.Hear>(Assert.Single(run.Engine.Frames[0].Input));
        Assert.Equal("start", hear.Name);
        Assert.Equal([2L], hear.Values);
        run.Engine.Telling = [new ScreenTell("scored", [JsonDocument.Parse("3").RootElement])];
        run.Frame(16);
        var (part, tell, values) = Assert.Single(run.Host.Tells);
        Assert.Equal(("pad", "scored", 3), (part, tell, values[0].GetInt32()));
    }

    [Fact]
    public void A_canvas_whose_engine_the_window_has_not_says_so()
    {
        var host = new CanvasPartHost();
        var (screen, part) = CanvasPartTests.Alone("""<Canvas name="pad" runs="sky" />""", host, 320, 200);
        using var shot = Shot.Of(screen, 320, 200);
        Assert.NotNull(((CanvasPart)part).Engine!.Stopped);
    }
}
