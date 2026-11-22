// A date compared with an empty one, as every stack compares them
// (conformance/compare-empty.json): generated C# orders dates through Values.Compare.

using System.Globalization;
using System.Text.Json;
using Clarity.Runtime;
using Xunit;

namespace Clarity.Runtime.Tests;

public class CompareEmptyTests
{
    [Fact]
    public void An_empty_date_is_smallest_as_every_stack_compares_it()
    {
        var file = JsonDocument.Parse(File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "compare-empty.json"))).RootElement;
        foreach (var c in file.GetProperty("cases").EnumerateArray())
        {
            string? Text(string side) => c.GetProperty(side).ValueKind == JsonValueKind.Null ? null : c.GetProperty(side).GetString();
            int order = c.GetProperty("kind").GetString() == "date"
                ? Values.Compare(Text("a") is { } a ? DateOnly.ParseExact(a, "yyyy-MM-dd", CultureInfo.InvariantCulture) : null, Text("b") is { } b ? DateOnly.ParseExact(b, "yyyy-MM-dd", CultureInfo.InvariantCulture) : null)
                : Values.Compare(Text("a") is { } x ? DateTime.ParseExact(x, "yyyy-MM-ddTHH:mm:ss", CultureInfo.InvariantCulture) : null, Text("b") is { } y ? DateTime.ParseExact(y, "yyyy-MM-ddTHH:mm:ss", CultureInfo.InvariantCulture) : null);
            var got = new Dictionary<string, bool> { ["<"] = order < 0, ["<="] = order <= 0, [">"] = order > 0, [">="] = order >= 0 };
            foreach (var op in got.Keys)
                Assert.True(c.GetProperty(op).GetBoolean() == got[op], $"{Text("a") ?? "none"} {op} {Text("b") ?? "none"}");
        }
    }
}
