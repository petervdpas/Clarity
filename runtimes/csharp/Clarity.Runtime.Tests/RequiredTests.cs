// A required field must have a value: a save refuses a record whose
// required field is empty, as every runtime does (conformance/required.json).

using System.Text.Json;
using Clarity.Runtime;
using Xunit;

namespace Clarity.Runtime.Tests;

sealed record Ordered : IRecord<Ordered>
{
    public static string TableName => "orders";
    public static IReadOnlyList<string> ColumnNames { get; } = ["number", "customer_id"];
    public static IReadOnlyList<(string Column, string Label)> Required { get; } = [("number", "Number"), ("customer_id", "Customer")];
    public long Id { get; set; }
    public string Number { get; set; } = "";
    public long? Customer { get; set; }
    public object?[] ToValues() => [Number, Customer];
    public static Ordered FromValues(long id, object?[] values) => new() { Id = id, Number = Sql.GetText("number", values[0]), Customer = values[1] is long c ? c : null };
}

/// <summary>A window with a form on an order: its number box and its customer box show the order's fields.</summary>
sealed class OrderFormViewModel(Db db) : WindowViewModel(db)
{
    public Ordered Order { get; set; } = new();
    public PartState NumberBox { get; } = new("numberBox") { Field = ("order", "number") };
    public PartState CustomerBox { get; } = new("customerBox") { Field = ("order", "customer_id") };
    public PartState Other { get; } = new("other");
    public void SaveClicked() => Handle(() => Db.Save(Order));
}

public sealed class RequiredTests
{
    static JsonElement Cases() =>
        JsonDocument.Parse(File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "required.json"))).RootElement;

    static object? Value(JsonElement value) => value.ValueKind switch
    {
        JsonValueKind.Null => null,
        JsonValueKind.True => true,
        JsonValueKind.False => false,
        JsonValueKind.Number => value.GetInt64(),
        JsonValueKind.String => value.GetString(),
        _ => throw new InvalidOperationException($"no case value {value}"),
    };

    [Fact]
    public void A_save_refuses_an_empty_required_field_as_every_runtime_does()
    {
        var file = Cases();
        var refusal = file.GetProperty("refusal");
        foreach (var c in file.GetProperty("cases").EnumerateArray())
        {
            var name = c.GetProperty("name").GetString();
            var required = c.GetProperty("required").EnumerateArray().Select(r => (r.GetProperty("column").GetString()!, r.GetProperty("label").GetString()!)).ToList();
            var values = c.GetProperty("values").EnumerateObject().ToList();
            var check = () => Requirements.Check(required, [.. values.Select(v => v.Name)], [.. values.Select(v => Value(v.Value))]);
            if (c.GetProperty("refused").ValueKind == JsonValueKind.Null)
            {
                check();
                continue;
            }
            var label = c.GetProperty("refused").GetString();
            var refused = Assert.Throws<RefusedException>(check);
            Assert.Equal(refusal.GetProperty("key").GetString(), refused.Text.Key);
            Assert.Equal(refusal.GetProperty("english").GetString(), refused.Text.Template);
            Assert.Equal(label, refused.Text.Values["field"].Value);
            Assert.True($"{label} is required" == refused.Text.Show(), name);
        }
    }

    [Fact]
    public void An_order_without_its_customer_is_not_saved_and_stays_new()
    {
        using var db = Sqlite.Memory();
        db.Migrate([["CREATE TABLE orders (id INTEGER PRIMARY KEY, number TEXT NOT NULL, customer_id INTEGER);"]]);
        var order = new Ordered { Number = "ORD-1" };
        Assert.Equal("Customer is required", Assert.Throws<RefusedException>(() => db.Save(order)).Message);
        Assert.Equal(0, order.Id);
        Assert.Empty(db.All<Ordered>());
        order.Customer = 4;
        db.Save(order);
        Assert.NotEqual(0, order.Id);
        // A stored record that loses a required value is not changed either.
        var kept = order with { };
        order.Number = "";
        Assert.Throws<RefusedException>(() => db.Save(order));
        Assert.Equal([kept], db.All<Ordered>());
    }

    [Fact]
    public void A_refused_required_field_shows_why_at_its_input_and_code_gets_the_refusal()
    {
        using var db = Sqlite.Memory();
        db.Migrate([["CREATE TABLE orders (id INTEGER PRIMARY KEY, number TEXT NOT NULL, customer_id INTEGER);"]]);
        var window = new OrderFormViewModel(db);
        window.SaveClicked();
        // The person sees why at the input that shows the field, as a form check shows it; no message beside it.
        Assert.Equal("Number is required", window.NumberBox.Error);
        Assert.Equal("fieldRequired", window.NumberBox.ErrorText.Key);
        Assert.Equal("", window.CustomerBox.Error);
        Assert.Empty(window.Messages);
        // The next try clears it, and the next empty field shows at its own input.
        window.Order.Number = "ORD-1";
        window.SaveClicked();
        Assert.Equal("", window.NumberBox.Error);
        Assert.Equal("Customer is required", window.CustomerBox.Error);
        window.Order.Customer = 4;
        window.SaveClicked();
        Assert.Equal("", window.CustomerBox.Error);
        Assert.NotEqual(0, window.Order.Id);
        // Code gets the refusal it can handle: which record, which column.
        var refused = Assert.Throws<RefusedException>(() => db.Save(new Ordered { Number = "ORD-2" }));
        Assert.Equal("customer_id", refused.Column);
        Assert.IsType<Ordered>(refused.Record);
        // A record no input shows is refused as an error message.
        window.Order = new Ordered { Number = "ORD-3" };
        window.Handle(() => db.Save(new Ordered()));
        Assert.Equal("Number is required", Assert.Single(window.Messages).Text.Show());
    }
}
