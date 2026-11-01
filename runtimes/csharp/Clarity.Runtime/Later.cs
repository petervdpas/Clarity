// What runs once the work at hand is done: a change handler's edit is joined
// to the edit its value made until then (DESIGN 5.29). The screen that shows
// the windows says how; Clarity.Avalonia posts it to Avalonia's UI thread,
// and elsewhere it goes to the current synchronization context, or runs at once.

namespace Clarity.Runtime;

public static class Later
{
    /// <summary>How the screen runs an action once the work at hand is done.</summary>
    public static Action<Action>? Poster { get; set; }

    /// <summary>Runs <paramref name="action"/> once the work at hand is done.</summary>
    public static void Post(Action action)
    {
        if (Poster is { } poster)
            poster(action);
        else if (SynchronizationContext.Current is { } context)
            context.Post(_ => action(), null);
        else
            action();
    }
}

/// <summary>The size a window opens at, in pixels: <c>size 320 x 460</c>.</summary>
public readonly record struct Extent(double Width, double Height);
