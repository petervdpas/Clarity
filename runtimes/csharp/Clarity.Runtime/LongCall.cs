// A long engine call (DESIGN 5.12): the handler that runs is watched, and
// while one of its engine calls runs on another thread the screen waits as
// it says (Waiter): on the desktop the windows' loop goes on, so the window keeps
// painting, shows how far the call is with a bar and Stop, and takes no other
// handler. One handler runs at a time in the windows that show together, so a
// long call belongs to them: the window stack on the desktop (with the desktop
// windows it opened), a session on a server. Each window holds the long call
// of the windows it shows with (WindowViewModel.LongCall), and another
// stack's or session's handlers neither wait for it nor see it.

using Clarity.Runtime;

namespace Clarity.Runtime;

public sealed class LongCall
{
    readonly Lock gate = new();
    CancellationTokenSource? stopping;
    DateTime last;
    (long Done, long Total)? current;
    volatile bool waiting;

    /// <summary>How far the call that runs is, done of total; none while no call reports.</summary>
    public (long Done, long Total)? Current
    {
        get
        {
            lock (gate)
                return current;
        }
    }

    /// <summary>Whether the screen waits for an engine call: no other handler of these windows runs then.</summary>
    public bool Waiting => waiting;

    /// <summary>Raised on the screen's thread when Current changes.</summary>
    public event Action? Changed;

    /// <summary>How the screen waits for a call while it goes on showing the window; without one the call is waited for.</summary>
    public Action<Task>? Waiter { get; set; }

    /// <summary>The person's Stop: the call ends at its next look, failing with "it was stopped".</summary>
    public void Stop()
    {
        lock (gate)
            stopping?.Cancel();
    }

    /// <summary>The watch of a handler that starts.</summary>
    internal EngineWatch Start()
    {
        lock (gate)
        {
            stopping?.Dispose();
            stopping = new CancellationTokenSource();
            last = default;
            var started = stopping;
            return new EngineWatch((done, total) => Report(started, done, total), started.Token, Wait);
        }
    }

    /// <summary>The handler ended: the bar goes, and what its call still reports shows no more.</summary>
    internal void End()
    {
        bool shown;
        lock (gate)
        {
            shown = current is not null;
            current = null;
            stopping?.Dispose();
            stopping = null;
        }
        if (shown)
            Changed?.Invoke();
    }

    // From the engine call's thread, at most ten times a second.
    void Report(CancellationTokenSource of, long done, long total)
    {
        lock (gate)
        {
            var now = DateTime.UtcNow;
            if (stopping != of || (now - last < TimeSpan.FromMilliseconds(100) && done < total))
                return;
            last = now;
        }
        Later.Post(() =>
        {
            lock (gate)
            {
                // A report that comes after its handler ended shows nothing.
                if (stopping != of)
                    return;
                current = (done, total);
            }
            Changed?.Invoke();
        });
    }

    // The screen goes on showing the window until the call ended.
    void Wait(Task call)
    {
        if (call.IsCompleted)
            return;
        var was = waiting;
        waiting = true;
        try
        {
            if (Waiter is { } waiter)
                waiter(call);
            else
                ((IAsyncResult)call).AsyncWaitHandle.WaitOne();
        }
        catch (AggregateException) when (call.IsCompleted)
        {
            // Waiting only waits: the caller reads the call's own refusal or failure from it after.
        }
        finally
        {
            waiting = was;
        }
    }
}
