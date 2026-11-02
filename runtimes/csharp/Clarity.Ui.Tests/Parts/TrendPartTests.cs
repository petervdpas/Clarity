// The trend (DESIGN 5.26), as on the web: the last values of one number,
// taken every so many seconds on the screen's frame clock while it shows,
// drawn as the line chart with the newest at the right.

namespace Clarity.Ui.Tests;

public class TrendPartTests
{
    static (Screen Screen, TrendPart Trend, TestHost Host) Trend(string extra = "")
    {
        var host = new TestHost();
        var (screen, _, parts) = Made.Window($"""<Trend name="load" at="1,1" span="6" rows="4" bind="machine.load" {extra} />""", host);
        return (screen, Assert.IsType<TrendPart>(parts[0]), host);
    }

    [Fact]
    public void It_keeps_the_last_values_it_took_that_are_numbers()
    {
        var (_, trend, host) = Trend("""points="3" """);
        foreach (var value in new object?[] { 1, "2.5", null, "x", 4m, 5L })
        {
            host.Values["machine.load"] = value;
            trend.Take();
        }
        Assert.Equal([2.5, 4.0, 5.0], trend.History);
        Assert.Equal(3, trend.Points);
    }

    [Fact]
    public void It_takes_a_value_when_first_drawn_and_then_every_so_many_seconds()
    {
        var (screen, trend, host) = Trend("""every="0.5" max="100" """);
        var now = new DateTime(2026, 11, 2, 12, 0, 0);
        screen.Now = () => now;
        Assert.Equal(0.5, trend.Every);
        Assert.Equal(100, trend.Max);
        host.Values["machine.load"] = 10;
        Shot.Of(screen, 800, 400);
        Assert.Equal([10.0], trend.History);
        // It does not draw every frame: it wakes when the next value is due.
        Assert.Equal(0, screen.Animating);
        Assert.Equal(now.AddSeconds(0.5), screen.NextWake);
        host.Values["machine.load"] = 20;
        now = now.AddMilliseconds(300);
        Shot.Of(screen, 800, 400);
        Assert.Equal([10.0], trend.History);
        now = now.AddMilliseconds(200);
        var drawn = 0;
        screen.Invalidated += () => drawn++;
        screen.WakeDue(now);
        Assert.Equal(1, drawn);
        Shot.Of(screen, 800, 400);
        Assert.Equal([10.0, 20.0], trend.History);
    }

    [Fact]
    public void It_keeps_sixty_values_a_second_apart_without_points_or_every()
    {
        var (_, trend, host) = Trend();
        Assert.Equal(60, trend.Points);
        Assert.Equal(1, trend.Every);
        for (var i = 0; i < 70; i++)
        {
            host.Values["machine.load"] = i;
            trend.Take();
        }
        Assert.Equal(60, trend.History.Count);
        Assert.Equal(69, trend.History[^1]);
    }

    [Fact]
    public void Its_values_are_drawn_as_the_line_chart()
    {
        var (screen, trend, host) = Trend();
        host.Values["machine.load"] = 5;
        trend.Take();
        trend.Take();
        var shot = Shot.Of(screen, 800, 400);
        var marks = trend.Marks();
        Assert.Equal(3, marks.Count);
        Assert.All(marks, m => Assert.Equal(ChartMarkKind.Point, m.Kind));
        var (x, y) = marks[1].Center;
        Assert.Equal(ThemeLook.Clarity(dark: false).Color("accent"), shot.PixelAt((int)x, (int)y));
    }

    [Fact]
    public void A_hidden_trend_keeps_taking_values_while_its_window_is_open()
    {
        var (screen, trend, host) = Trend();
        var now = new DateTime(2026, 11, 2, 12, 0, 0);
        screen.Now = () => now;
        host.Values["machine.load"] = 5;
        host.State("load")!.IsVisible = false;
        Shot.Of(screen, 800, 400);
        Assert.Equal([5.0], trend.History);
        Assert.Equal(now.AddSeconds(1), screen.NextWake);
        host.Values["machine.load"] = 6;
        now = now.AddSeconds(1);
        screen.WakeDue(now);
        // Shown again, no value is missing.
        Assert.Equal([5.0, 6.0], trend.History);
    }
}
