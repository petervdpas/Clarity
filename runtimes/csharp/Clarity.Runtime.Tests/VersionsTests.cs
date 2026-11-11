// A save never overwrites someone else's change (conformance/versions.json):
// the version read with every record, the checked save and delete, a save
// anyway, reload and undo by the version. The steps of one edit are undone
// all or none, a handler is one transaction, and only the last UndoSteps
// are kept. The file case is in Clarity.Ui.Native.Tests (FileSourceTests).

using System.Text.Json;
using Clarity.Runtime;
using Xunit;

namespace Clarity.Runtime.Tests;

sealed record Item : IRecord<Item>
{
    public static string TableName => "item";
    public static string? VersionColumn => "row_version";
    public static IReadOnlyList<string> ColumnNames { get; } = ["name"];
    public long Id { get; set; }
    public long Version { get; set; }
    public string Name { get; set; } = "";
    public object?[] ToValues() => [Name];
    public static Item FromValues(long id, object?[] values) => new() { Id = id, Name = Sql.GetText("name", values[0]) };
}

public class VersionsTests
{
    const string UndoTable = "CREATE TABLE item (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL, row_version INTEGER NOT NULL DEFAULT 1)";

    static JsonElement File() =>
        JsonDocument.Parse(System.IO.File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "versions.json"))).RootElement;

    /// <summary>Whether <paramref name="run"/> does what the step expects: goes, is refused with <paramref name="refusal"/>'s words, or finds nothing.</summary>
    static void Expect(string name, JsonElement step, JsonElement refusal, Action run)
    {
        var expected = step.TryGetProperty("expect", out var e) ? e.GetString() : "ok";
        try
        {
            run();
            Assert.True(expected == "ok", $"{name}: {step} went, {expected} expected");
        }
        catch (RefusedException refused)
        {
            Assert.True(expected == "refused", $"{name}: {step} refused: {refused.Message}");
            Assert.Equal(refusal.GetProperty("key").GetString(), refused.Text.Key);
            Assert.Equal(refusal.GetProperty("english").GetString(), refused.Text.Template);
        }
        catch (NotFoundException missing)
        {
            Assert.True(expected == "notFound", $"{name}: {step} not found: {missing.Message}");
        }
    }

    [Fact]
    public void Versions_are_checked_as_every_runtime_does()
    {
        var file = File();
        foreach (var c in file.GetProperty("cases").EnumerateArray())
        {
            var name = c.GetProperty("name").GetString()!;
            using var db = Sqlite.Memory();
            db.Migrate([[file.GetProperty("table").GetString()!]]);
            var copies = new Dictionary<string, Item>();
            long last = 0;
            foreach (var step in c.GetProperty("steps").EnumerateArray())
            {
                var doing = step.GetProperty("do").GetString();
                var copy = step.TryGetProperty("copy", out var k) ? k.GetString()! : "";
                string Text(string field) => step.GetProperty(field).GetString()!;
                switch (doing)
                {
                    case "insert":
                        var item = new Item { Name = Text("name") };
                        Expect(name, step, file.GetProperty("refusal"), () => db.Save(item));
                        copies[copy] = item;
                        break;
                    case "read":
                        copies[copy] = db.Get<Item>(copies[Text("of")].Id);
                        break;
                    case "set":
                        copies[copy].Name = Text("name");
                        break;
                    case "save":
                        Expect(name, step, file.GetProperty("refusal"), () => db.Save(copies[copy]));
                        break;
                    case "saveAnyway":
                        Expect(name, step, file.GetProperty("refusal"), () => db.SaveAnyway(copies[copy]));
                        break;
                    case "delete":
                        Expect(name, step, file.GetProperty("refusal"), () => db.Delete(copies[copy]));
                        break;
                    case "reload":
                        Expect(name, step, file.GetProperty("refusal"), () => copies[copy] = db.Reload(copies[copy]));
                        break;
                    case "version":
                        Assert.Equal(step.GetProperty("is").GetInt64(), copies[copy].Version);
                        break;
                    case "stored":
                        var rows = db.Rows("", "SELECT name, row_version FROM item WHERE id = ?1", [copies[copy].Id]);
                        if (step.TryGetProperty("gone", out var gone) && gone.GetBoolean())
                            Assert.Empty(rows);
                        else
                        {
                            Assert.Single(rows);
                            Assert.Equal(Text("name"), rows[0][0]);
                            Assert.Equal(step.GetProperty("version").GetInt64(), Sql.GetInteger("version", rows[0][1]));
                        }
                        break;
                    case "stepSave":
                        Expect(name, step, file.GetProperty("refusal"), () => db.Step(() => db.Save(copies[copy])));
                        last = db.LastStep;
                        break;
                    case "undo":
                        Expect(name, step, file.GetProperty("undoRefusal"), () => db.Undo(last));
                        break;
                    case "redo":
                        Expect(name, step, file.GetProperty("undoRefusal"), () => db.Redo(last));
                        break;
                    default:
                        Assert.Fail($"{name}: no step {doing}");
                        break;
                }
            }
        }
    }

    [Fact]
    public void A_handler_is_one_transaction_kept_at_its_end()
    {
        using var db = Sqlite.Memory();
        db.Migrate([[UndoTable]]);
        var a = new Item { Name = "A" };
        db.Save(a);
        // A failure after two saves rolls both back, with the database's own rollback.
        Assert.Throws<StorageException>(() => db.Step(() =>
        {
            a.Name = "changed";
            db.Save(a);
            db.Save(new Item { Name = "B" });
            throw new StorageException("no");
        }));
        var rows = db.Rows("", "SELECT name, row_version FROM item", []);
        Assert.Single(rows);
        Assert.Equal("A", rows[0][0]);
        Assert.Equal(1L, Sql.GetInteger("version", rows[0][1]));
    }

    [Fact]
    public void The_steps_of_one_edit_are_undone_all_or_none()
    {
        using var db = Sqlite.Memory();
        db.Migrate([[UndoTable]]);
        Item a = new() { Name = "A" }, b = new() { Name = "B" };
        db.Save(a);
        db.Save(b);
        db.Step(() => { a.Name = "A2"; db.Save(a); });
        var first = db.LastStep;
        db.Step(() => { b.Name = "B2"; db.Save(b); });
        var second = db.LastStep;
        // B changed since the edit, so neither step is taken back.
        db.Save(db.Get<Item>(b.Id));
        var refused = Assert.Throws<RefusedException>(() => db.ReplaySteps([second, first], back: true));
        Assert.Equal("undoChangedSince", refused.Text.Key);
        Assert.Equal("A2", db.Get<Item>(a.Id).Name);
    }

    [Fact]
    public void Only_the_last_steps_are_kept()
    {
        using var db = Sqlite.Memory();
        db.Migrate([[UndoTable]]);
        var a = new Item { Name = "A" };
        db.Save(a);
        for (var i = 0; i <= Db.UndoSteps; i++)
            db.Step(() => { a.Name = $"A{i}"; db.Save(a); });
        var refused = Assert.Throws<RefusedException>(() => db.Undo(1));
        Assert.Equal("undoTooOld", refused.Text.Key);
        Assert.True(db.Undo(db.LastStep));
    }
}

