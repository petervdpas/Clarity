// A value written into JSON as every stack writes it for parts, reports, the
// wire and `json of` (conformance/values-json.json). C# writes values three
// ways: Clarity's JSON (JsonValues), a part's value on the wire (WireJson) and
// a report's data from `pdf of ... with` (PartValues); all three write the same.

using System.Globalization;
using System.Text.Json;
using Clarity.Runtime;
using Xunit;

namespace Clarity.Runtime.Tests;

public class ValuesJsonTests
{
    /// <summary>A record as a generated one holds its fields, named as the project wrote them.</summary>
    public sealed class Invoice
    {
        public string CustomerName { get; set; } = "";
        public decimal Total { get; set; }
        public DateOnly? Due { get; set; }
        public long? Customer { get; set; }
        public List<long> Lines { get; set; } = [];
        public Kind Kind { get; set; }
    }

    public enum Kind { Person, Business }

    /// <summary>The value a generated record holds for a field of <paramref name="kind"/>.</summary>
    static object? ValueOf(string kind, JsonElement value)
    {
        string? text = value.ValueKind == JsonValueKind.String ? value.GetString() : null;
        return kind switch
        {
            "decimal" or "money" => decimal.Parse(text!, CultureInfo.InvariantCulture),
            "integer" => long.Parse(text!, CultureInfo.InvariantCulture),
            "date" => text is null ? null : DateOnly.ParseExact(text, "yyyy-MM-dd", CultureInfo.InvariantCulture),
            "datetime" => text is null ? null : DateTime.ParseExact(text, "yyyy-MM-ddTHH:mm:ss", CultureInfo.InvariantCulture),
            "ref" => text is null ? null : long.Parse(text, CultureInfo.InvariantCulture),
            "bool" => value.GetBoolean(),
            "text" => text,
            "list" => new List<long>(),
            _ => throw new ArgumentException(kind),
        };
    }

    [Fact]
    public void Every_value_is_written_as_every_stack_writes_it_three_ways_alike()
    {
        var file = JsonDocument.Parse(File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "values-json.json"))).RootElement;
        foreach (var c in file.GetProperty("cases").EnumerateArray())
        {
            var name = c.GetProperty("name").GetString();
            var want = c.GetProperty("json").GetString();
            var value = ValueOf(c.GetProperty("kind").GetString()!, c.GetProperty("value"));
            Assert.True(want == JsonValues.Of(value), $"{name}: JsonValues {JsonValues.Of(value)}");
            Assert.True(want == (WireJson.Value(value)?.ToJsonString(JsonValues.Options) ?? "null"), $"{name}: WireJson");
            Assert.True(want == PartValues.Write(value), $"{name}: PartValues {PartValues.Write(value)}");
        }
    }

    [Fact]
    public void A_record_given_to_a_report_is_written_as_when_it_is_given_to_a_part()
    {
        var invoice = new Invoice { CustomerName = "Ada", Total = 1250.50m, Due = new DateOnly(2026, 11, 21), Customer = 7, Kind = Kind.Business };
        var part = WireJson.Value(invoice)!.ToJsonString(JsonValues.Options);
        Assert.Equal(part, PartValues.Write(invoice));
        Assert.Contains("\"customerName\":\"Ada\"", part);
        Assert.Contains("\"total\":\"1250.50\"", part);
        Assert.Contains("\"kind\":\"Business\"", part);
        Assert.Contains("\"lines\":[]", part);
    }
}
