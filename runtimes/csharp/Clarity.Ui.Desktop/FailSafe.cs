// The desktop's fail safe (DESIGN 7.9): whatever goes wrong on the screen's
// thread (what the person did, drawing, laying out on a resize, a sidebar
// growing the window, time going on) is said as Clarity's error message in
// the window, and the program goes on, as Avalonia's dispatcher did. A
// failure that comes again every frame (a drawing that keeps failing) is
// said once, not sixty times a second.

namespace Clarity.Ui.Desktop;

public sealed class FailSafe(Action<Exception> say)
{
    string? lastSaid;
    long lastAt = long.MinValue;

    /// <summary>How long the same failure is not said again, in milliseconds.</summary>
    public const long Quiet = 5000;

    /// <summary>Runs <paramref name="action"/>; a failure is said (once while it repeats) and the program goes on. False when it failed.</summary>
    public bool Run(Action action, long now)
    {
        try
        {
            action();
            return true;
        }
        catch (Exception e) when (e is not OutOfMemoryException)
        {
            var what = $"{e.GetType().FullName}: {e.Message}";
            if (what == lastSaid && now - lastAt < Quiet)
                return false;
            (lastSaid, lastAt) = (what, now);
            Console.Error.WriteLine($"Unhandled: {e}");
            say(e);
            return false;
        }
    }
}
