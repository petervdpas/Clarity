// Where a canvas was clicked, in its own units, rounded down
// (conformance/drawing.json). A record that grows: the button or the kind of
// pointer can join x and y later without changing what reads them.

namespace Clarity.Avalonia;

public sealed record Pointer(long X, long Y);
