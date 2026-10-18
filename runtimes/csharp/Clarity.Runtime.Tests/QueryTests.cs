// The SQL of queries per database, against the cases every stack's runtime
// shares (conformance/queries.json).

using System.Globalization;
using System.Text.Json;
using Clarity.Runtime;
using Xunit;

namespace Clarity.Runtime.Tests;

public class QueryTests
{
    static readonly (string Word, Dialect Dialect)[] Dialects =
    [
        ("sqlite", Dialect.Sqlite),
        ("postgres", Dialect.Postgres),
        ("mysql", Dialect.Mysql),
        ("mssql", Dialect.Mssql),
    ];

    static JsonElement Cases()
    {
        var path = Path.Combine(AppContext.BaseDirectory, "conformance", "queries.json");
        return JsonDocument.Parse(File.ReadAllText(path)).RootElement.GetProperty("cases");
    }

    static Op ToOp(JsonElement value) => value.GetString() switch
    {
        "Eq" => Op.Eq,
        "NotEq" => Op.NotEq,
        "Lt" => Op.Lt,
        "LtEq" => Op.LtEq,
        "Gt" => Op.Gt,
        "GtEq" => Op.GtEq,
        var other => throw new ArgumentException($"unknown op {other}"),
    };

    static object? ToValue(JsonElement value)
    {
        var field = value.EnumerateObject().Single();
        return field.Name switch
        {
            "text" => field.Value.GetString(),
            "integer" => field.Value.GetInt64(),
            "decimal" => decimal.Parse(field.Value.GetString()!, CultureInfo.InvariantCulture),
            "bool" => field.Value.GetBoolean(),
            "date" => DateOnly.ParseExact(field.Value.GetString()!, "yyyy-MM-dd", CultureInfo.InvariantCulture),
            "null" => null,
            var other => throw new ArgumentException($"unknown value {other}"),
        };
    }

    static List<object?> ToValues(JsonElement values) => values.EnumerateArray().Select(ToValue).ToList();

    static Cond ToCond(JsonElement value)
    {
        var field = value.EnumerateObject().Single();
        var inner = field.Value;
        string Column() => inner.GetProperty("column").GetString()!;
        (Cond, Cond) Pair()
        {
            var both = inner.EnumerateArray().ToArray();
            return (ToCond(both[0]), ToCond(both[1]));
        }
        switch (field.Name)
        {
            case "compare":
                return Cond.Compare(Column(), ToOp(inner.GetProperty("op")), ToValue(inner.GetProperty("value")));
            case "number":
                return Cond.Number(Column(), ToOp(inner.GetProperty("op")), ToValue(inner.GetProperty("value")));
            case "contains":
                return Cond.Contains(Column(), inner.GetProperty("text").GetString()!);
            case "oneOf":
                return Cond.OneOf(Column(), ToValues(inner.GetProperty("values")));
            case "oneOfNumbers":
                return Cond.OneOfNumbers(Column(), ToValues(inner.GetProperty("values")));
            case "isTrue":
                return Cond.IsTrue(inner.GetString()!);
            case "isNone":
                return Cond.IsNone(inner.GetString()!);
            case "and":
            {
                var (left, right) = Pair();
                return left.And(right);
            }
            case "or":
            {
                var (left, right) = Pair();
                return left.Or(right);
            }
            case "not":
                return ToCond(inner).Negated();
            case "through":
                return Cond.Through(Column(), inner.GetProperty("table").GetString()!, inner.GetProperty("key").GetString()!, ToCond(inner.GetProperty("filter")));
            default:
                throw new ArgumentException($"unknown condition {field.Name}");
        }
    }

    static Query ToQuery(JsonElement value)
    {
        var query = new Query();
        if (value.TryGetProperty("filter", out var filter))
            query = query.Filter(ToCond(filter));
        if (value.TryGetProperty("order", out var order))
        {
            foreach (var key in order.EnumerateArray())
            {
                var column = key.GetProperty("column").GetString()!;
                var descending = key.GetProperty("descending").GetBoolean();
                var numeric = key.GetProperty("numeric").GetBoolean();
                if (key.TryGetProperty("through", out var through))
                {
                    var hops = through.EnumerateArray()
                        .Select(h => new Hop(h.GetProperty("column").GetString()!, h.GetProperty("table").GetString()!, h.GetProperty("key").GetString()!))
                        .ToArray();
                    query = query.OrderThrough(hops, column, descending, numeric);
                }
                else
                {
                    query = numeric ? query.OrderNumber(column, descending) : query.Order(column, descending);
                }
            }
        }
        return query;
    }

    [Fact]
    public void EveryCaseGivesItsSqlAndParametersOnEveryDatabase()
    {
        var cases = Cases().EnumerateArray().ToList();
        Assert.NotEmpty(cases);
        foreach (var @case in cases)
        {
            var name = @case.GetProperty("name").GetString();
            var key = @case.GetProperty("key").GetString()!;
            var table = @case.GetProperty("table").GetString()!;
            var query = ToQuery(@case.GetProperty("query"));
            foreach (var (word, dialect) in Dialects)
            {
                var expect = @case.GetProperty("expect").GetProperty(word);
                var columns = new[] { key }
                    .Concat(@case.GetProperty("columns").EnumerateArray().Select(c => c.GetString()!))
                    .Select(c => dialect.Quote(c));
                var (sql, parameters) = query.Sql(dialect, table, key, string.Join(", ", columns));
                Assert.True(expect.GetProperty("sql").GetString() == sql, $"{name} on {word}: {sql}");
                Assert.Equal(ToValues(expect.GetProperty("params")), parameters);
            }
        }
    }
}
