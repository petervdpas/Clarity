// What draws a canvas frame by frame (DESIGN 5.12): an engine beside the
// screen, which Clarity.Runtime.Engines hosts and Clarity.Avalonia's canvas
// runs at the screen's pace.

using System.Text.Json.Nodes;

namespace Clarity.Runtime;

/// <summary>A canvas's frames: each the milliseconds since the last, answered with the canvas's shapes.</summary>
public interface IFrames : IDisposable
{
    /// <summary>One frame: the shapes in the canvas's units (conformance/drawing.json); an EngineException when it fails.</summary>
    JsonArray Frame(double ms);
}
