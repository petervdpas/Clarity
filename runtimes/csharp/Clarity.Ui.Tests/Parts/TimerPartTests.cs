// The timer (DESIGN 5.26), as the web screen and Avalonia's Timer did it: no
// place in the window; while its window is on the screen and it is enabled,
// its tick handler runs every so many seconds; a tick while the handler
// still runs is left out; it stops as its window leaves the screen and
// starts as it is back.

namespace Clarity.Ui.Tests;

/// <summary>A clock the tests turn by hand: each Every it was asked for, and whether it was stopped.</summary>
public sealed class HandTicks : ITicks
{
    public sealed class Clock(TimeSpan every, Action tick) : IDisposable
    {
        public TimeSpan Every { get; } = every;
        public bool Stopped { get; private set; }
        public void Tick()
        {
            if (!Stopped)
                tick();
        }
        public void Dispose() => Stopped = true;
    }

    public readonly List<Clock> Clocks = [];

    public IDisposable Every(TimeSpan every, Action tick)
    {
        var clock = new Clock(every, tick);
        Clocks.Add(clock);
        return clock;
    }

    /// <summary>One tick of every clock still running.</summary>
    public void TickAll()
    {
        foreach (var clock in Clocks.ToList())
            clock.Tick();
    }
}

/// <summary>An element that takes a part in and out of its screen, as a window opened in place does.</summary>
sealed class ScreenPlace : Element
{
    public void Put(Element child) => AddChild(child);
    public void Take(Element child) => RemoveChild(child);

    public override void Arrange(Box box)
    {
        Bounds = box;
        foreach (var child in Children)
            child.Arrange(box);
    }
}

/// <summary>The tests that set the clock parts tick by run one at a time.</summary>
[CollectionDefinition("Ticks", DisableParallelization = true)]
public sealed class TicksCollection;

[Collection("Ticks")]
public class TimerPartTests
{
    static (Screen Screen, List<Part> Parts, HandTicks Ticks) Window(string markup, TestHost host)
    {
        var ticks = new HandTicks();
        Ticks.Source = ticks;
        var (screen, _, parts) = Made.Window(markup, host);
        return (screen, parts, ticks);
    }

    [Fact]
    public void It_runs_its_tick_handler_every_so_many_seconds()
    {
        var host = new TestHost();
        var (_, parts, ticks) = Window("""<Timer name="clock" at="1,1" every="0.5" ontick="clockTicked" />""", host);
        var clock = Assert.Single(ticks.Clocks);
        Assert.Equal(TimeSpan.FromSeconds(0.5), clock.Every);
        ticks.TickAll();
        ticks.TickAll();
        Assert.Equal(["run clock tick clockTicked", "run clock tick clockTicked"], host.Log);
        Assert.True(((TimerPart)parts[0]).Running);
    }

    [Fact]
    public void Its_seconds_are_one_unless_given_and_kept_between_a_tenth_and_a_day()
    {
        foreach (var (every, seconds) in new[] { ("", 1.0), ("2", 2.0), ("0", 0.1), ("0.01", 0.1), ("90000", 86400.0), ("soon", 1.0) })
        {
            var markup = every == "" ? """<Timer name="clock" at="1,1" />""" : $"""<Timer name="clock" at="1,1" every="{every}" />""";
            var (_, _, ticks) = Window(markup, new TestHost());
            Assert.Equal(TimeSpan.FromSeconds(seconds), Assert.Single(ticks.Clocks).Every);
        }
    }

    [Fact]
    public void It_has_no_place_in_the_window()
    {
        var host = new TestHost();
        var (screen, parts, _) = Window("""
            <Button name="go" at="1,1" span="3" caption="Go" onclick="went" />
            <Timer name="clock" at="1,1" span="12" ontick="clockTicked" />
            """, host);
        var timer = (TimerPart)parts[1];
        Assert.False(timer.IsShown);
        Assert.Equal(0, timer.Bounds.Width);
        Assert.Equal(0, timer.Bounds.Height);
        var go = parts[0].Bounds;
        Assert.Same(parts[0], screen.At(go.X + 4, go.Y + 4));
        screen.PointerPressed(go.X + 4, go.Y + 4, PointerButton.Left);
        screen.PointerReleased(go.X + 4, go.Y + 4, PointerButton.Left);
        Assert.Equal(["run go click went"], host.Log);
    }

    [Fact]
    public void A_disabled_timer_is_quiet_until_it_is_enabled_again()
    {
        var host = new TestHost();
        var (_, _, ticks) = Window("""<Timer name="clock" at="1,1" ontick="clockTicked" />""", host);
        host.State("clock")!.IsEnabled = false;
        ticks.TickAll();
        Assert.Empty(host.Log);
        host.State("clock")!.IsEnabled = true;
        ticks.TickAll();
        Assert.Equal(["run clock tick clockTicked"], host.Log);
    }

    sealed class SlowHost : TestHost
    {
        public Action? During;

        public override bool Run(string? part, HandlerCall call)
        {
            var ran = base.Run(part, call);
            During?.Invoke();
            return ran;
        }
    }

    [Fact]
    public void A_tick_while_its_handler_still_runs_is_left_out()
    {
        var host = new SlowHost();
        var (_, _, ticks) = Window("""<Timer name="clock" at="1,1" ontick="clockTicked" />""", host);
        host.During = ticks.TickAll;
        ticks.TickAll();
        Assert.Equal(["run clock tick clockTicked"], host.Log);
    }

    [Fact]
    public void It_stops_as_its_window_leaves_the_screen_and_starts_when_it_is_back()
    {
        var ticks = new HandTicks();
        Ticks.Source = ticks;
        var host = new TestHost();
        var spec = WindowMarkup.Read("""<Window name="Test"><Timer name="clock" at="1,1" ontick="clockTicked" /></Window>""").Parts[0];
        var timer = (TimerPart)PartKinds.Make(spec, host);
        // Not on a screen yet: no clock.
        timer.Arrange(new Box(0, 0, 100, 100));
        Assert.Empty(ticks.Clocks);
        var holder = new ScreenPlace();
        var screen = new Screen(holder, ThemeLook.Clarity(dark: false));
        holder.Put(timer);
        screen.Resize(400, 300, 1);
        ticks.TickAll();
        Assert.Single(host.Log);
        // A window opened in place over it: it stops at once.
        holder.Take(timer);
        Assert.False(timer.Running);
        Assert.True(ticks.Clocks[0].Stopped);
        ticks.TickAll();
        Assert.Single(host.Log);
        // Back on the screen: it runs again.
        holder.Put(timer);
        Assert.True(timer.Running);
        ticks.TickAll();
        Assert.Equal(2, host.Log.Count);
    }
}
