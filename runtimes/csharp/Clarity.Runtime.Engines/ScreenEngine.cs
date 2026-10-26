// Engines beside the screen (DESIGN 5.12) on Avalonia: Wasmtime in the
// application's process, one instance per canvas, which keeps its memory
// from frame to frame. Each frame calls clarity_tick with the milliseconds
// since the last, then clarity_draw for the canvas's shapes, each call
// within the engine's limits; the epoch stops a call that runs too long.
// A frame that fails ends the engine (conformance/screen-engines.json).
// Pointer and keys since the last frame go first, to clarity_pointer and
// clarity_key, when it has them, and hears to its functions of their names;
// what its answers tell comes back with the frame.

using System.Text.Json.Nodes;

namespace Clarity.Runtime;

public sealed class ScreenEngine : IFrames
{
    /// <summary>The window of the one instance: an engine beside the screen keeps it per canvas.</summary>
    const long Canvas = 1;

    readonly string name;
    readonly Engine engine;
    EngineException? ended;

    ScreenEngine(string name, Engine engine)
    {
        this.name = name;
        this.engine = engine;
        Hears = new Hears(engine.Exports("clarity_pointer"), engine.Exports("clarity_key"));
    }

    public Hears Hears { get; }

    /// <summary>
    /// Loads <paramref name="bytes"/> as engine <paramref name="name"/> for one canvas;
    /// refuses a module that is no engine, imports more than <paramref name="uses"/> or
    /// lacks its frame loop. With <paramref name="opens"/>, clarity_open runs before the first frame.
    /// </summary>
    public static ScreenEngine Load(string name, byte[] bytes, IReadOnlyList<string> uses, bool opens, EngineLimits limits) =>
        new(name, Engine.Load(name, bytes, new EngineDeclaration([], uses, limits, EngineState.PerWindow, opens, Screen: true)));

    public Drawn Frame(double ms, IReadOnlyList<ScreenInput> input)
    {
        if (ended is not null)
            throw ended;
        told.Clear();
        try
        {
            foreach (var given in input)
            {
                if (given is ScreenInput.Pointer p && Hears.Pointer)
                    Call("clarity_pointer", p.Kind, p.X, p.Y);
                else if (given is ScreenInput.Key k && Hears.Keys)
                    Call("clarity_key", k.Kind, k.Name);
                else if (given is ScreenInput.Hear h)
                    Call(h.Name, h.Values);
            }
            Call("clarity_tick", ms);
            var shapes = Call("clarity_draw") as JsonArray ?? throw new EngineException($"engine {name} failed in clarity_draw: it gave no list of shapes");
            return new Drawn(shapes, told.ToList());
        }
        catch (EngineException e)
        {
            ended = e;
            engine.CloseWindow(Canvas);
            throw;
        }
    }

    /// <summary>What the calls of this frame told.</summary>
    readonly List<ScreenTell> told = [];

    JsonNode? Call(string function, params object?[] args)
    {
        try
        {
            return engine.CallIn(Canvas, function, told, args);
        }
        catch (RefusedException e)
        {
            // Beside the screen a refusal ends the engine as any failure does.
            throw new EngineException($"engine {name} failed in {function}: {e.Message}");
        }
    }

    public void Dispose() => engine.CloseWindow(Canvas);
}
