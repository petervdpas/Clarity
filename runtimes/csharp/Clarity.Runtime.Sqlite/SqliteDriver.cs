// SQLite: one file per data source. Decimals and dates are stored as text,
// bools as 0 and 1.

using System.Globalization;
using Microsoft.Data.Sqlite;

namespace Clarity.Runtime;

internal sealed class SqliteDriver(SqliteConnection conn) : IDriver
{
    /// <summary>SQLITE_CONSTRAINT_FOREIGNKEY.</summary>
    const int ForeignKey = 787;

    public Dialect Dialect => Dialect.Sqlite;

    internal static SqliteDriver Open(string path)
    {
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

    /// <summary>
    /// References are not enforced while a step runs, since a step may build a
    /// table again that others refer to; they are checked before the step is kept.
    /// </summary>
    public void Apply(int version, IReadOnlyList<string> statements)
    {
        Batch("PRAGMA foreign_keys = OFF");
        try
        {
            Run(() =>
            {
                using var tx = conn.BeginTransaction();
                foreach (var statement in statements)
                {
                    using var command = Command(statement, [], tx);
                    command.ExecuteNonQuery();
                }
                using (var check = Command("SELECT COUNT(*) FROM pragma_foreign_key_check", [], tx))
                {
                    var broken = (long)check.ExecuteScalar()!;
                    if (broken > 0)
                        throw new StorageException($"migration step {version} leaves {broken} reference(s) to records that do not exist");
                }
                using (var applied = Command(Store.Applied(version), [], tx))
                    applied.ExecuteNonQuery();
                tx.Commit();
                return 0;
            });
        }
        finally
        {
            Batch("PRAGMA foreign_keys = ON");
        }
    }

    public void Dispose() => conn.Dispose();
}
