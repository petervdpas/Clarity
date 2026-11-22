// A backup before migration steps change a database (docs/ledger/backups.md),
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

}
