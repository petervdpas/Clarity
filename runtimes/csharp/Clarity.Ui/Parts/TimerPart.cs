// The timer (DESIGN 5.26), as on the web: it shows nothing and has no place
// in the window. While its window is on the screen and it is enabled, it
// runs its tick handler every so many seconds (`every`, 1 unless given, kept
// between a tenth of a second and a day). A tick that comes while the
// handler still runs is left out. It starts as its window joins the screen
// and stops as it leaves (closed, or under a window opened in place); back
// on the screen, it starts again.
// Its ticks come from Ticks.Source, which runs them on the screen's thread:
// the desktop and browser hosts set a FrameTicks they advance from their
// frame loops.

using System.Globalization;
using Clarity.Runtime;

namespace Clarity.Ui;

/// <summary>A clock that runs an action every so often, on the screen's thread, until it is disposed.</summary>
public interface ITicks
{
    IDisposable Every(TimeSpan every, Action tick);
}

/// <summary>Where the parts that keep time (a timer, a playing sound) get their ticks.</summary>
public static class Ticks
{
    /// <summary>The clock the parts tick by: the host's frame loop (FrameTicks), else the system's through the screen's thread; a test sets its own.</summary>
    public static ITicks Source { get; set; } = new SystemTicks();
}

/// <summary>
/// The system's clock: each tick is posted to the screen's thread
/// (<see cref="Later"/>, which the desktop window runs on its next frame and
/// the browser on its only thread).
/// </summary>
public sealed class SystemTicks : ITicks
{
    public IDisposable Every(TimeSpan every, Action tick) => new System.Threading.Timer(_ => Later.Post(tick), null, every, every);
}

/// <summary>
/// A clock the host's frame loop drives: each frame the host advances it to
/// the time of the frame, on the screen's thread, and every clock that is due
/// runs then. Time a long frame lost is not made up: a clock runs once and
/// waits its whole time again. A tick that fails is said and the others run on.
/// </summary>
public sealed class FrameTicks : ITicks
{
    sealed class Clock(FrameTicks owner, TimeSpan every, TimeSpan next, Action tick) : IDisposable
    {
        public TimeSpan Every { get; } = every;
        public TimeSpan Next { get; set; } = next;
        public Action Tick { get; } = tick;
        public bool Stopped { get; private set; }

        public void Dispose()
        {
            Stopped = true;
            owner.clocks.Remove(this);
        }
    }

    readonly List<Clock> clocks = [];
    TimeSpan now;

    /// <summary>A clock started: a host whose loop rests while no clock runs (a browser page) goes on.</summary>
    public event Action? Started;

    /// <summary>What a tick that fails says; the error output unless the host shows it.</summary>
    public Action<Exception> Failed { get; set; } = e => Console.Error.WriteLine($"a tick failed: {e}");

    /// <summary>Whether any clock runs, so the host keeps advancing it.</summary>
    public bool Running => clocks.Count > 0;

    public IDisposable Every(TimeSpan every, Action tick)
    {
        var clock = new Clock(this, every, now + every, tick);
        clocks.Add(clock);
        Started?.Invoke();
        return clock;
    }

    /// <summary>The frame's time came: each clock that is due runs, on the thread that calls this.</summary>
    public void Advance(TimeSpan time)
    {
        now = time;
        foreach (var clock in clocks.ToList())
        {
            if (clock.Stopped || now < clock.Next)
                continue;
            clock.Next += clock.Every;
            if (clock.Next <= now)
                clock.Next = now + clock.Every;
            try
            {
                clock.Tick();
            }
            catch (Exception e) when (e is not OutOfMemoryException)
            {
                Failed(e);
            }
        }
    }
}

[PartKind("timer")]
public sealed class TimerPart : Part
{
    IDisposable? clock;
    bool ticking;

    /// <summary>Never seen: it takes no pointer, no focus and draws nothing.</summary>
    public override bool IsShown => false;

    /// <summary>Seconds between its ticks: <c>every 0.5</c>.</summary>
    public double Every =>
        double.TryParse(Spec.Text("every"), NumberStyles.Float, CultureInfo.InvariantCulture, out var seconds) && double.IsFinite(seconds)
            ? Math.Clamp(seconds, 0.1, 86400)
            : 1;

    /// <summary>Whether its clock runs.</summary>
    public bool Running => clock is not null;

    /// <summary>Laid out with its window: it takes no room.</summary>
    public override void Arrange(Box box) => Bounds = new Box(box.X, box.Y, 0, 0);

    /// <summary>Its window joined the screen: its clock starts.</summary>
    protected override void OnAttached() => clock ??= Ticks.Source.Every(TimeSpan.FromSeconds(Every), Tick);

    /// <summary>Its window left the screen: its clock stops.</summary>
    protected override void OnDetached() => Stop();

    /// <summary>One tick: runs the handler when its window is on the screen, it is enabled and the last tick is done.</summary>
    public void Tick()
    {
        if (!Attached)
        {
            Stop();
            return;
        }
        if (ticking || !Enabled)
            return;
        ticking = true;
        try
        {
            Fire("tick");
        }
        finally
        {
            ticking = false;
        }
    }

    /// <summary>Its clock stops, until its window joins a screen again.</summary>
    public void Stop()
    {
        clock?.Dispose();
        clock = null;
    }
}
