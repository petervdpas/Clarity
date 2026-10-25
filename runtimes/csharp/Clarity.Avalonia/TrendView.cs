// A trend (DESIGN 5.26), the same as on the web: the last values of one
// number, taken every so many seconds while its window is open, drawn as
// Clarity's line chart with the newest at the right.

using System.Globalization;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Threading;

namespace Clarity.Avalonia;

public class TrendView : UserControl
{
    public static readonly StyledProperty<object?> ValueProperty = AvaloniaProperty.Register<TrendView, object?>(nameof(Value));

    /// <summary>The number it follows, as the window holds it.</summary>
    public object? Value { get => GetValue(ValueProperty); set => SetValue(ValueProperty, value); }

    /// <summary>How many values it keeps: `points 60`, 60 without it.</summary>
    public int Points { get; set; } = 60;

    /// <summary>The top of the axis, `max 100`; NaN lets it take in the largest value.</summary>
    public double Max { get; set; } = double.NaN;

    /// <summary>Seconds between the values it takes: `every 0.5`, 1 without it.</summary>
    public double Every { get; set; } = 1;

    /// <summary>The values it has taken, oldest first.</summary>
    public IReadOnlyList<double> History => history;

    readonly List<double> history = [];
    readonly ChartView chart = new() { Shape = ChartShape.Line };
    DispatcherTimer? timer;

    public TrendView()
    {
        Classes.Add("trend");
        Content = chart;
    }

    /// <summary>Takes the value it follows now, keeping the last Points; a value that is no number is not taken.</summary>
    public void Take()
    {
        if (Value is null || !double.TryParse(Convert.ToString(Value, CultureInfo.InvariantCulture), NumberStyles.Float, CultureInfo.InvariantCulture, out var number))
            return;
        history.Add(number);
        if (history.Count > Math.Max(2, Points))
            history.RemoveRange(0, history.Count - Math.Max(2, Points));
        chart.Max = Max;
        chart.Numbers = history.ToList();
    }

    protected override void OnAttachedToVisualTree(VisualTreeAttachmentEventArgs e)
    {
        base.OnAttachedToVisualTree(e);
        Take();
        timer = new DispatcherTimer { Interval = TimeSpan.FromSeconds(Math.Max(0.1, Every)) };
        timer.Tick += (_, _) => Take();
        timer.Start();
    }

    protected override void OnDetachedFromVisualTree(VisualTreeAttachmentEventArgs e)
    {
        timer?.Stop();
        timer = null;
        base.OnDetachedFromVisualTree(e);
    }
}
