// Records written the way Clarity generates them, stored in SQLite. The same
// behavior as the Rust runtime's tests (runtimes/rust/tests/db.rs).

using Clarity.Runtime;
using Xunit;

namespace Clarity.Runtime.Tests;

public enum CustomerKind
{
    Private,
    Business,
}

public sealed class Customer : IRecord<Customer>
{
    public static string TableName => "customer";
    public static IReadOnlyList<string> ColumnNames { get; } = ["name", "kind", "credit_limit", "active", "since", "referred_by"];

    public long Id { get; set; }
    public string Name { get; set; } = "";
    public CustomerKind Kind { get; set; }
    public decimal CreditLimit { get; set; }
    public bool Active { get; set; }
    public DateOnly? Since { get; set; }
    public long? ReferredBy { get; set; }

    public object?[] ToValues() => [Name, Sql.Name(Kind), CreditLimit, Active, Since, ReferredBy];

    public static Customer FromValues(long id, object?[] values) => new()
    {
        Id = id,
        Name = Sql.GetText("name", values[0]),
        Kind = Sql.GetName<CustomerKind>("kind", values[1]),
        CreditLimit = Sql.GetDecimal("credit_limit", values[2]),
        Active = Sql.GetBoolean("active", values[3]),
        Since = Sql.GetDate("since", values[4]),
        ReferredBy = Sql.GetReference("referred_by", values[5]),
    };
}

/// A record kept in another database than the customers.
public sealed class Invoice : IRecord<Invoice>
{
    public static string SourceName => "erp";
    public static string TableName => "invoice";
    public static IReadOnlyList<string> ColumnNames { get; } = ["number", "customer"];

    public long Id { get; set; }
    public string Number { get; set; } = "";
    public long? Customer { get; set; }

    public object?[] ToValues() => [Number, Customer];

    public static Invoice FromValues(long id, object?[] values) => new()
    {
        Id = id,
        Number = Sql.GetText("number", values[0]),
        Customer = Sql.GetReference("customer", values[1]),
    };
}

/// A record of a table that was there before Clarity: its own key column and column names.
public sealed class Legacy : IRecord<Legacy>
{
    public static string TableName => "tblInvoice";
    public static string KeyColumn => "InvoiceID";
    public static IReadOnlyList<string> ColumnNames { get; } = ["InvNo"];

    public long Id { get; set; }
    public string Number { get; set; } = "";

    public object?[] ToValues() => [Number];

    public static Legacy FromValues(long id, object?[] values) => new() { Id = id, Number = Sql.GetText("InvNo", values[0]) };
}

public class DbTests
{
    const string Customers = """
        CREATE TABLE customer (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            kind TEXT NOT NULL,
            credit_limit TEXT NOT NULL,
            active INTEGER NOT NULL,
            since TEXT,
            referred_by INTEGER REFERENCES customer (id)
        );
        """;

    static readonly string[][] Steps = [[Customers]];

    static Db NewDb()
    {
        var db = Sqlite.Memory();
        Assert.Equal(1, db.Migrate(Steps));
        return db;
    }

    static Customer Ada() => new()
    {
        Name = "Ada Lovelace",
        Kind = CustomerKind.Business,
        CreditLimit = 1250.50m,
        Active = true,
        Since = new DateOnly(2026, 1, 15),
    };

    static Customer NewCustomer(string name, CustomerKind kind, string limit, bool active) => new()
    {
        Name = name,
        Kind = kind,
        CreditLimit = decimal.Parse(limit, System.Globalization.CultureInfo.InvariantCulture),
        Active = active,
    };

    /// Ada (Business, 1250.50), Grace (Private, 99.5) and Linus (Business, 900, inactive), stored in that order.
    static (Db, List<Customer>) Three()
    {
        var db = NewDb();
        List<Customer> all =
        [
            NewCustomer("Ada Lovelace", CustomerKind.Business, "1250.50", true),
            NewCustomer("Grace Hopper", CustomerKind.Private, "99.5", true),
            NewCustomer("Linus", CustomerKind.Business, "900", false),
        ];
        foreach (var c in all)
            db.Save(c);
        return (db, all);
    }

