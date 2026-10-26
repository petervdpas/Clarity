// The frame loop of a canvas an engine beside the screen draws (DESIGN
// 5.12): a frame at the screen's pace, its work off the screen's thread so
// the window stays painted, one frame at a time: a frame that comes while
// the last is busy is skipped, its time not lost. A frame that fails ends
// the loop, and the canvas says why. The engine's own limits stop a frame
// that runs too long (ScreenEngine). Pointer and keys wait for the next
// frame and go with it.

using Clarity.Runtime;

namespace Clarity.Avalonia;

public sealed class FrameLoop(
    IFrames frames,
    Action<Action<TimeSpan>> request,
    Action<Action> background,
    Action<Action> ui,
    Action<IReadOnlyList<Shape>> drawn,
    Action<string> stopped) : IDisposable
{
    bool busy;
    bool ended;
    TimeSpan? last;
    List<ScreenInput> waiting = [];

    /// <summary>The pointer and keys its engine takes.</summary>
    public Hears Hears => frames.Hears;

    /// <summary>The pointer on the canvas, in its units.</summary>
    public void Pointer(string kind, long x, long y)
    {
        if (!ended && Hears.Pointer)
            ScreenInput.Queue(waiting, new ScreenInput.Pointer(kind, x, y));
    }

    /// <summary>A key on the canvas, by Clarity's name.</summary>
    public void Key(string kind, string name)
    {
        if (!ended && Hears.Keys)
            ScreenInput.Queue(waiting, new ScreenInput.Key(kind, name));
    }

    /// <summary>Asks for the first frame.</summary>
    public void Start() => request(Tick);

    void Tick(TimeSpan now)
    {
        if (ended)
            return;
        if (!busy)
        {
            var ms = last is { } before ? (now - before).TotalMilliseconds : 0;
            last = now;
            busy = true;
            var input = waiting;
            waiting = [];
            background(() => Work(ms, input));
        }
        request(Tick);
    }

    void Work(double ms, IReadOnlyList<ScreenInput> input)
    {
        try
        {
            var shapes = Shape.All(frames.Frame(ms, input));
            ui(() =>
            {
                busy = false;
                if (!ended)
                    drawn(shapes);
            });
        }
        catch (EngineException e)
        {
            ui(() =>
            {
                if (ended)
                    return;
                End();
                stopped(e.Message);
            });
        }
    }

    void End()
    {
        ended = true;
        // The engine may be in a frame still: it goes once that is done.
        background(frames.Dispose);
    }

    /// <summary>Ends the loop: its canvas goes.</summary>
    public void Dispose()
    {
        if (!ended)
            End();
    }
}
