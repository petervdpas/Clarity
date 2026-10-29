// What the screen sees of a long engine call and how the person stops it
// (DESIGN 5.12, `uses progress`): the shell watches the handler it runs, and
// every engine call the handler makes reports to that watch.

namespace Clarity.Runtime;

/// <summary>
/// What the screen sees of a long call and how the person stops it (<c>uses progress</c>,
/// conformance/progress.json): every report of how far it is goes to <paramref name="Progress"/>,
/// and once <paramref name="Stop"/> is cancelled the call ends, failing with "it was stopped".
/// A screen that must keep painting while the call runs gives <paramref name="Wait"/>: the
/// call then runs on another thread, and Wait waits for it in the screen's own way.
/// </summary>
public sealed record EngineWatch(Action<long, long>? Progress = null, CancellationToken Stop = default, Action<Task>? Wait = null);

/// <summary>The watch of the handler running now, which every engine call it makes reports to.</summary>
public static class Watching
{
    static readonly AsyncLocal<EngineWatch?> current = new();

    /// <summary>The watch of the handler running now; none outside one.</summary>
    public static EngineWatch? Current => current.Value;

    /// <summary>Runs <paramref name="run"/> with <paramref name="watch"/> as the watch of every engine call it makes, as a shell runs a handler.</summary>
    public static T Run<T>(EngineWatch watch, Func<T> run)
    {
        var was = current.Value;
        current.Value = watch;
        try
        {
            return run();
        }
        finally
        {
            current.Value = was;
        }
    }

    /// <summary>Runs <paramref name="run"/> with <paramref name="watch"/> as the watch of every engine call it makes.</summary>
    public static void Run(EngineWatch watch, Action run) => Run(watch, () =>
    {
        run();
        return 0;
    });
}
