// The generated migration steps (conformance/migrations.json) applied by the
// C# runtime, with records stored between the steps: the same scenario as
// the Rust runtime's server tests (crates/clarity-gen/tests/servers.rs). It
// always runs on SQLite; on PostgreSQL, MySQL and SQL Server it needs a
// scratch database whose tables it may drop, named by an environment
// variable, and is skipped without it:
//
//   CLARITY_TEST_POSTGRES="host=localhost port=5432 user=postgres password=... dbname=scratch"
//   CLARITY_TEST_MYSQL="mysql://root:...@localhost:3306/scratch"
//   CLARITY_TEST_MSSQL="server=tcp:localhost,1433;database=scratch;user=sa;password=...;TrustServerCertificate=true"
//
// These are the variables of the Rust tests, in the forms a project file
// writes (Locations), so one set of scratch databases serves both runtimes.

using System.Text.Json;
using Clarity.Runtime;
using Xunit;

namespace Clarity.Runtime.Tests.Servers;

/// A customer as the first step stores it.
public sealed record Before : IRecord<Before>
{
    public static string TableName => "customer";
    public static IReadOnlyList<string> ColumnNames { get; } = ["name", "email", "kind", "limit", "active", "since", "notes"];

    public long Id { get; set; }
    public string Name { get; set; } = "";
    public string Email { get; set; } = "";
    public string Kind { get; set; } = "";
    public decimal Limit { get; set; }
    public bool Active { get; set; }
    public DateOnly? Since { get; set; }
    public string Notes { get; set; } = "";

    public object?[] ToValues() => [Name, Email, Kind, Limit, Active, Since, Notes];

    public static Before FromValues(long id, object?[] v) => new()
    {
        Id = id,
        Name = Sql.GetText("name", v[0]),
        Email = Sql.GetText("email", v[1]),
        Kind = Sql.GetText("kind", v[2]),
        Limit = Sql.GetDecimal("limit", v[3]),
        Active = Sql.GetBoolean("active", v[4]),
        Since = Sql.GetDate("since", v[5]),
        Notes = Sql.GetText("notes", v[6]),
    };
}

/// An order as the first step stores it.
public sealed record Order : IRecord<Order>
{
    public static string TableName => "order";
    public static IReadOnlyList<string> ColumnNames { get; } = ["number", "customer", "total", "placed", "paid", "code"];

    public long Id { get; set; }
    public string Number { get; set; } = "";
    public long? Customer { get; set; }
    public long Total { get; set; }
    public DateTime? Placed { get; set; }
    public bool Paid { get; set; }
    public string Code { get; set; } = "";

    public object?[] ToValues() => [Number, Customer, Total, Placed, Paid, Code];

    public static Order FromValues(long id, object?[] v) => new()
    {
        Id = id,
        Number = Sql.GetText("number", v[0]),
        Customer = Sql.GetReference("customer", v[1]),
        Total = Sql.GetInteger("total", v[2]),
        Placed = Sql.GetDateTime("placed", v[3]),
        Paid = Sql.GetBoolean("paid", v[4]),
        Code = Sql.GetText("code", v[5]),
    };
}

/// A customer after the second step.
public sealed record Customer : IRecord<Customer>
{
    public static string TableName => "customer";
    public static IReadOnlyList<string> ColumnNames { get; } = ["name", "email", "kind", "credit_limit", "vip", "remarks", "phone", "portrait"];
    public static IReadOnlyList<PictureColumn> Pictures { get; } = [new("portrait", null)];

    public long Id { get; set; }
    public string Name { get; set; } = "";
    public string Email { get; set; } = "";
    public string Kind { get; set; } = "";
    public decimal CreditLimit { get; set; }
    public bool Vip { get; set; }
    public string Remarks { get; set; } = "";
    public string Phone { get; set; } = "";
    public Picture? Portrait { get; set; }

    public object?[] ToValues() => [Name, Email, Kind, CreditLimit, Vip, Remarks, Phone, Sql.Picture(Portrait)];

    public static Customer FromValues(long id, object?[] v) => new()
    {
        Id = id,
        Name = Sql.GetText("name", v[0]),
        Email = Sql.GetText("email", v[1]),
        Kind = Sql.GetText("kind", v[2]),
        CreditLimit = Sql.GetDecimal("credit_limit", v[3]),
        Vip = Sql.GetBoolean("vip", v[4]),
        Remarks = Sql.GetText("remarks", v[5]),
        Phone = Sql.GetText("phone", v[6]),
        Portrait = Sql.GetPicture("portrait", v[7]),
    };
}

