// Lists of values, against the cases every stack's runtime shares
// (conformance/lists.json).

using System.Text.Json;
using Clarity.Runtime;
using Xunit;

namespace Clarity.Runtime.Tests;

public class ListTests
{
    static JsonElement Cases() =>
        JsonDocument.Parse(File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "lists.json"))).RootElement;

    [Fact]
    public void AnItemIsReadByItsPositionFrom1()
    {
        foreach (var c in Cases().GetProperty("items").EnumerateArray())
        {
            var list = c.GetProperty("list").EnumerateArray().Select(v => v.GetInt64()).ToList();
            var at = c.GetProperty("at").GetInt64();
            if (c.TryGetProperty("gives", out var gives))
                Assert.Equal(gives.GetInt64(), Lists.At(list, at));
            else
                Assert.Equal(c.GetProperty("fails").GetString(), Assert.Throws<ValueException>(() => Lists.At(list, at)).Message);
        }
    }

    [Fact]
    public void AnItemIsSetByItsPositionFrom1()
    {
        var list = new List<long> { 10, 20, 30 };
        Lists.Set(list, 2, 99L);
        Assert.Equal([10L, 99L, 30L], list);
        Assert.Equal("there is no item 4 in a list of 3", Assert.Throws<ValueException>(() => Lists.Set(list, 4, 1L)).Message);
    }
}