/// <summary>An item whose entity words the refusal itself, about the record as stored now and what the database holds (as <c>{current.changedBy.name}</c> needs).</summary>
sealed record Worded : IRecord<Worded>
{
    public static string TableName => "item";
    public static string? VersionColumn => "row_version";
    public static IReadOnlyList<string> ColumnNames { get; } = ["name"];
    public long Id { get; set; }
    public long Version { get; set; }
    public string Name { get; set; } = "";
    public object?[] ToValues() => [Name];
    public static Worded FromValues(long id, object?[] values) => new() { Id = id, Name = Sql.GetText("name", values[0]) };
    public static Text SavedSince(Db db, Worded current) => new($"{current.Name} saved it; {db.All<Worded>().Count} records");
}

public class OwnWordsTests
{
    [Fact]
    public void An_entity_words_the_refusal_with_the_database_at_hand()
    {
        using var db = Sqlite.Memory();
        db.Migrate([["CREATE TABLE item (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL, row_version INTEGER NOT NULL DEFAULT 1)"]]);
        var first = new Worded { Name = "first" };
        db.Save(first);
        var second = db.Get<Worded>(first.Id);
        first.Name = "Anna";
        db.Save(first);
        Assert.Equal("Anna saved it; 1 records", Assert.Throws<RefusedException>(() => db.Save(second)).Text.Template);
        Assert.Equal("Anna saved it; 1 records", Assert.Throws<RefusedException>(() => db.Delete(second)).Text.Template);
    }
}