/// An order after the second step: another table, and a money total.
public sealed record Purchase : IRecord<Purchase>
{
    public static string TableName => "purchase";
    public static IReadOnlyList<string> ColumnNames { get; } = ["number", "customer", "total", "placed", "paid", "code"];

    public long Id { get; set; }
    public string Number { get; set; } = "";
    public long? Customer { get; set; }
    public decimal Total { get; set; }
    public DateTime? Placed { get; set; }
    public long Paid { get; set; }
    public long Code { get; set; }

    public object?[] ToValues() => [Number, Customer, Total, Placed, Paid, Code];

    public static Purchase FromValues(long id, object?[] v) => new()
    {
        Id = id,
        Number = Sql.GetText("number", v[0]),
        Customer = Sql.GetReference("customer", v[1]),
        Total = Sql.GetDecimal("total", v[2]),
        Placed = Sql.GetDateTime("placed", v[3]),
        Paid = Sql.GetInteger("paid", v[4]),
        Code = Sql.GetInteger("code", v[5]),
    };
}

public class ServerTests
{
    static string[][] Steps(string database)
    {
        var path = Path.Combine(AppContext.BaseDirectory, "conformance", "migrations.json");
        var steps = JsonDocument.Parse(File.ReadAllText(path)).RootElement.GetProperty("steps").GetProperty(database);
        return steps.EnumerateArray().Select(s => s.EnumerateArray().Select(t => t.GetString()!).ToArray()).ToArray();
    }

    static string[] Names<R>(IEnumerable<R> found, Func<R, string> name) => found.Select(name).ToArray();

