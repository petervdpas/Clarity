// What the server databases share over ADO.NET: commands with the
// dialect's parameter marks, values read back as the runtime's types, and
// a reference error told apart from other failures. Each database adds how
// it gives the new key, which error is a broken reference, and how it runs
// a migration step.

using System.Data.Common;

namespace Clarity.Runtime;

internal abstract class ServerDriver(DbConnection conn) : IDriver
{
    protected DbConnection Conn { get; } = conn;

    public abstract Dialect Dialect { get; }

    /// <summary>Whether the error means a record refers to one that is not there, or is referred to.</summary>
    protected abstract bool IsReference(DbException e);

    /// <summary>A value as the database's driver takes it.</summary>
    protected virtual object Stored(object? value) => value switch
    {
        null => DBNull.Value,
        Enum e => e.ToString(),
        var other => other,
    };

    /// <summary>The name a parameter has in a command: none where the marks are positional.</summary>
    protected virtual string? ParameterName(int n) => null;

    protected DbCommand Command(string sql, IReadOnlyList<object?> parameters, DbTransaction? tx = null)
    {
        var command = Conn.CreateCommand();
        command.CommandText = sql;
        command.Transaction = tx;
        for (var i = 0; i < parameters.Count; i++)
        {
            var parameter = command.CreateParameter();
            if (ParameterName(i + 1) is { } name)
                parameter.ParameterName = name;
            parameter.Value = Stored(parameters[i]);
            command.Parameters.Add(parameter);
        }
        return command;
    }

    /// <summary>Runs <paramref name="run"/>, telling a broken reference apart from other failures.</summary>
    protected T Run<T>(Func<T> run)
    {
        try
        {
            return run();
        }
        catch (DbException e) when (IsReference(e))
        {
            throw new ReferenceFailure(e.Message, e);
        }
        catch (DbException e)
        {
            throw new StorageException(e.Message, e);
        }
    }

    public int Execute(string sql, IReadOnlyList<object?> parameters) => Run(() =>
    {
        using var command = Command(sql, parameters);
        return command.ExecuteNonQuery();
    });

    public virtual long Insert(string sql, IReadOnlyList<object?> parameters) => Run(() =>
    {
        // The dialect's INSERT gives the new key back.
        using var command = Command(sql, parameters);
        return Sql.GetInteger("id", Read(command.ExecuteScalar()));
    });

    /// <summary>A value as the runtime holds it: whole numbers as long, a moment as DateTime.</summary>
    protected static object? Read(object? value) => value switch
    {
        null or DBNull => null,
        int n => (long)n,
        short n => (long)n,
        byte n => (long)n,
        sbyte n => (long)n,
        uint n => (long)n,
        ushort n => (long)n,
        ulong n => (long)n,
        float r => (double)r,
        DateTimeOffset d => d.DateTime,
        var other => other,
    };

    public List<object?[]> Rows(string sql, IReadOnlyList<object?> parameters) => Run(() =>
    {
        using var command = Command(sql, parameters);
        using var reader = command.ExecuteReader();
        var rows = new List<object?[]>();
        while (reader.Read())
        {
            var row = new object?[reader.FieldCount];
            for (var i = 0; i < row.Length; i++)
                row[i] = reader.IsDBNull(i) ? null : Read(reader.GetValue(i));
            rows.Add(row);
        }
        return rows;
    });

    public void Batch(string sql) => Run(() =>
    {
        using var command = Command(sql, []);
        return command.ExecuteNonQuery();
    });

    /// <summary>Runs every statement of a step and records it, all or nothing.</summary>
    public virtual void Apply(int version, IReadOnlyList<string> statements) => Run(() =>
    {
        using var tx = Conn.BeginTransaction();
        foreach (var statement in statements)
        {
            using var command = Command(statement, [], tx);
            command.ExecuteNonQuery();
        }
        using (var applied = Command(Store.Applied(version), [], tx))
            applied.ExecuteNonQuery();
        tx.Commit();
        return 0;
    });

    public void Dispose() => Conn.Dispose();
}
