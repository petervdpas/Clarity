// A long engine call on Avalonia (DESIGN 5.12): the handler that runs is
// watched, and while one of its engine calls runs on another thread the
// screen's thread waits in a frame of its own, so the window keeps painting,
// shows how far the call is with a bar and Stop, and takes no other handler.
// One handler runs at a time in an application, so this is the application's.

using Avalonia.Threading;
using Clarity.Runtime;

namespace Clarity.Avalonia;

public static class LongCall
{
    static CancellationTokenSource? stopping;
    static DateTime last;

    /// <summary>How far the call that runs is, done of total; none while no call reports.</summary>
    public static (long Done, long Total)? Current { get; private set; }

    /// <summary>Whether the screen waits for an engine call: no other handler runs then.</summary>
    public static bool Waiting { get; private set; }

    /// <summary>Raised on the screen's thread when Current changes.</summary>
    public static event Action? Changed;

    /// <summary>The person's Stop: the call ends at its next look, failing with "it was stopped".</summary>
    public static void Stop() => stopping?.Cancel();

    /// <summary>The watch of a handler that starts.</summary>
    internal static EngineWatch Start()
    {
        stopping?.Dispose();
        stopping = new CancellationTokenSource();
        last = default;
        return new EngineWatch(Report, stopping.Token, Wait);
    }

    /// <summary>The handler ended: the bar goes.</summary>
    internal static void End()
    {
        if (Current is null)
            return;
        Current = null;
        Changed?.Invoke();
    }

    // From the engine call's thread, at most ten times a second.
    static void Report(long done, long total)
    {
        var now = DateTime.UtcNow;
        if (now - last < TimeSpan.FromMilliseconds(100) && done < total)
            return;
        last = now;
        Dispatcher.UIThread.Post(() =>
        {
            Current = (done, total);
            Changed?.Invoke();
        });
    }

    // The screen's thread goes on painting until the call ended.
    static void Wait(Task call)
    {
        if (call.IsCompleted)
            return;
        var frame = new DispatcherFrame();
        call.ContinueWith(_ => Dispatcher.UIThread.Post(() => frame.Continue = false), TaskScheduler.Default);
        var was = Waiting;
        Waiting = true;
        try
        {
            Dispatcher.UIThread.PushFrame(frame);
        }
        finally
        {
            Waiting = was;
        }
    }
}
