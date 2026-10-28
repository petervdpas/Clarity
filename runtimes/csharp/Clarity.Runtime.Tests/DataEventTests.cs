// Data events: a record whose entity has them runs them around its save and
// delete, in one transaction with them (runtimes/rust/tests/data_events.rs).

using Clarity.Runtime;
using Xunit;

namespace Clarity.Runtime.Tests;

/// What the events of an account wrote, in order.
public sealed class Log : IRecord<Log>
{
    public static string TableName => "log";
    public static IReadOnlyList<string> ColumnNames { get; } = ["text"];

    public long Id { get; set; }
    public string Text { get; set; } = "";

    public object?[] ToValues() => [Text];

    public static Log FromValues(long id, object?[] values) => new() { Id = id, Text = Sql.GetText("text", values[0]) };
}

public sealed class Account : IRecord<Account>
{
    public static string TableName => "account";
    public static IReadOnlyList<string> ColumnNames { get; } = ["name", "balance"];
    public static bool HasEvents => true;

    public long Id { get; set; }
    public string Name { get; set; } = "";
    public long Balance { get; set; }

    public object?[] ToValues() => [Name, Balance];

    public static Account FromValues(long id, object?[] values) => new()
    {
        Id = id,
        Name = Sql.GetText("name", values[0]),
        Balance = Sql.GetInteger("balance", values[1]),
    };

    public void BeforeSave(Db db)
    {
        if (Name.Trim() == "")
            throw new RefusedException(Text.Plain("An account needs a name"));
        Name = Name.Trim();
    }

    public void AfterSave(Db db)
    {
        db.Save(new Log { Text = $"saved {Name} with {Balance}" });
        if (Balance < 0)
            throw new RefusedException(Text.Plain("An account cannot be in debt"));
    }

    public void BeforeDelete(Db db)
    {
        if (Balance != 0)
            throw new RefusedException(Text.Plain($"{Name} still holds {Balance}"));
    }

    public void AfterDelete(Db db) => db.Save(new Log { Text = $"deleted {Name}" });
}

public class DataEventTests
{
    static Db NewDb()
    {
        var db = Sqlite.Memory();
        db.Migrate([[
            "CREATE TABLE account (id INTEGER PRIMARY KEY, name TEXT NOT NULL, balance INTEGER NOT NULL);",
            "CREATE TABLE log (id INTEGER PRIMARY KEY, text TEXT NOT NULL);",
        ]]);
        return db;
    }

    static string[] Logged(Db db) => db.All<Log>().Select(l => l.Text).ToArray();

    [Fact]
    public void EventsRunAroundASave()
    {
        using var db = NewDb();
        var ada = new Account { Name = "  Ada ", Balance = 10 };
        db.Save(ada);
        // Before save changed the record that was stored.
        Assert.Equal("Ada", ada.Name);
        Assert.Equal("Ada", db.Get<Account>(ada.Id).Name);
        Assert.Equal(["saved Ada with 10"], Logged(db));

        var refused = Assert.Throws<RefusedException>(() => db.Save(new Account()));
        Assert.Equal("An account needs a name", refused.Message);
        Assert.Single(db.All<Account>());
    }

    [Fact]
    public void AFailingEventKeepsNothing()
    {
        using var db = NewDb();
        var debtor = new Account { Name = "Grace", Balance = -5 };
        Assert.Equal("An account cannot be in debt", Assert.Throws<RefusedException>(() => db.Save(debtor)).Message);
        // The record, and what after save wrote, are gone; the record is new again.
        Assert.Equal(0, debtor.Id);
        Assert.Empty(db.All<Account>());
        Assert.Empty(Logged(db));

        // The database is usable afterwards.
        debtor.Balance = 0;
        db.Save(debtor);
        Assert.Equal(["saved Grace with 0"], Logged(db));
    }

    [Fact]
    public void EventsRunAroundADelete()
    {
        using var db = NewDb();
        var ada = new Account { Name = "Ada", Balance = 10 };
        db.Save(ada);
        Assert.Equal("Ada still holds 10", Assert.Throws<RefusedException>(() => db.Delete(ada)).Message);
        Assert.Single(db.All<Account>());

        ada.Balance = 0;
        db.Save(ada);
        db.Delete(ada);
        Assert.Empty(db.All<Account>());
        Assert.Equal(["saved Ada with 10", "saved Ada with 0", "deleted Ada"], Logged(db));
    }
}
