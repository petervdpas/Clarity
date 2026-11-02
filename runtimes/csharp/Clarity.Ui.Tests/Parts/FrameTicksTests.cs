// The clock the hosts tick the parts by (Ticks.Source): its time goes on
// with each frame of the host's loop, on the screen's thread, and a clock
// that is due runs then. A tick that fails is said and the others run on.

namespace Clarity.Ui.Tests;

public class FrameTicksTests
{
    static TimeSpan Ms(double ms) => TimeSpan.FromMilliseconds(ms);

    [Fact]
    public void A_clock_runs_when_its_time_has_come_at_a_frame()
    {
        var ticks = new FrameTicks();
        ticks.Advance(Ms(1000));
        var ran = 0;
        using var clock = ticks.Every(Ms(500), () => ran++);
        Assert.True(ticks.Running);
        ticks.Advance(Ms(1400));
        Assert.Equal(0, ran);
        ticks.Advance(Ms(1516));
        Assert.Equal(1, ran);
        ticks.Advance(Ms(1900));
        Assert.Equal(1, ran);
        ticks.Advance(Ms(2001));
        Assert.Equal(2, ran);
    }

    [Fact]
    public void Time_lost_in_a_long_frame_is_not_made_up()
    {
        var ticks = new FrameTicks();
        var ran = 0;
        using var clock = ticks.Every(Ms(100), () => ran++);
        ticks.Advance(Ms(1000));
        Assert.Equal(1, ran);
        ticks.Advance(Ms(1050));
        Assert.Equal(1, ran);
        ticks.Advance(Ms(1100));
        Assert.Equal(2, ran);
    }

    [Fact]
    public void A_stopped_clock_runs_no_more_and_the_loop_rests_without_clocks()
    {
        var ticks = new FrameTicks();
        var wanted = 0;
        ticks.Started += () => wanted++;
        var ran = 0;
        var clock = ticks.Every(Ms(100), () => ran++);
        Assert.Equal(1, wanted);
        clock.Dispose();
        ticks.Advance(Ms(500));
        Assert.Equal(0, ran);
        Assert.False(ticks.Running);
    }

    [Fact]
    public void A_tick_that_fails_is_said_and_the_others_run_on()
    {
        var ticks = new FrameTicks();
        var said = new List<string>();
        ticks.Failed = e => said.Add(e.Message);
        var ran = 0;
        using var bad = ticks.Every(Ms(100), () => throw new InvalidOperationException("broken"));
        using var good = ticks.Every(Ms(100), () => ran++);
        ticks.Advance(Ms(100));
        Assert.Equal(["broken"], said);
        Assert.Equal(1, ran);
    }

    [Fact]
    public void A_clock_started_by_a_tick_waits_for_its_own_time()
    {
        var ticks = new FrameTicks();
        var inner = 0;
        IDisposable? started = null;
        using var outer = ticks.Every(Ms(100), () => started ??= ticks.Every(Ms(100), () => inner++));
        ticks.Advance(Ms(100));
        Assert.Equal(0, inner);
        ticks.Advance(Ms(200));
        Assert.Equal(1, inner);
        started!.Dispose();
    }
}
