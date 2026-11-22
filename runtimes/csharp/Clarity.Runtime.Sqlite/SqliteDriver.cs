// SQLite: one file per data source. Decimals and dates are stored as text,
// bools as 0 and 1.

using System.Globalization;
using Microsoft.Data.Sqlite;

namespace Clarity.Runtime;

internal sealed class SqliteDriver(SqliteConnection conn) : IDriver, ITransactional
{
    /// <summary>SQLITE_CONSTRAINT_FOREIGNKEY.</summary>
    const int ForeignKey = 787;

    public Dialect Dialect => Dialect.Sqlite;

    /// <summary>Whether a transaction is open: SQLite is out of autocommit.</summary>
    public bool InTransaction => conn.Handle is { } handle && SQLitePCL.raw.sqlite3_get_autocommit(handle) == 0;

    internal static SqliteDriver Open(string path)
    {
        if (path == ":memory:")
            return Memory();
        var dir = Path.GetDirectoryName(Path.GetFullPath(path));
        if (!string.IsNullOrEmpty(dir))
            Directory.CreateDirectory(dir);
        return New(new SqliteConnection(new SqliteConnectionStringBuilder { DataSource = path }.ToString()));
    }

    /// <summary>A database that lives in memory, for tests.</summary>
    internal static SqliteDriver Memory() => New(new SqliteConnection("Data Source=:memory:"));

    static SqliteDriver New(SqliteConnection conn)
    {
        conn.Open();
        var driver = new SqliteDriver(conn);
        driver.Batch("PRAGMA foreign_keys = ON");
        return driver;
    }

    static object Stored(object? value) => value switch
    {
        null => DBNull.Value,
        decimal d => d.ToString(CultureInfo.InvariantCulture),
        bool b => b ? 1L : 0L,
        DateOnly d => d.ToString(Sql.DateFormat, CultureInfo.InvariantCulture),
        DateTime d => d.ToString(Sql.DateTimeFormat, CultureInfo.InvariantCulture),
        Enum e => e.ToString(),
        var other => other,
    };

    SqliteCommand Command(string sql, IReadOnlyList<object?> parameters, SqliteTransaction? tx = null)
    {
        var command = conn.CreateCommand();
        command.CommandText = sql;
        command.Transaction = tx;
        for (var i = 0; i < parameters.Count; i++)
            command.Parameters.AddWithValue($"?{i + 1}", Stored(parameters[i]));
        return command;
    }

    /// <summary>Runs <paramref name="run"/>, telling a broken reference apart from other failures.</summary>
    static T Run<T>(Func<T> run)
    {
        try
        {
            return run();
        }
        catch (SqliteException e) when (e.SqliteExtendedErrorCode == ForeignKey)
        {
            throw new ReferenceFailure(e.Message, e);
        }
        catch (SqliteException e)
        {
            throw new StorageException(e.Message, e);
        }
    }

    public int Execute(string sql, IReadOnlyList<object?> parameters) => Run(() =>
    {
        using var command = Command(sql, parameters);
        return command.ExecuteNonQuery();
    });

    public long Insert(string sql, IReadOnlyList<object?> parameters) => Run(() =>
    {
        using var command = Command(sql + "; SELECT last_insert_rowid()", parameters);
        return (long)command.ExecuteScalar()!;
    });

    public List<object?[]> Rows(string sql, IReadOnlyList<object?> parameters) => Run(() =>
    {
        using var command = Command(sql, parameters);
        using var reader = command.ExecuteReader();
        var rows = new List<object?[]>();
        while (reader.Read())
        {
            var row = new object?[reader.FieldCount];
            for (var i = 0; i < row.Length; i++)
                row[i] = reader.IsDBNull(i) ? null : reader.GetValue(i);
            rows.Add(row);
        }
        return rows;
    });

    public void Batch(string sql) => Run(() =>
    {
        using var command = Command(sql, []);
        return command.ExecuteNonQuery();
    });

    public void Dispose() => conn.Dispose();
}
