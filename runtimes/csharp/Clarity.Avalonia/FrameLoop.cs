// The frame loop of a canvas an engine beside the screen draws (DESIGN
// 5.12): a frame at the screen's pace, its work off the screen's thread so
// the window stays painted, one frame at a time: a frame that comes while
// the last is busy is skipped, its time not lost. A frame that fails ends
// the loop, and the canvas says why. The engine's own limits stop a frame
// that runs too long (ScreenEngine). Pointer, keys and hears wait for the
// next frame and go with it; what the engine tells comes once it is drawn.

using Clarity.Runtime;

namespace Clarity.Avalonia;

public sealed class FrameLoop(
    IFrames frames,
    Action<Action<TimeSpan>> request,
    Action<Action> background,
    Action<Action> ui,
    Action<IReadOnlyList<Shape>> drawn,
    Action<string> stopped,
    Action<ScreenTell>? told = null) : IDisposable
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

    /// <summary>What a handler told the engine: kept for the next frame.</summary>
    public void Hear(string name, object?[] values)
    {
        if (!ended)
            ScreenInput.Queue(waiting, new ScreenInput.Hear(name, values));
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
            var frame = frames.Frame(ms, input);
            var shapes = Shape.All(frame.Shapes);
            ui(() =>
            {
                busy = false;
                if (ended)
                    return;
                drawn(shapes);
                foreach (var tell in frame.Tells)
                    told?.Invoke(tell);
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
