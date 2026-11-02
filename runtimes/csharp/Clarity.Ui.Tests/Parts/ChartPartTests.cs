// The chart against the cases every renderer shares (conformance/charts.json),
// and as a part: a list's rows grouped by a field and drawn as bars, a line
// or a pie, again when the rows change; a list of numbers drawn as they are;
// a bar, point or slice telling its group and number when pointed at.

using System.Text.Json;
using Clarity.Runtime;

namespace Clarity.Ui.Tests;

public class ChartPartTests
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

    [Fact]
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

    [Fact]
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
        public long Customer { get; set; }
    }

    static TestHost Orders(params Order[] orders)
    {
        var host = new TestHost();
        host.Lists["orders"] = [.. orders];
        return host;
    }

    const string Summed = """<Chart name="totals" at="1,1" span="6" rows="6" bind="orders" by="status" measure="{&quot;kind&quot;:&quot;sum&quot;,&quot;field&quot;:&quot;total&quot;}" CHART />""";

    static ChartPart Chart(TestHost host, string shape = "bar", string markup = Summed)
    {
        var (_, _, parts) = Made.Window(markup.Replace("CHART", $"chart=\"{shape}\""), host);
        return Assert.IsType<ChartPart>(parts[0]);
    }

    [Fact]
    public void A_chart_groups_its_rows_and_groups_them_again_when_they_change()
    {
        var host = Orders(new Order { Status = Status.Sent, Total = 5 }, new Order { Status = Status.Open, Total = 2.5m }, new Order { Status = Status.Sent, Total = 1 });
        var chart = Chart(host);
        Assert.Equal(ChartShape.Bar, chart.Shape);
        Assert.Equal(["Open", "Sent"], chart.Groups().Select(g => g.Label));
        Assert.Equal([2.5, 6.0], chart.Groups().Select(g => g.Value));
        Assert.Equal(ShownText.Of(6m), chart.Groups()[1].Text);
        host.Lists["orders"].Add(new Order { Status = Status.Paid, Total = 1 });
        host.Change();
        Assert.Equal(["Open", "Paid", "Sent"], chart.Groups().Select(g => g.Label));
    }

    [Fact]
    public void A_group_shows_by_the_label_its_choice_has_and_a_reference_by_its_text()
    {
        var host = Orders(new Order { Status = Status.Open, Customer = 2 }, new Order { Status = Status.Paid, Customer = 1 }, new Order { Status = Status.Open, Customer = 1 });
        host.Offered["orders.status"] = [new(Status.Open, "Still open"), new(Status.Paid, "Paid up"), new(Status.Sent, "On its way")];
        host.Offered["orders.customer"] = [new(1L, "Zeno"), new(2L, "Ada")];
        var byStatus = Chart(host, markup: """<Chart at="1,1" span="6" rows="6" bind="orders" by="status" CHART />""");
        Assert.Equal(["Still open", "Paid up"], byStatus.Groups().Select(g => g.Label));
        Assert.Equal(["2", "1"], byStatus.Groups().Select(g => g.Text));
        var byCustomer = Chart(host, markup: """<Chart at="1,1" span="6" rows="6" bind="orders" by="customer" CHART />""");
        Assert.Equal(["Ada", "Zeno"], byCustomer.Groups().Select(g => g.Label));
        Assert.Equal([1.0, 2.0], byCustomer.Groups().Select(g => g.Value));
    }

    [Fact]
    public void A_chart_of_numbers_shows_its_last_points()
    {
        var shown = ChartPart.NumbersShown(new object?[] { "1.5", 2L, "x", null, 3m }, 2);
        Assert.Equal([2.0, 3.0], shown.Select(s => s.Value));
        Assert.All(shown, s => Assert.Equal("", s.Label));
        Assert.Equal(3, ChartPart.NumbersShown(new object?[] { 1, 2, 3 }, 0).Count);

        var host = new TestHost();
        host.Values["readings"] = new List<double> { 4, 5, 6, 7 };
        var chart = Chart(host, "line", """<Chart at="1,1" span="6" rows="6" bind="readings" points="3" max="10" CHART />""");
        Assert.Equal([5.0, 6.0, 7.0], chart.Groups().Select(g => g.Value));
        Assert.Equal(10, chart.Max);
    }

    static SKBitmap Drawn(ChartPart chart) => Shot.Of(chart.Screen!, 800, 400);

    static bool Has(SKBitmap shot, Box box, SKColor color)
    {
        for (var x = (int)box.X; x < (int)box.Right; x++)
            for (var y = (int)box.Y; y < (int)box.Bottom; y++)
                if (shot.PixelAt(x, y) == color)
                    return true;
        return false;
    }

    [Fact]
    public void Bars_are_drawn_in_the_accent_and_slices_in_the_palette()
    {
        var host = Orders(new Order { Status = Status.Sent, Total = 5 }, new Order { Status = Status.Open, Total = 2.5m });
        var bars = Chart(host);
        var accent = ThemeLook.Clarity(dark: false).Color("accent");
        var shot = Drawn(bars);
        Assert.True(Has(shot, bars.Bounds, accent));
        var marks = bars.Marks(bars.Bounds);
        Assert.Equal(2, marks.Count);
        Assert.All(marks, m => Assert.Equal(accent, shot.PixelAt((int)(m.Box.X + m.Box.Width / 2), (int)(m.Box.Bottom - 2))));

        var pie = Chart(host, "pie");
        var pieShot = Drawn(pie);
        Assert.True(Has(pieShot, pie.Bounds, SKColor.Parse("#64748b")));
        Assert.True(Has(pieShot, pie.Bounds, SKColor.Parse("#ef4444")));
        Assert.False(Has(pieShot, pie.Bounds, accent));
    }

    [Fact]
    public void A_line_of_many_numbers_has_no_dot_on_each()
    {
        var host = new TestHost();
        host.Values["readings"] = Enumerable.Range(1, 40).Select(i => (double)i).ToList();
        var chart = Chart(host, "line", """<Chart at="1,1" span="6" rows="6" bind="readings" max="100" CHART />""");
        Drawn(chart);
        Assert.Equal(0, chart.Marks(chart.Bounds).Count(m => m.Dot));
        host.Values["readings"] = new List<double> { 1, 2, 3 };
        Assert.Equal(3, chart.Marks(chart.Bounds).Count(m => m.Dot));
    }

    [Fact]
    public void A_bar_point_or_slice_tells_its_group_and_number_when_pointed_at()
    {
        var host = Orders(new Order { Status = Status.Sent, Total = 5 }, new Order { Status = Status.Open, Total = 2.5m });
        foreach (var shape in new[] { "bar", "line", "pie" })
        {
            var chart = Chart(host, shape);
            Drawn(chart);
            var open = chart.Marks(chart.Bounds)[0];
            var (x, y) = open.Center;
            Assert.Equal($"Open: {ShownText.Of(2.5m)}", chart.TipAt(x, y));
            Assert.Null(chart.TipAt(chart.Bounds.X + 1, chart.Bounds.Y + 1));
            chart.Screen!.PointerMoved(x, y);
            Assert.Equal($"Open: {ShownText.Of(2.5m)}", chart.Tip);
            // It shows as the screen's tooltip once the pointer rested, as Avalonia's ToolTip did; not at once.
            Assert.Null(chart.Screen.Tip);
            chart.Screen.WakeDue(chart.Screen.Now() + Screen.TipPause);
            Assert.Equal($"Open: {ShownText.Of(2.5m)}", chart.Screen.Tip?.Text);
        }
    }

    [Fact]
    public void A_hidden_chart_draws_nothing()
    {
        var host = Orders(new Order { Status = Status.Sent, Total = 5 });
        var chart = Chart(host);
        host.State("totals")!.IsVisible = false;
        var shot = Drawn(chart);
        Assert.False(Has(shot, chart.Bounds, ThemeLook.Clarity(dark: false).Color("accent")));
    }
}
