// round(value, places) in code, against the cases every stack's runtime
// shares (conformance/rounding.json).

using System.Globalization;
using System.Text.Json;
using Clarity.Runtime;
using Xunit;

namespace Clarity.Runtime.Tests;

public class RoundingTests
{
    [Fact]
    public void EveryRoundingCaseRoundsAsEveryRuntimeDoes()
    {
        var path = Path.Combine(AppContext.BaseDirectory, "conformance", "rounding.json");
        var cases = JsonDocument.Parse(File.ReadAllText(path)).RootElement.GetProperty("cases");
        Assert.True(cases.GetArrayLength() > 0);
        foreach (var c in cases.EnumerateArray())
        {
            var value = decimal.Parse(c.GetProperty("value").GetString()!, CultureInfo.InvariantCulture);
            var gives = decimal.Parse(c.GetProperty("gives").GetString()!, CultureInfo.InvariantCulture);
            Assert.Equal(gives, Numbers.Round(value, c.GetProperty("places").GetInt64()));
        }
    }
}

public class AngleTests
{
    [Fact]
    public void SinAndCosTakeDegreesAsEveryRuntimeDoes()
    {
        var path = Path.Combine(AppContext.BaseDirectory, "conformance", "drawing.json");
        var angles = JsonDocument.Parse(File.ReadAllText(path)).RootElement.GetProperty("angles");
        Assert.True(angles.GetArrayLength() > 0);
        foreach (var c in angles.EnumerateArray())
        {
            var degrees = decimal.Parse(c.GetProperty("degrees").GetString()!, CultureInfo.InvariantCulture);
            Assert.Equal(decimal.Parse(c.GetProperty("sin").GetString()!, CultureInfo.InvariantCulture), Numbers.Sin(degrees));
            Assert.Equal(decimal.Parse(c.GetProperty("cos").GetString()!, CultureInfo.InvariantCulture), Numbers.Cos(degrees));
        }
    }
}
