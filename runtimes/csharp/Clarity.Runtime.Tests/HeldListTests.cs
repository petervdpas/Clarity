// A stored record's list (order.lines) loads with the record, one level deep:
// Find and All call LoadListsOf for every record found, which reads each list
// once for all of them (FindByOwners); Get loads one record's the same way
// (runtimes/rust/tests/held_lists.rs, docs/ledger/runtime-data.md).

using System.Text.Json;
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

    /// <summary>How many times the lines were read: once per list for a whole find.</summary>
    public static int LineReads;

    // As generated code writes them.
    public void LoadLists(Db db) => LoadListsOf(db, [this]);

    public static void LoadListsOf(Db db, IReadOnlyList<HeldOrder> records)
    {
        var owners = records.Select(r => r.Id).ToList();
        LineReads += (owners.Count + Db.OwnersPerRead - 1) / Db.OwnersPerRead;
        var lines = Db.ByOwner(db.FindByOwners<HeldLine>("held_order", owners), l => l.Order);
        foreach (var record in records)
            record.Lines = lines.TryGetValue(record.Id, out var found) ? found : [];
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
        db.ApplySteps([[
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

    /// <summary>Every case of conformance/held-lists.json: a find reads each list once per ownersPerRead orders and gives each order its own lines in order.</summary>
    [Fact]
    public void A_find_reads_each_list_once_for_all_the_records_it_finds()
    {
        var file = JsonDocument.Parse(File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "held-lists.json"))).RootElement;
        Assert.Equal(Db.OwnersPerRead, file.GetProperty("ownersPerRead").GetInt32());
        foreach (var c in file.GetProperty("cases").EnumerateArray())
        {
            var name = c.GetProperty("name").GetString();
            using var db = Sqlite.Memory();
            db.ApplySteps([[
                "CREATE TABLE held_order (id INTEGER PRIMARY KEY, number TEXT NOT NULL);",
                "CREATE TABLE held_line (id INTEGER PRIMARY KEY, held_order INTEGER, product TEXT NOT NULL);",
            ]]);
            for (var n = 1; n <= c.GetProperty("orders").GetInt32(); n++)
                db.Save(new HeldOrder { Number = $"ORD-{n:00000}" });
            foreach (var line in c.GetProperty("lines").EnumerateArray())
                db.Save(new HeldLine { Order = line[0].GetInt64(), Product = line[1].GetString()! });
            HeldOrder.LineReads = 0;
            var found = db.Find<HeldOrder>(new Query().Order("number", false));
            Assert.True(c.GetProperty("reads").GetInt32() == HeldOrder.LineReads, $"{name}: {HeldOrder.LineReads} reads");
            foreach (var expect in c.GetProperty("expect").EnumerateObject())
            {
                var order = found.Single(o => o.Id.ToString() == expect.Name);
                var want = expect.Value.EnumerateArray().Select(p => p.GetString()).ToArray();
                Assert.True(want.SequenceEqual(order.Lines.Select(l => l.Product)), $"{name}: order {expect.Name}");
                Assert.Equal(want.Length, db.Get<HeldOrder>(order.Id).Lines.Count);
            }
        }
    }
}
