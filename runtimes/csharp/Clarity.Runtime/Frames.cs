// What draws a canvas frame by frame (DESIGN 5.12): an engine beside the
// screen, which Clarity.Runtime.Engines hosts and Clarity.Avalonia's canvas
// runs at the screen's pace. Pointer, keys and what handlers tell the engine
// (hears) wait for the next frame and go with it; what the engine tells
// comes back with the frame's shapes (conformance/screen-engines.json).

using System.Text.Json;
using System.Text.Json.Nodes;

namespace Clarity.Runtime;

/// <summary>Pointer or key on a canvas, as its engine gets it.</summary>
public abstract record ScreenInput
{
    /// <summary>The pointer: down, move or up, in the canvas's units.</summary>
    public sealed record Pointer(string Kind, long X, long Y) : ScreenInput;

    /// <summary>A key, down or up, by Clarity's name.</summary>
    public sealed record Key(string Kind, string Name) : ScreenInput;

    /// <summary>What a handler told the canvas (<c>sky.start(2)</c>): the engine's function of that name.</summary>
    public sealed record Hear(string Name, object?[] Values) : ScreenInput;

    /// <summary>Adds <paramref name="input"/> to what waits for the next frame: moves in a row keep only the last.</summary>
    public static void Queue(List<ScreenInput> waiting, ScreenInput input)
    {
        if (waiting.Count > 0 && waiting[^1] is Pointer { Kind: "move" } && input is Pointer { Kind: "move" })
            waiting[^1] = input;
        else
            waiting.Add(input);
    }
}

/// <summary>What the engine told the window: the handler wired to it runs with its values.</summary>
public sealed record ScreenTell(string Tell, JsonElement[] Values);

/// <summary>A frame: the canvas's shapes, and what the engine told while making it.</summary>
public sealed record Drawn(JsonArray Shapes, IReadOnlyList<ScreenTell> Tells);

/// <summary>Which input an engine beside the screen takes: it exports clarity_pointer, clarity_key.</summary>
public readonly record struct Hears(bool Pointer, bool Keys);

/// <summary>A canvas's frames: each the input and milliseconds since the last, answered with the canvas's shapes.</summary>
public interface IFrames : IDisposable
{
    /// <summary>The pointer and keys it takes.</summary>
    Hears Hears { get; }

    /// <summary>One frame: the shapes in the canvas's units (conformance/drawing.json) and what it told; an EngineException when it fails. Pointer and keys it does not take are left out.</summary>
    Drawn Frame(double ms, IReadOnlyList<ScreenInput> input);
}
