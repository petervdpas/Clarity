// The open windows of an application, by number: every window that opens
// gets a number of its own, and what belongs to one open window (an engine's
// state per window) hears when it closes.

namespace Clarity.Runtime;

public static class OpenWindows
{
    static long next;

    /// <summary>Tells what listens that a window has closed, by its number.</summary>
    public static event Action<long>? Closed;

    /// <summary>The number of a window that opens.</summary>
    public static long Next() => Interlocked.Increment(ref next);

    /// <summary>Window <paramref name="instance"/> has closed, once its own <c>on close</c> has run.</summary>
    public static void Close(long instance) => Closed?.Invoke(instance);
}
