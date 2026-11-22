// Whole numbers and decimals that do not fit, as every stack treats them
// (conformance/overflow.json): generated C# works them out through Values, a
// value error in Clarity's own words, never a wrapped number.

using System.Globalization;
using System.Text.Json;
using Clarity.Runtime;
using Xunit;

namespace Clarity.Runtime.Tests;

public class OverflowTests
{
    [Fact]
    public void Numbers_that_do_not_fit_are_a_value_error()
    {
        var file = JsonDocument.Parse(File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "overflow.json"))).RootElement;
        foreach (var c in file.GetProperty("cases").EnumerateArray())
        {
            var op = c.GetProperty("op").GetString();
            var a = c.GetProperty("a").GetString()!;
            var b = c.TryGetProperty("b", out var bValue) ? bValue.GetString()! : "0";
            Func<string> work = c.GetProperty("kind").GetString() == "whole"
                ? () => Whole(op!, long.Parse(a, CultureInfo.InvariantCulture), long.Parse(b, CultureInfo.InvariantCulture)).ToString(CultureInfo.InvariantCulture)
                : () => Decimal(op!, decimal.Parse(a, CultureInfo.InvariantCulture), decimal.Parse(b, CultureInfo.InvariantCulture)).ToString(CultureInfo.InvariantCulture);
            if (c.TryGetProperty("error", out _))
            {
                var e = Assert.Throws<ValueException>(() => work());
                Assert.Equal("numberTooLarge", e.Said.Key);
                continue;
            }
            Assert.Equal(c.GetProperty("result").GetString(), work());
        }
    }

    static long Whole(string op, long a, long b) => op switch
    {
        "+" => Values.Add(a, b),
        "-" => Values.Subtract(a, b),
        "*" => Values.Multiply(a, b),
        "div" => Lists.WholeDiv(a, b),
        "mod" => Lists.WholeMod(a, b),
        _ => Values.Negate(a),
    };

    static decimal Decimal(string op, decimal a, decimal b) => op switch
    {
        "+" => Values.Add(a, b),
        "-" => Values.Subtract(a, b),
        "*" => Values.Multiply(a, b),
        _ => -a,
    };
}
