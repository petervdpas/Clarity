// The connection to one database, behind the same few calls for every kind:
// run a statement, insert and get the new key, read rows, and apply a
// migration step. Each kind of database is a package of its own
// (Clarity.Runtime.Sqlite, .PostgreSql, .MySql, .SqlServer), which a
// program turns on with its Use() before it opens its databases.

using System.Data.Common;

namespace Clarity.Runtime;

/// <summary>A reference between records would no longer fit.</summary>
internal sealed class ReferenceFailure(string what, Exception inner) : Exception(what, inner);

internal interface IDriver : IDisposable
{
    Dialect Dialect { get; }

    /// <summary>Runs a statement and gives the number of rows it changed.</summary>
    int Execute(string sql, IReadOnlyList<object?> parameters);

    /// <summary>Runs an INSERT from <c>Dialect.Insert</c> and gives the new record's key.</summary>
    long Insert(string sql, IReadOnlyList<object?> parameters);

    List<object?[]> Rows(string sql, IReadOnlyList<object?> parameters);

    /// <summary>Runs SQL without parameters, such as a statement that creates a table.</summary>
    void Batch(string sql);
}

/// <summary>A driver that can say whether a transaction is open, as a file source asks of its SQLite database.</summary>
internal interface ITransactional
{
    bool InTransaction { get; }
}

internal static class Store
{
    /// <summary>
    /// The location of a database: as written, or from the environment variable
    /// that <c>env:NAME</c> names, so passwords stay out of the program.
    /// </summary>
    internal static string Location(string text)
    {
        // A secret setting (manual/data.settings.md): setting:<program>/<name>, from the system's protected store.
        if (text.StartsWith("setting:", StringComparison.Ordinal))
        {
            var at = text[8..].Split('/', 2);
            var setting = at.Length == 2 ? at[1] : at[0];
            var value = at.Length == 2 ? Secrets.Get(at[0], setting) : null;
            return string.IsNullOrEmpty(value) ? throw new NotKeptException(setting) : value;
        }
        if (!text.StartsWith("env:", StringComparison.Ordinal))
            return text;
        var name = text[4..];
        return Environment.GetEnvironmentVariable(name) is { Length: > 0 } found ? found : throw new NotSetException(name);
    }

    /// <summary>The kinds of database the application turned on.</summary>
    static readonly Dictionary<Dialect, Func<string, IDriver>> drivers = [];

    /// <summary>Turns on a kind of database: what its package's Use() does.</summary>
    internal static void Use(Dialect dialect, Func<string, IDriver> open) => drivers[dialect] = open;

    /// <summary>Connects to the database of a data source.</summary>
    internal static IDriver Open(Dialect dialect, string at) => dialect switch
    {
        _ when drivers.TryGetValue(dialect, out var open) => open(Location(dialect == Dialect.Sqlite ? DataFolder.Sqlite(at) : at)),
        _ => throw new StorageException($"this application was built without {dialect.Label()}"),
    };

    /// <summary>Opens a server connection; what goes wrong reads as a storage failure.</summary>
    internal static void Connect(DbConnection conn, Dialect dialect)
    {
        try
        {
            conn.Open();
        }
        catch (Exception e) when (e is DbException or InvalidOperationException or ArgumentException)
        {
            conn.Dispose();
            throw new StorageException($"cannot connect to {dialect.Label()}: {e.Message}", e);
        }
    }
}

/// <summary>
/// Where a program's SQLite databases open (conformance/data-folder.json): a
/// database the project keeps beside it is written data:&lt;program&gt;/&lt;file&gt;,
/// and opens in the folder CLARITY_DATA names (Clarity's Run names the
/// project's data folder), else in the person's folder for application data
/// under the program's name. CLARITY_DATA moves any SQLite file there under
/// its own file name. The same in every runtime.
/// </summary>
public static class DataFolder
{
    /// <summary>The environment variable that names the folder.</summary>
    public const string Variable = "CLARITY_DATA";

    /// <summary>The file of an SQLite database at <paramref name="location"/> on this system; <paramref name="env"/> reads the environment (the process's when none).</summary>
    public static string Sqlite(string location, Func<string, string?>? env = null) =>
        SqliteOn(OperatingSystem.IsWindows() ? "windows" : OperatingSystem.IsMacOS() ? "macos" : "linux", location, env);

