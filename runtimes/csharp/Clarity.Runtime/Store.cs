// The connection to one database, behind the same few calls for every kind:
// run a statement, insert and get the new key, read rows, and apply a
// migration step. SQLite is always there; a generated project holds the
// server drivers it uses (CLARITY_POSTGRES, CLARITY_MYSQL, CLARITY_MSSQL)
// and their packages only.

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

    /// <summary>
    /// Runs the statements of migration step <paramref name="version"/> and records
    /// that the database has it, all or nothing where the database can undo
    /// changes to tables.
    /// </summary>
    void Apply(int version, IReadOnlyList<string> statements);
}

internal static class Store
{
    /// <summary>The statement that records migration step <paramref name="version"/>.</summary>
    internal static string Applied(int version) => $"INSERT INTO clarity_schema (version) VALUES ({version})";

    /// <summary>
    /// The location of a database: as written, or from the environment variable
    /// that <c>env:NAME</c> names, so passwords stay out of the program.
    /// </summary>
    static string Location(string text)
    {
        if (!text.StartsWith("env:", StringComparison.Ordinal))
            return text;
        var name = text[4..];
        return Environment.GetEnvironmentVariable(name) ?? throw new StorageException($"the environment variable {name} is not set");
    }

    /// <summary>Connects to the database of a data source.</summary>
    internal static IDriver Open(Dialect dialect, string at) => dialect switch
    {
        Dialect.Sqlite => SqliteDriver.Open(Location(at)),
#if CLARITY_POSTGRES
        Dialect.Postgres => PostgresDriver.Open(Location(at)),
#endif
#if CLARITY_MYSQL
        Dialect.Mysql => MysqlDriver.Open(Location(at)),
#endif
#if CLARITY_MSSQL
        Dialect.Mssql => MssqlDriver.Open(Location(at)),
#endif
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
