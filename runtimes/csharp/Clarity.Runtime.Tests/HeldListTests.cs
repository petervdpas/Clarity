// A stored record's list (order.lines) loads with the record, one level deep:
// Find, All and Get call LoadLists, which reads the lines with FindAlone
// (runtimes/rust/tests/held_lists.rs, docs/ledger/runtime-data.md).

using Clarity.Runtime;
using Xunit;

namespace Clarity.Runtime.Tests;

public sealed class HeldOrder : IRecord<HeldOrder>
{
    public static string TableName => "held_order";
    public static IReadOnlyList<string> ColumnNames { get; } = ["number"];
    public long Id { get; set; }
    public string Number { get; set; } = "";
    public List<HeldLine> Lines { get; set; } = [];
    public object?[] ToValues() => [Number];
    public static HeldOrder FromValues(long id, object?[] values) => new() { Id = id, Number = Sql.GetText("number", values[0]) };

    // As generated code writes it.
    public void LoadLists(Db db)
    {
        Lines = db.FindAlone<HeldLine>(new Query().Filter(Cond.Compare("held_order", Op.Eq, Id)).Order("id", false));
    }
}

public sealed class HeldLine : IRecord<HeldLine>
{
    public static string TableName => "held_line";
    public static IReadOnlyList<string> ColumnNames { get; } = ["held_order", "product"];
    public long Id { get; set; }
    public long? Order { get; set; }
    public string Product { get; set; } = "";
    public object?[] ToValues() => [Order, Product];
    public static HeldLine FromValues(long id, object?[] values) => new() { Id = id, Order = Sql.GetReference("held_order", values[0]), Product = Sql.GetText("product", values[1]) };
}

public class HeldListTests
{
    [Fact]
    public void A_stored_records_lists_load_with_it()
    {
        using var db = Sqlite.Memory();
        db.Migrate([[
            "CREATE TABLE held_order (id INTEGER PRIMARY KEY, number TEXT NOT NULL);",
            "CREATE TABLE held_line (id INTEGER PRIMARY KEY, held_order INTEGER, product TEXT NOT NULL);",
        ]]);
        var first = new HeldOrder { Number = "ORD-1" };
        var second = new HeldOrder { Number = "ORD-2" };
        db.Save(first);
        db.Save(second);
        foreach (var product in new[] { "pen", "ink" })
            db.Save(new HeldLine { Order = first.Id, Product = product });
        var found = db.Find<HeldOrder>(new Query().Order("number", false));
        Assert.Equal(["pen", "ink"], found[0].Lines.Select(l => l.Product));
        Assert.Empty(found[1].Lines);
        Assert.Equal(2, db.All<HeldOrder>()[0].Lines.Count);
        Assert.Equal(2, db.Get<HeldOrder>(first.Id).Lines.Count);
        Assert.Empty(db.FindAlone<HeldOrder>(new Query())[0].Lines);
    }
}
