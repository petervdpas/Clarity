// Where a canvas was clicked or dragged, in its own units, rounded down
// (conformance/drawing.json). A record that grows: the button or the kind of
// pointer can join x and y later without changing what reads them.

namespace Clarity.Avalonia;

public sealed record Pointer(long X, long Y)
{
    /// <summary>Where a drag started, across: <c>canvas.fromX</c>; 0 for a click.</summary>
    public long FromX { get; init; }

    /// <summary>Where a drag started, down: <c>canvas.fromY</c>.</summary>
    public long FromY { get; init; }

    /// <summary>The name of the link a person bent: <c>canvas.bent</c>; empty for a click or drag.</summary>
    public string Bent { get; init; } = "";

    /// <summary>That link's bend points now, x, y, x, y, ...: <c>canvas.via</c>.</summary>
    public IReadOnlyList<long> Via { get; init; } = [];
}