    /// <summary>The file of an SQLite database at <paramref name="location"/> on system <paramref name="os"/> (windows, macos or another); env:, setting: and :memory: stay as they are.</summary>
    public static string SqliteOn(string os, string location, Func<string, string?>? env = null)
    {
        if (location.StartsWith("env:", StringComparison.Ordinal) || location.StartsWith("setting:", StringComparison.Ordinal) || location == ":memory:")
            return location;
        env ??= Environment.GetEnvironmentVariable;
        string? Set(string name) => env(name) is { Length: > 0 } value ? value : null;
        var kept = location.StartsWith("data:", StringComparison.Ordinal) ? location["data:".Length..] : null;
        var name = kept is null ? location : kept[(kept.LastIndexOf('/') + 1)..];
        if (Set(Variable) is { } moved && Path.GetFileName(name) is { Length: > 0 } file)
            return Path.Combine(moved, file);
        if (kept is null) return location;
        // The person's folder for application data, under the program's name.
        var folder = os switch
        {
            "windows" => Set("APPDATA"),
            "macos" => Set("HOME") is { } home ? home + "/Library/Application Support" : null,
            _ => Set("XDG_DATA_HOME") ?? (Set("HOME") is { } home ? home + "/.local/share" : null),
        };
        return folder is null ? name : folder + "/" + kept;
    }

    static string ThisOs => OperatingSystem.IsWindows() ? "windows" : OperatingSystem.IsMacOS() ? "macos" : "linux";

    /// <summary>
    /// The program's data folder (conformance/data-folder.json, folders): the folder
    /// CLARITY_DATA names, else the person's folder for application data under the program's
    /// name; "" when the environment names neither, which is where the program runs. A relative
    /// file store and a relative folder setting are in it, as a relative SQLite database is.
    /// </summary>
    public static string Folder(string program, Func<string, string?>? env = null) => FolderOn(ThisOs, program, env);

    /// <summary><see cref="Folder"/> on system <paramref name="os"/> (windows, macos or another).</summary>
    public static string FolderOn(string os, string program, Func<string, string?>? env = null)
    {
        env ??= Environment.GetEnvironmentVariable;
        string? Set(string name) => env(name) is { Length: > 0 } value ? value : null;
        if (Set(Variable) is { } moved)
            return moved;
        var folder = os switch
        {
            "windows" => Set("APPDATA"),
            "macos" => Set("HOME") is { } home ? home + "/Library/Application Support" : null,
            _ => Set("XDG_DATA_HOME") ?? (Set("HOME") is { } home ? home + "/.local/share" : null),
        };
        return folder is null ? "" : folder + "/" + program;
    }

    /// <summary>
    /// The folder of a file store at <paramref name="location"/> (conformance/data-folder.json,
    /// stores): <c>data:&lt;program&gt;/&lt;name&gt;</c> (a relative folder in the project file) is in
    /// the program's data folder under its own name; <c>env:NAME</c> is that variable's value, a
    /// <see cref="NotSetException"/> while it is unset or empty; anything else is as it is.
    /// </summary>
    public static string Store(string location, Func<string, string?>? env = null) => StoreOn(ThisOs, location, env);

    /// <summary><see cref="Store"/> on system <paramref name="os"/>.</summary>
    public static string StoreOn(string os, string location, Func<string, string?>? env = null)
    {
        env ??= Environment.GetEnvironmentVariable;
        if (location.StartsWith("env:", StringComparison.Ordinal))
        {
            var variable = location[4..];
            return env(variable) is { Length: > 0 } value ? value : throw new NotSetException(variable);
        }
        if (!location.StartsWith("data:", StringComparison.Ordinal))
            return location;
        var kept = location[5..];
        var slash = kept.IndexOf('/');
        var (program, name) = slash < 0 ? ("", kept) : (kept[..slash], kept[(slash + 1)..]);
        return InFolder(FolderOn(os, program, env), name);
    }

    /// <summary><paramref name="name"/> in <paramref name="folder"/>; just the name when there is no folder.</summary>
    public static string InFolder(string folder, string name) => folder.Length == 0 ? name : folder + "/" + name;
}
