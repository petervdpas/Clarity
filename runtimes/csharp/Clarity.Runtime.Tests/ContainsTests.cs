// `contains` outside a query, as every stack does it (conformance/contains.json):
// the same whatever the person's language, so it is checked under a Turkish one.

using System.Globalization;
using System.Text.Json;
using Clarity.Runtime;
using Xunit;

namespace Clarity.Runtime.Tests;

public class ContainsTests
{
    [Fact]
    public void Text_contains_a_part_case_ignored_as_every_stack_does_in_any_language()
    {
        var file = JsonDocument.Parse(File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "contains.json"))).RootElement;
        var before = CultureInfo.CurrentCulture;
        CultureInfo.CurrentCulture = new CultureInfo("tr-TR");
        try
        {
            foreach (var c in file.GetProperty("cases").EnumerateArray())
            {
                var (text, part) = (c.GetProperty("text").GetString()!, c.GetProperty("part").GetString()!);
                Assert.True(c.GetProperty("contains").GetBoolean() == Texts.Contains(text, part), $"'{text}' contains '{part}'");
            }
        }
        finally
        {
            CultureInfo.CurrentCulture = before;
        }
    }
}
