// A backup before migration steps change a database (docs/BACKUP-PLAN.md),
// named and pruned as every stack does it (conformance/backups.json).

using System.Text.Json;
using Clarity.Runtime;
using Xunit;

namespace Clarity.Runtime.Tests;

public class BackupTests
{
    static JsonElement Cases() =>
        JsonDocument.Parse(File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "backups.json"))).RootElement;

    static string Folder(string name)
    {
        var dir = Path.Combine(Path.GetTempPath(), $"clarity-cs-backup-{name}-{Environment.ProcessId}");
        if (Directory.Exists(dir))
            Directory.Delete(dir, true);
        Directory.CreateDirectory(dir);
        return dir;
    }

    [Fact]
    public void BackupsAreNamedAndPrunedAsEveryStackDoes()
    {
        var c = Cases();
        foreach (var s in c.GetProperty("stamps").EnumerateArray())
            Assert.Equal(s.GetProperty("stamp").GetString(), Backup.Stamp(s.GetProperty("seconds").GetInt64()));
        foreach (var n in c.GetProperty("names").EnumerateArray())
            Assert.Equal(n.GetProperty("file").GetString(), Backup.FileName(n.GetProperty("database").GetString()!, n.GetProperty("seconds").GetInt64(), n.GetProperty("step").GetInt32()));
        Assert.Equal(Backup.KeptAutomatic, c.GetProperty("kept").GetInt32());
        var prune = c.GetProperty("prune");
        var dir = Folder("prune");
        Directory.CreateDirectory(Path.Combine(dir, "backups"));
        foreach (var f in prune.GetProperty("files").EnumerateArray())
            File.WriteAllText(Path.Combine(dir, "backups", f.GetString()!), "");
        Backup.Prune(Path.Combine(dir, prune.GetProperty("database").GetString() + ".db"));
        var left = Directory.GetFiles(Path.Combine(dir, "backups")).Select(Path.GetFileName).Order(StringComparer.Ordinal).ToList();
        Assert.Equal(prune.GetProperty("left").EnumerateArray().Select(f => f.GetString()).ToList(), left);
        Directory.Delete(dir, true);
    }

    [Fact]
    public void ADatabaseWithASchemaIsBackedUpBeforeNewStepsAndANewOneIsNot()
    {
        var dir = Folder("migrate");
        var path = Path.Combine(dir, "shop.db");
        string[] first = ["CREATE TABLE item (id INTEGER PRIMARY KEY, name TEXT NOT NULL DEFAULT '')"];
        string[] second = ["ALTER TABLE item ADD COLUMN price INTEGER NOT NULL DEFAULT 0"];
        using (var db = Db.Connect(("main", Dialect.Sqlite, path)))
        {
            db.Migrate([first]);
            db.Batch("", "INSERT INTO item (name) VALUES ('kept')");
        }
        Assert.False(Directory.Exists(Path.Combine(dir, "backups")), "a new database needs no backup");
        using (var db = Db.Connect(("main", Dialect.Sqlite, path)))
            Assert.Equal(1, db.Migrate([first, second]));
        var backups = Directory.GetFiles(Path.Combine(dir, "backups"));
        var name = Path.GetFileName(Assert.Single(backups));
        Assert.StartsWith("shop-", name);
        Assert.EndsWith("-before-migration-0002.db", name);
        using (var copy = Db.Connect(("main", Dialect.Sqlite, backups[0])))
        {
            Assert.Equal(1, copy.SchemaVersion());
            Assert.ThrowsAny<ClarityException>(() => copy.Batch("", "SELECT price FROM item"));
        }
        Microsoft.Data.Sqlite.SqliteConnection.ClearAllPools();
        Directory.Delete(dir, true);
    }
}
