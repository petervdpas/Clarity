// Charts against the cases every renderer shares (conformance/charts.json),
// and through the real ChartView: its rows grouped and drawn, a bar, point
// or slice telling its group and number.

using System.Collections.ObjectModel;
using System.Text.Json;
using Avalonia.Controls.Shapes;
using Path = System.IO.Path;
using PathShape = Avalonia.Controls.Shapes.Path;
using Avalonia.Headless.XUnit;
using Clarity.Avalonia;
using Xunit;

namespace Clarity.Avalonia.Tests;

public class ChartTests
{
    static readonly JsonElement File = JsonDocument.Parse(System.IO.File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "charts.json"))).RootElement;

    static object? Value(JsonElement e) => e.ValueKind switch
    {
        JsonValueKind.Null => null,
        JsonValueKind.Number => e.GetDouble(),
        JsonValueKind.True => true,
        JsonValueKind.False => false,
        _ => e.GetString(),
    };

    [AvaloniaFact]
    public void Rows_group_count_sum_and_average_as_on_every_renderer()
    {
        foreach (var c in File.GetProperty("groups").EnumerateArray())
        {
            var by = c.GetProperty("by").GetString() switch
            {
                "number" => ChartBy.Number,
                "date" => ChartBy.Date,
                "enum" => ChartBy.Enum,
                "bool" => ChartBy.Bool,
                _ => ChartBy.Text,
            };
            var measure = c.GetProperty("measure").GetString() switch { "sum" => ChartMeasure.Sum, "average" => ChartMeasure.Average, _ => ChartMeasure.Count };
            List<object>? order = c.TryGetProperty("order", out var o) ? [.. o.EnumerateArray().Select(v => (object)v.GetString()!)] : null;
            var rows = c.GetProperty("rows").EnumerateArray().Select(r => (Value(r.GetProperty("by")), Value(r.GetProperty("of"))));
            var got = Charts.Groups(rows, by, measure, order).Select(g => $"{g.Key}={g.Value}").ToList();
            var want = c.GetProperty("gives").EnumerateArray().Select(g => $"{Value(g.GetProperty("key"))}={g.GetProperty("value").GetDouble()}").ToList();
            Assert.True(want.SequenceEqual(got), $"{c.GetProperty("name").GetString()}: {string.Join(", ", got)}");
        }
    }

    [AvaloniaFact]
    public void The_axis_and_the_slices_are_as_on_every_renderer()
    {
        foreach (var c in File.GetProperty("scale").EnumerateArray())
            Assert.Equal(new ChartScale(c.GetProperty("bottom").GetDouble(), c.GetProperty("top").GetDouble(), c.GetProperty("step").GetDouble()), Charts.Scale(c.GetProperty("low").GetDouble(), c.GetProperty("high").GetDouble()));
        foreach (var c in File.GetProperty("slices").EnumerateArray())
            Assert.Equal(c.GetProperty("colors").EnumerateArray().Select(v => v.ValueKind == JsonValueKind.Null ? null : v.GetString()), Charts.SliceColors(c.GetProperty("values").EnumerateArray().Select(v => v.GetDouble())));
    }

    public enum Status { Open, Paid, Sent }

    public sealed class Order
    {
        public Status Status { get; set; }
        public decimal Total { get; set; }
    }

    [AvaloniaFact]
    public void A_chart_draws_its_rows_grouped_and_again_when_they_change()
    {
        var orders = new ObservableCollection<Order> { new() { Status = Status.Sent, Total = 5 }, new() { Status = Status.Open, Total = 2.5m }, new() { Status = Status.Sent, Total = 1 } };
        var chart = new ChartView { Shape = ChartShape.Bar, By = "Status", ByKind = ChartBy.Enum, Number = ChartMeasure.Sum, Of = "Total", Width = 300, Height = 200 };
        chart.ItemsSource = orders;
        Assert.Equal([new ChartShown("Open", 2.5, (2.5m).ToString(System.Globalization.CultureInfo.CurrentCulture)), new ChartShown("Sent", 6, 6m.ToString(System.Globalization.CultureInfo.CurrentCulture))], chart.Shown);
        Assert.Equal(2, chart.Drawing.Children.OfType<Rectangle>().Count(r => r.Classes.Contains("bar")));
        orders.Add(new Order { Status = Status.Paid, Total = 1 });
        Assert.Equal(["Open", "Paid", "Sent"], chart.Shown.Select(g => g.Label));
        chart.Shape = ChartShape.Pie;
        chart.ItemsSource = orders.ToList();
        Assert.Equal(3, chart.Drawing.Children.OfType<PathShape>().Count(p => p.Classes.Contains("slice")));
    }
}