    static string[] Names(IEnumerable<Customer> found) => found.Select(c => c.Name).ToArray();

    [Fact]
    public void SavesLoadsUpdatesAndDeletesRecords()
    {
        using var db = NewDb();
        var customer = Ada();
        db.Save(customer);
        Assert.Equal(1, customer.Id);
        var stored = db.Get<Customer>(1);
        Assert.Equal(("Ada Lovelace", CustomerKind.Business, 1250.50m, true, new DateOnly(2026, 1, 15), (long?)null),
            (stored.Name, stored.Kind, stored.CreditLimit, stored.Active, stored.Since, stored.ReferredBy));
        Assert.Equal("1250.50", Show.Of(stored.CreditLimit));

        customer.CreditLimit = 9.9m;
        db.Save(customer);
        Assert.Equal(9.9m, db.Get<Customer>(1).CreditLimit);

        var grace = new Customer { Name = "Grace Hopper", ReferredBy = customer.Id };
        db.Save(grace);
        Assert.Equal(["Ada Lovelace", "Grace Hopper"], Names(db.All<Customer>()));
        Assert.Equal("Ada Lovelace", db.Load<Customer>(grace.ReferredBy).Name);

        // A record another record refers to stays until that reference is gone.
        var inUse = Assert.Throws<InUseException>(() => db.Delete(customer));
        Assert.Equal(("customer", 1L), (inUse.Table, inUse.Id));
        db.Delete(grace);
        db.Delete(customer);
        var missing = Assert.Throws<NotFoundException>(() => db.Get<Customer>(1));
        Assert.Equal(("customer", (long?)1), (missing.Table, missing.Id));
        Assert.Null(Assert.Throws<NotFoundException>(() => db.Load<Customer>(null)).Id);
    }

    [Fact]
    public void AppliesEachMigrationStepOnce()
    {
        using var db = NewDb();
        Assert.Equal(1, db.SchemaVersion());
        Assert.Equal(0, db.Migrate(Steps));
        Assert.Equal(1, db.Migrate([Steps[0], ["ALTER TABLE customer ADD COLUMN notes TEXT;"]]));
        Assert.Equal(2, db.SchemaVersion());
    }

    [Fact]
    public void AFailedStepChangesNothing()
    {
        using var db = NewDb();
        Assert.Throws<StorageException>(() => db.Migrate([Steps[0], ["ALTER TABLE customer ADD COLUMN a TEXT; THIS IS NOT SQL;"]]));
        Assert.Equal(1, db.SchemaVersion());
    }

    [Fact]
    public void ContractsAndText()
    {
        Contracts.Require(true, "x > 0");
        Assert.Equal("x > 0", Assert.Throws<ContractException>(() => Contracts.Require(false, "x > 0")).What);
        Assert.Equal("condition failed: x > 0", new ContractException("x > 0").Message);
        Assert.Equal("1250.50", Show.Of(1250.50m));
        Assert.Equal("2026-01-15", Show.Of((DateOnly?)new DateOnly(2026, 1, 15)));
        Assert.Equal("", Show.Of((DateOnly?)null));
        Assert.Equal("yes", Show.Of(true));
    }