    /// <summary>Both steps on a scratch database, with records kept across them.</summary>
    static void MigratesAndKeepsRecords(Dialect dialect, string at, string database)
    {
        using var db = Db.Connect(("main", dialect, at));
        foreach (var table in new[] { "purchase", "order", "customer", "clarity_schema" })
            db.Batch("main", $"DROP TABLE IF EXISTS {dialect.Quote(table)}");
        var steps = Steps(database);

        Assert.Equal(1, db.Migrate(steps[..1]));
        var since = new DateOnly(2026, 1, 15);
        var ada = new Before { Name = "Ada Lovelace", Email = "ada@example.com", Kind = "Business", Limit = 1250.50m, Active = true, Since = since, Notes = "first's" };
        db.Save(ada);
        var grace = new Before { Name = "Grace Hopper", Kind = "Private", Limit = 99.5m };
        db.Save(grace);
        Assert.Equal(ada, db.Get<Before>(ada.Id));
        // Saving what is stored already is no missing record.
        db.Save(grace);

        var placed = new DateTime(2026, 10, 17, 9, 30, 15);
        var order = new Order { Number = "ORD-1", Customer = ada.Id, Total = 99, Placed = placed, Paid = true, Code = "42" };
        db.Save(order);
        var blank = new Order { Number = "ORD-0" };
        db.Save(blank);
        db.Save(order);
        var inUse = Assert.Throws<InUseException>(() => db.Delete(ada));
        Assert.Equal(("customer", ada.Id), (inUse.Table, inUse.Id));

        // Queries: contains ignores case and wildcards, decimals compare as numbers.
        Assert.Equal(["Ada Lovelace"], Names(db.Find<Before>(new Query().Filter(Cond.Contains("name", "LOVE"))), c => c.Name));
        Assert.Empty(db.Find<Before>(new Query().Filter(Cond.Contains("name", "%"))));
        var big = new Query().Filter(Cond.Number("limit", Op.Gt, 900L).And(Cond.IsTrue("active")));
        Assert.Equal(["Ada Lovelace"], Names(db.Find<Before>(big), c => c.Name));
        Assert.Equal(["Grace Hopper", "Ada Lovelace"], Names(db.Find<Before>(new Query().OrderNumber("limit", false)), c => c.Name));
        Assert.Single(db.Find<Before>(new Query().Filter(Cond.Compare("since", Op.Eq, since))));
        Assert.Equal(["Grace Hopper"], Names(db.Find<Before>(new Query().Filter(Cond.OneOf("kind", ["Private"]))), c => c.Name));

        // Through a reference: the customer's name, in a condition and an order.
        Assert.Equal(["ORD-1"], Names(db.Find<Order>(new Query().Filter(Cond.Through("customer", "customer", "id", Cond.Contains("name", "LOVE")))), o => o.Number));
        var grace2 = new Order { Number = "ORD-G", Customer = grace.Id };
        db.Save(grace2);
        var byName = new Query().Filter(Cond.IsNone("customer").Negated()).OrderThrough([new Hop("customer", "customer", "id")], "name", true, false);
        Assert.Equal(["ORD-G", "ORD-1"], Names(db.Find<Order>(byName), o => o.Number));
        db.Delete(grace2);

        Assert.Equal(1, db.Migrate(steps));
        Assert.Equal(0, db.Migrate(steps));
        Assert.Equal(2, db.SchemaVersion());

        var customer = db.Get<Customer>(ada.Id);
        Assert.Equal(1250.50m, customer.CreditLimit);
        Assert.True(customer.Vip);
        Assert.Equal("first's", customer.Remarks);
        Assert.Equal("", customer.Phone);
        var purchase = db.Get<Purchase>(order.Id);
        Assert.Equal(99m, purchase.Total);
        Assert.Equal(ada.Id, purchase.Customer);
        Assert.Equal(placed, purchase.Placed);
        Assert.Equal((1L, 42L), (purchase.Paid, purchase.Code));
        // Empty text is no number: it becomes the default.
        Assert.Equal(0, db.Get<Purchase>(blank.Id).Code);

        // A picture in a binary column: saved, kept by a save that leaves it, emptied.
        byte[] png = [0x89, (byte)'P', (byte)'N', (byte)'G', 0x0D, 0x0A, 0x1A, 0x0A];
        var pictured = db.Get<Customer>(ada.Id);
        Assert.Null(pictured.Portrait);
        db.Save(pictured with { Portrait = Picture.New(png) });
        pictured = db.Get<Customer>(ada.Id);
        Assert.Same(Picture.Stored, pictured.Portrait);
        db.Save(pictured with { Phone = "555" });
        Assert.Equal(png, db.PictureOf<Customer>(ada.Id, "portrait"));
        db.Save(pictured with { Portrait = null, Phone = "" });
        Assert.Null(db.PictureOf<Customer>(ada.Id, "portrait"));

        // Unique email: empty many times, a filled one once.
        var linus = new Customer { Name = "Linus" };
        db.Save(linus);
        Assert.Equal(0m, linus.CreditLimit);
        Assert.ThrowsAny<ClarityException>(() => db.Save(new Customer { Name = "Copy", Email = "ada@example.com" }));
        // The reference is kept through the renamed table.
        var adaNow = db.Get<Customer>(ada.Id);
        Assert.Throws<InUseException>(() => db.Delete(adaNow));
        Assert.Throws<InvalidValueException>(() => db.Save(new Purchase { Number = "ORD-2", Customer = 999 }));
        db.Delete(purchase);
        db.Delete(adaNow);
        Assert.Equal(2, db.All<Customer>().Count);
    }

    [Fact]
    public void Sqlite()
    {
        var file = Path.Combine(Path.GetTempPath(), $"clarity-server-{Guid.NewGuid():N}.db");
        try
        {
            MigratesAndKeepsRecords(Dialect.Sqlite, file, "sqlite");
        }
        finally
        {
            Microsoft.Data.Sqlite.SqliteConnection.ClearAllPools();
            File.Delete(file);
        }
    }

    static void OnServer(string variable, Dialect dialect, string database)
    {
        if (Environment.GetEnvironmentVariable(variable) is not { Length: > 0 } at)
        {
            Assert.Skip($"{variable} is not set");
            return;
        }
        MigratesAndKeepsRecords(dialect, at, database);
    }

    [Fact]
    public void Postgres() => OnServer("CLARITY_TEST_POSTGRES", Dialect.Postgres, "postgres");

    [Fact]
    public void Mysql() => OnServer("CLARITY_TEST_MYSQL", Dialect.Mysql, "mysql");

    [Fact]
    public void Mssql() => OnServer("CLARITY_TEST_MSSQL", Dialect.Mssql, "mssql");
}
