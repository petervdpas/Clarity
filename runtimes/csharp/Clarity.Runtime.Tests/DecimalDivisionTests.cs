// Dividing decimals as every stack divides them (conformance/decimal-division.json):
// generated C# divides through Values.Divide, a value error for dividing by zero.

using System.Globalization;
using System.Text.Json;
using Clarity.Runtime;
using Xunit;

namespace Clarity.Runtime.Tests;

public class DecimalDivisionTests
{
    [Fact]
    public void Decimals_divide_as_every_stack_divides_them()
    {
        var file = JsonDocument.Parse(File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "decimal-division.json"))).RootElement;
        foreach (var c in file.GetProperty("cases").EnumerateArray())
        {
            var a = decimal.Parse(c.GetProperty("a").GetString()!, CultureInfo.InvariantCulture);
            var b = decimal.Parse(c.GetProperty("b").GetString()!, CultureInfo.InvariantCulture);
            if (c.TryGetProperty("error", out _))
            {
                Assert.Equal("divisionByZero", Assert.Throws<ValueException>(() => Values.Divide(a, b)).Said.Key);
                continue;
            }
            Assert.Equal(c.GetProperty("quotient").GetString(), Values.Divide(a, b).ToString(CultureInfo.InvariantCulture));
        }
    }
}
