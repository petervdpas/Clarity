// Live graphs (DESIGN 5.26): a chart of a list of numbers, a trend that
// keeps the last values of one number, and a timer that runs its handler
// while it is enabled, as on the web.

using System.Windows.Input;
using Avalonia.Controls;
using Avalonia.Controls.Shapes;
using Avalonia.Headless.XUnit;
using Clarity.Avalonia;
using Xunit;

namespace Clarity.Avalonia.Tests;

public class LiveChartTests
{
    sealed class Counted : ICommand
    {
        public int Runs;
        public event EventHandler? CanExecuteChanged { add { } remove { } }
        public bool CanExecute(object? parameter) => true;
        public void Execute(object? parameter) => Runs++;
    }

    [AvaloniaFact]
    public void A_chart_of_numbers_shows_its_last_points()
    {
        var shown = ChartView.NumbersShown(new object[] { "1.5", 2L, "x", 3m }, 2);
        Assert.Equal([2.0, 3.0], shown.Select(s => s.Value));
        Assert.All(shown, s => Assert.Equal("", s.Label));
        Assert.Equal(3, ChartView.NumbersShown(new[] { 1, 2, 3 }, 0).Count);
    }

    [AvaloniaFact]
    public void A_chart_of_many_numbers_draws_a_line_without_dots()
    {
        var chart = new ChartView { Shape = ChartShape.Line, Numbers = Enumerable.Range(1, 40).ToList(), Max = 100 };
        var window = new Window { Content = chart, Width = 400, Height = 300 };
        window.Show();
        Assert.Single(chart.Drawing.Children.OfType<Polyline>());
        Assert.Empty(chart.Drawing.Children.OfType<Ellipse>());
        window.Close();
    }

    [AvaloniaFact]
    public void A_trend_keeps_the_last_values_it_took()
    {
        var trend = new TrendView { Points = 3 };
        foreach (var value in new object?[] { 1, "2.5", null, "x", 4, 5 })
        {
            trend.Value = value;
            trend.Take();
        }
        Assert.Equal([2.5, 4.0, 5.0], trend.History);
    }

    [AvaloniaFact]
    public void A_timer_ticks_only_while_it_is_enabled()
    {
        var handler = new Counted();
        var timer = new Clarity.Avalonia.Timer { Command = handler, Every = 1 };
        timer.Tick();
        timer.IsEnabled = false;
        timer.Tick();
        timer.IsEnabled = true;
        timer.Tick();
        Assert.Equal(2, handler.Runs);
    }
}
