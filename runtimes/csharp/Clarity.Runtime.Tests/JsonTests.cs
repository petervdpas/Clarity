// Reading JSON into records (DESIGN 5.23) against the cases every runtime
// shares (conformance/json.json): what is read is fitted to the entity.

using System.Text.Json;
using System.Text.Json.Nodes;
using Clarity.Runtime;
using Xunit;

namespace Clarity.Runtime.Tests;

public sealed class JsonTests
{
    static JsonKind KindOf(JsonElement field, string word) => new(
        word,
        field.TryGetProperty("values", out var values) ? values.EnumerateArray().Select(v => v.GetString()!).ToArray() : null,
        word == "values" ? KindOf(field, field.GetProperty("item").GetString()!) : null,
        word is "record" or "records" ? field.GetProperty("entity").GetString() : null,
        word is "record" or "records" ? FieldsOf(field.GetProperty("fields")) : null,
        word is "record" or "records" ? JsonNode.Parse(field.GetProperty("base").GetRawText()) : null);

    static JsonField[] FieldsOf(JsonElement list) =>
        list.EnumerateArray().Select(f => new JsonField(f.GetProperty("name").GetString()!, KindOf(f, f.GetProperty("kind").GetString()!))).ToArray();

    [Fact]
    public void JsonIsFittedAsEveryRuntimeFitsIt()
    {
        var file = JsonDocument.Parse(File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "json.json"))).RootElement;
        foreach (var c in file.GetProperty("cases").EnumerateArray())
        {
            var name = c.GetProperty("name").GetString();
            var fields = FieldsOf(c.GetProperty("fields"));
            var at = c.TryGetProperty("at", out var a) ? a.GetString() : null;
            var many = c.TryGetProperty("many", out var m) && m.GetBoolean();
            var template = JsonNode.Parse(c.GetProperty("base").GetRawText());
            string text = c.GetProperty("json").GetString()!, entity = c.GetProperty("entity").GetString()!;
            if (c.TryGetProperty("error", out var error))
            {
                var thrown = Assert.Throws<ValueException>(() => JsonValues.Fit(text, at, many, entity, fields, template));
                Assert.True(thrown.Message == error.GetString(), $"{name}: {thrown.Message}");
                continue;
            }
            var fitted = JsonValues.Fit(text, at, many, entity, fields, template);
            Assert.True(JsonNode.DeepEquals(fitted, JsonNode.Parse(c.GetProperty("fitted").GetRawText())), $"{name}: {fitted!.ToJsonString()}");
        }
    }

    sealed class Order
    {
        public long Id { get; set; }
        public string Number { get; set; } = "new";
        public decimal CreditLimit { get; set; }
        public DateOnly? Since { get; set; }
    }

    /// <summary>Records written and read as Clarity's JSON: snake case, decimals as text.</summary>
    [Fact]
    public void RecordsTravelAsClaritysJson()
    {
        var order = new Order { Id = 4, Number = "A-1", CreditLimit = 12.50m, Since = new DateOnly(2026, 10, 25) };
        Assert.Equal("{\"id\":4,\"number\":\"A-1\",\"credit_limit\":\"12.50\",\"since\":\"2026-10-25\"}", JsonValues.Of(order));
        var fields = new[] { new JsonField("number", new JsonKind("text")), new JsonField("credit_limit", new JsonKind("decimal")), new JsonField("since", new JsonKind("date")) };
        var read = JsonValues.ReadList("{\"orders\": [{\"creditLimit\": 99.95}]}", "orders", "Order", fields, new Order());
        Assert.Equal(("new", 99.95m, (DateOnly?)null, 0L), (read[0].Number, read[0].CreditLimit, read[0].Since, read[0].Id));
    }
}
