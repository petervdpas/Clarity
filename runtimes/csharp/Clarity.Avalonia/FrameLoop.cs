// The frame loop of a canvas an engine beside the screen draws (DESIGN
// 5.12): a frame at the screen's pace, its work off the screen's thread so
// the window stays painted, one frame at a time: a frame that comes while
// the last is busy is skipped, its time not lost. A frame that fails ends
// the loop, and the canvas says why. The engine's own limits stop a frame
// that runs too long (ScreenEngine).

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
            background(() => Work(ms));
        }
        request(Tick);
    }

    void Work(double ms)
    {
        try
        {
            var shapes = Shape.All(frames.Frame(ms));
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
