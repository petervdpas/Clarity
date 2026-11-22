// A backup of an SQLite database before migration steps change it
// (docs/ledger/backups.md): a copy beside it in backups/, made with VACUUM
// INTO, named <database>-<when>-before-migration-NNNN.db with the time in
// UTC, as Clarity and every runtime name it (conformance/backups.json). The
// newest ten automatic backups are kept; a manual one is the person's and
// stays.

using System.Globalization;

namespace Clarity.Runtime;

public static class Backup
{
    /// <summary>How many automatic backups each database keeps.</summary>
    public const int KeptAutomatic = 10;

    /// <summary>A moment, in seconds since 1970, as a backup names it: <c>20261105T205905Z</c>.</summary>
    public static string Stamp(long seconds) =>
        DateTimeOffset.FromUnixTimeSeconds(seconds).UtcDateTime.ToString("yyyyMMdd'T'HHmmss'Z'", CultureInfo.InvariantCulture);

    /// <summary>The file of a backup of the database named <paramref name="database"/> (its file's name without .db) taken at <paramref name="seconds"/> before <paramref name="step"/>.</summary>
    public static string FileName(string database, long seconds, int step) =>
        $"{database}-{Stamp(seconds)}-before-migration-{step.ToString("D4", CultureInfo.InvariantCulture)}.db";

    /// <summary>A backup file of the database named <paramref name="database"/>: its stamp, count within that second, and whether it is automatic.</summary>
    static (string When, int Count, bool Automatic)? Parse(string database, string file)
    {
        if (!file.StartsWith(database + "-", StringComparison.Ordinal) || !file.EndsWith(".db", StringComparison.Ordinal))
            return null;
        var rest = file[(database.Length + 1)..^3];
        var dash = rest.IndexOf('-');
        if (dash != 16 || rest[15] != 'Z')
            return null;
        var (when, why) = (rest[..16], rest[17..]);
        var count = 1;
        var under = why.IndexOf('_');
        if (under >= 0)
        {
            if (!int.TryParse(why[(under + 1)..], NumberStyles.None, CultureInfo.InvariantCulture, out count))
                return null;
            why = why[..under];
        }
        return (when, count, why != "manual");
    }

    /// <summary>Removes the automatic backups of <paramref name="database"/> beyond the newest ten; manual ones, other databases' and other files stay.</summary>
    public static void Prune(string database)
    {
        var name = Path.GetFileNameWithoutExtension(database);
        var dir = Path.Combine(Path.GetDirectoryName(database) ?? ".", "backups");
        if (!Directory.Exists(dir))
            return;
        var automatic = Directory.GetFiles(dir)
            .Select(path => (Path: path, Found: Parse(name, Path.GetFileName(path))))
            .Where(f => f.Found is { Automatic: true })
            .OrderByDescending(f => f.Found!.Value.When, StringComparer.Ordinal)
            .ThenByDescending(f => f.Found!.Value.Count)
            .Skip(KeptAutomatic);
        foreach (var old in automatic)
        {
            try { File.Delete(old.Path); } catch (IOException) { }
        }
    }

}
