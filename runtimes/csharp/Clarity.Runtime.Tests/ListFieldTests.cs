// Lists of values as fields (U1c), kept in one column as JSON, against the
// cases every runtime shares (conformance/list-fields.json).

using System.Text.Json;
using Clarity.Runtime;
using Xunit;

namespace Clarity.Runtime.Tests;

public enum Genre { Jazz, Rock }

public class ListFieldTests
{
    static IEnumerable<JsonElement> Cases(string part) =>
        JsonDocument.Parse(File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "list-fields.json"))).RootElement.GetProperty(part).EnumerateArray();

    static string? Column(JsonElement c) => c.GetProperty("column").ValueKind == JsonValueKind.Null ? null : c.GetProperty("column").GetString();

    /// <summary>The case's items as the item type of its list.</summary>
    static string Kept(JsonElement c)
    {
        var items = c.GetProperty("items").EnumerateArray();
        return c.GetProperty("item").GetString() switch
        {
            "text" => ListFields.Kept(items.Select(i => i.GetString()!)),
            "integer" => ListFields.Kept(items.Select(i => i.GetInt64())),
            "decimal" => ListFields.Kept(items.Select(i => decimal.Parse(i.GetString()!, System.Globalization.CultureInfo.InvariantCulture))),
            "date" => ListFields.Kept(items.Select(i => (DateOnly?)DateOnly.Parse(i.GetString()!, System.Globalization.CultureInfo.InvariantCulture))),
            _ => ListFields.Kept(items.Select(i => Enum.Parse<Genre>(i.GetString()!))),
        };
    }

    /// <summary>The column read as the item type, back as JSON to compare.</summary>
    static string Read(JsonElement c)
    {
        var column = Column(c);
        return c.GetProperty("item").GetString() switch
        {
            "text" => ListFields.Kept(ListFields.Read<string>("tags", column)),
            "integer" => ListFields.Kept(ListFields.Read<long>("tags", column)),
            "decimal" => ListFields.Kept(ListFields.Read<decimal>("tags", column)),
            "date" => ListFields.Kept(ListFields.Read<DateOnly?>("tags", column)),
            _ => ListFields.Kept(ListFields.Read<Genre>("tags", column)),
        };
    }

    [Fact]
    public void AListIsKeptAsCompactJson()
    {
        foreach (var c in Cases("kept"))
            Assert.Equal(c.GetProperty("column").GetString(), Kept(c));
    }

    [Fact]
    public void AColumnGivesItsItems()
    {
        foreach (var c in Cases("read"))
        {
            var items = c.GetProperty("items").EnumerateArray().Select(i => i.ValueKind == JsonValueKind.Number ? i.GetRawText() : JsonSerializer.Serialize(i.GetString(), new JsonSerializerOptions { Encoder = System.Text.Encodings.Web.JavaScriptEncoder.UnsafeRelaxedJsonEscaping }));
            Assert.Equal("[" + string.Join(",", items) + "]", Read(c));
        }
    }

    [Fact]
    public void AColumnThatIsNoListIsRefused() =>
        Assert.Throws<InvalidValueException>(() => ListFields.Read<long>("tags", "[\"x\"]"));
}
