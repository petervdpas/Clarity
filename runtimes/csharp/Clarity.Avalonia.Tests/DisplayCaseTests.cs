// The display part's text and segments, against the cases every renderer
// shares (conformance/display.json).

using System.Text.Json;
using Avalonia.Controls;
using Avalonia.Headless.XUnit;
using Clarity.Avalonia;
using Xunit;

namespace Clarity.Avalonia.Tests;

public class DisplayCaseTests
{
    static readonly JsonElement File = JsonDocument.Parse(System.IO.File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "display.json"))).RootElement;

    [Fact]
    public void AValueShowsRoundedToFitItsDigitsOrAsE()
    {
        foreach (var c in File.GetProperty("shows").EnumerateArray())
            Assert.Equal(c.GetProperty("text").GetString(), SegmentDisplay.ShownText(c.GetProperty("value").GetString()!, c.GetProperty("digits").GetInt32()));
    }

    [Fact]
    public void TheTextFillsTheCellsFromTheRightAPointLitOnItsDigit()
    {
        foreach (var c in File.GetProperty("cells").EnumerateArray())
        {
            var expected = c.GetProperty("cells").EnumerateArray().Select(x => (x[0].GetString()![0], x[1].GetBoolean())).ToArray();
            Assert.Equal(expected, SegmentDisplay.Cells(c.GetProperty("text").GetString()!, c.GetProperty("digits").GetInt32()));
        }
    }

    [Fact]
    public void EachCharacterLightsItsSegments()
    {
        foreach (var lit in File.GetProperty("segments").EnumerateObject())
            Assert.Equal(lit.Value.GetString(), SegmentDisplay.Segments[lit.Name[0]]);
    }

    [AvaloniaFact]
    public void ADisplayShowsItsValueInAWindow()
    {
        var display = new SegmentDisplay { Value = 12.5m, Digits = 4, Width = 200, Height = 60 };
        var window = new Window { Content = display };
        window.Show();
        Assert.Equal("12.5", display.Text);
        display.Value = 123456m;
        Assert.Equal("E", display.Text);
        display.Value = null;
        Assert.Equal("0", display.Text);
    }
}