    [Fact]
    public void FindsRecordsByACondition()
    {
        var (db, _) = Three();
        using var _db = db;
        Assert.Equal(["Ada Lovelace", "Linus"],
            Names(db.Find<Customer>(new Query().Filter(Cond.Compare("kind", Op.Eq, Sql.Name(CustomerKind.Business))))));

        // Contains ignores case.
        Assert.Equal(["Grace Hopper"], Names(db.Find<Customer>(new Query().Filter(Cond.Contains("name", "HOP")))));

        // Decimals compare as numbers: 99.5 is less than 900, though "99.5" sorts after "900".
        Assert.Equal(["Ada Lovelace"], Names(db.Find<Customer>(new Query().Filter(Cond.Number("credit_limit", Op.Gt, 900m)))));

        var activeBusiness = Cond.IsTrue("active").And(Cond.OneOf("kind", [Sql.Name(CustomerKind.Business)]));
        Assert.Equal(["Ada Lovelace"], Names(db.Find<Customer>(new Query().Filter(activeBusiness))));

        var inactiveOrPrivate = Cond.IsTrue("active").Negated().Or(Cond.Compare("kind", Op.Eq, Sql.Name(CustomerKind.Private)));
        Assert.Equal(["Grace Hopper", "Linus"], Names(db.Find<Customer>(new Query().Filter(inactiveOrPrivate))));
    }

    [Fact]
    public void FindsRecordsByAReference()
    {
        var (db, all) = Three();
        using var _db = db;
        var referred = NewCustomer("Referred", CustomerKind.Private, "0", true);
        referred.ReferredBy = all[0].Id;
        db.Save(referred);
        Assert.Equal(["Referred"], Names(db.Find<Customer>(new Query().Filter(Cond.Compare("referred_by", Op.Eq, all[0].Id)))));
    }

    [Fact]
    public void OrdersWhatItFinds()
    {
        var (db, _) = Three();
        using var _db = db;
        Assert.Equal(["Ada Lovelace", "Linus", "Grace Hopper"], Names(db.Find<Customer>(new Query().OrderNumber("credit_limit", true))));
        Assert.Equal(["Linus", "Ada Lovelace", "Grace Hopper"],
            Names(db.Find<Customer>(new Query().Order("kind", false).Order("name", true))));
        // Without an order, records come in the order they were stored.
        Assert.Equal(["Ada Lovelace", "Grace Hopper", "Linus"], Names(db.Find<Customer>(new Query())));
    }

    [Fact]
    public void ContainsTreatsWildcardsAsText()
    {
        using var db = NewDb();
        db.Save(NewCustomer("100% sure_thing", CustomerKind.Private, "0", true));
        db.Save(NewCustomer("100 sure things", CustomerKind.Private, "0", true));
        Assert.Equal(["100% sure_thing"], Names(db.Find<Customer>(new Query().Filter(Cond.Contains("name", "0% sure_")))));
        Assert.True(Texts.Contains("Ada Lovelace", "LOVE"));
        Assert.False(Texts.Contains("Ada", "Grace"));
    }

    /// The table built again, as a step that changes a column does it.
    const string Rebuild = """
        CREATE TABLE customer__new (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            kind TEXT NOT NULL,
            credit_limit TEXT NOT NULL,
            active INTEGER NOT NULL,
            since TEXT,
            referred_by INTEGER REFERENCES customer (id)
        );
        INSERT INTO customer__new (id, name, kind, credit_limit, active, since, referred_by)
            SELECT id, name, kind, credit_limit, active, since, referred_by FROM customer;
        DROP TABLE customer;
        ALTER TABLE customer__new RENAME TO customer;
        """;

    [Fact]
    public void AStepCanBuildAReferencedTableAgain()
    {
        using var db = NewDb();
        var ada = Ada();
        db.Save(ada);
        var grace = NewCustomer("Grace Hopper", CustomerKind.Private, "0", true);
        grace.ReferredBy = ada.Id;
        db.Save(grace);

        Assert.Equal(1, db.Migrate([Steps[0], [Rebuild]]));
        Assert.Equal(ada.Id, db.Get<Customer>(grace.Id).ReferredBy);
        // References are checked again after the step.
        Assert.Throws<InUseException>(() => db.Delete(ada));
    }

    [Fact]
    public void AStepThatBreaksReferencesChangesNothing()
    {
        using var db = NewDb();
        var grace = NewCustomer("Grace Hopper", CustomerKind.Private, "0", true);
        db.Save(grace);
        Assert.Throws<StorageException>(() => db.Migrate([Steps[0], ["UPDATE customer SET referred_by = 99;"]]));
        Assert.Equal(1, db.SchemaVersion());
        Assert.Null(db.Get<Customer>(grace.Id).ReferredBy);
    }

    [Fact]
    public void EachRecordLivesInItsOwnDataSource()
    {
        var dir = Path.Combine(Path.GetTempPath(), $"clarity-sources-{Environment.ProcessId}");
        if (Directory.Exists(dir))
            Directory.Delete(dir, true);
        var main = Path.Combine(dir, "main.db");
        var erp = Path.Combine(dir, "erp.db");
        try
        {
            using (var db = Db.Connect(("main", Dialect.Sqlite, main), ("erp", Dialect.Sqlite, erp)))
            {
                db.MigrateSource("main", Steps);
                // A reference to another database is a plain id: no foreign key.
                db.MigrateSource("erp", [["CREATE TABLE invoice (id INTEGER PRIMARY KEY, number TEXT NOT NULL, customer INTEGER);"]]);
                var ada = Ada();
                db.Save(ada);
                db.Save(new Invoice { Number = "INV-1", Customer = ada.Id });
            }

            // Each table is in its own file.
            using (var onlyMain = Sqlite.Open(main))
                Assert.Single(onlyMain.All<Customer>());
            using (var db = Db.Connect(("main", Dialect.Sqlite, main), ("erp", Dialect.Sqlite, erp)))
            {
                Assert.Equal(["INV-1"], db.All<Invoice>().Select(i => i.Number));
                Assert.Equal("Ada Lovelace", db.Load<Customer>(db.Get<Invoice>(1).Customer).Name);
            }
        }
        finally
        {
            Microsoft.Data.Sqlite.SqliteConnection.ClearAllPools();
            Directory.Delete(dir, true);
        }
    }

    [Fact]
    public void ARecordCanLiveInAnExistingTable()
    {
        using var db = Sqlite.Memory();
        db.Migrate([["CREATE TABLE tblInvoice (InvoiceID INTEGER PRIMARY KEY, InvNo TEXT NOT NULL, Other TEXT);"]]);
        var first = new Legacy { Number = "INV-1" };
        db.Save(first);
        var second = new Legacy { Number = "INV-2" };
        db.Save(second);
        Assert.Equal(2, second.Id);
        second.Number = "INV-2b";
        db.Save(second);
        Assert.Equal("INV-2b", db.Get<Legacy>(2).Number);
        Assert.Equal([2L], db.Find<Legacy>(new Query().Filter(Cond.Contains("InvNo", "2b"))).Select(l => l.Id));
        db.Delete(first);
        Assert.Equal([2L], db.All<Legacy>().Select(l => l.Id));
    }

    [Fact]
    public void ValuesWrittenByOtherProgramsAreRead()
    {
        // SQLite keeps what was written: numbers in text columns, dates with seconds or a T.
        Assert.Equal("42", Sql.GetText("code", 42L));
        Assert.Equal("1.5", Sql.GetText("ratio", 1.5));
        Assert.Equal(new DateTime(2026, 10, 17, 9, 30, 0), Sql.GetDateTime("at", "2026-10-17 09:30"));
        Assert.Equal(new DateTime(2026, 10, 17, 9, 30, 15), Sql.GetDateTime("at", "2026-10-17 09:30:15"));
        Assert.Equal(new DateTime(2026, 10, 17, 9, 30, 15), Sql.GetDateTime("at", "2026-10-17T09:30:15"));
        Assert.Equal(new DateOnly(2026, 10, 17), Sql.GetDate("on", "2026-10-17 00:00:00"));
        Assert.Throws<InvalidValueException>(() => Sql.GetName<CustomerKind>("kind", "Unknown"));
        Assert.Equal(CustomerKind.Private, Sql.GetName<CustomerKind>("kind", ""));
    }
}
