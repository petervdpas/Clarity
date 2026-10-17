// The databases of an application: one per data source. Records are saved,
// deleted and read through here, with the entity's data events around a
// save or delete, and migration steps are applied here at startup.

namespace Clarity.Runtime;

public sealed class Db : IDisposable
{
    /// <summary>The database of one data source.</summary>
    sealed class SourceDb(string name, IDriver driver)
    {
        public string Name { get; } = name;
        public IDriver Driver { get; } = driver;

        /// <summary>
        /// How many saves and deletes with data events are running in it; the
        /// outermost one holds the transaction.
        /// </summary>
        public int Depth { get; set; }
    }

    /// <summary>By data source name; the first is the default.</summary>
    readonly List<SourceDb> sources;

    Db(List<SourceDb> sources) => this.sources = sources;

    /// <summary>One SQLite database, used for every record.</summary>
    public static Db Open(string path) => new([new SourceDb("", SqliteDriver.Open(path))]);

    /// <summary>A SQLite database that lives in memory, for tests.</summary>
    public static Db Memory() => new([new SourceDb("", SqliteDriver.Memory())]);

    /// <summary>
    /// A database per data source: its name, its kind and where it is (a file
    /// for SQLite, a connection for the others). <c>env:NAME</c> takes the
    /// location from that environment variable.
    /// </summary>
    public static Db Connect(params (string Name, Dialect Dialect, string At)[] sources) =>
        new(sources.Select(s => new SourceDb(s.Name, Store.Open(s.Dialect, s.At))).ToList());

    /// <summary>The data source <paramref name="source"/>. With one database, every record is in it.</summary>
    SourceDb At(string source)
    {
        var single = sources.Count == 1;
        return sources.Find(s => single || source == "" || s.Name == source)
            ?? throw new StorageException($"there is no data source '{source}'");
    }

    IDriver Conn(string source) => At(source).Driver;

    /// <summary>
    /// Runs <paramref name="run"/> in a transaction of <paramref name="source"/>, or in
    /// the one that is open already: a save that an event of another save makes is part of it.
    /// </summary>
    void Atomic(string source, Action run)
    {
        var at = At(source);
        var outer = at.Depth == 0;
        if (outer)
            at.Driver.Batch(at.Driver.Dialect.Begin());
        at.Depth++;
        try
        {
            run();
        }
        catch
        {
            at.Depth--;
            if (outer)
            {
                // The error that made it roll back is the one to report.
                try
                {
                    at.Driver.Batch(at.Driver.Dialect.Rollback());
                }
                catch (ClarityException)
                {
                }
            }
            throw;
        }
        at.Depth--;
        if (outer)
            at.Driver.Batch(at.Driver.Dialect.Commit());
    }

    /// <summary>Runs SQL without parameters in the database of <paramref name="source"/>, for tools and tests.</summary>
    public void Batch(string source, string sql) => Conn(source).Batch(sql);

    /// <summary>The kind of database <paramref name="source"/> is.</summary>
    public Dialect DialectOf(string source) => Conn(source).Dialect;

    /// <summary>The number of migration steps applied so far to the default database.</summary>
    public int SchemaVersion() => SchemaVersion(Conn(""));

    /// <summary><see cref="MigrateSource"/> for the default database.</summary>
    public int Migrate(IReadOnlyList<string[]> steps) => MigrateSource("", steps);

    /// <summary>Applies the steps of <paramref name="source"/> its database has not had yet, each on its own.</summary>
    public int MigrateSource(string source, IReadOnlyList<string[]> steps)
    {
        var conn = Conn(source);
        var done = SchemaVersion(conn);
        for (var i = done; i < steps.Count; i++)
            conn.Apply(i + 1, steps[i]);
        return Math.Max(0, steps.Count - done);
    }

    /// <summary>
    /// Inserts a new record (and gives it its id) or updates a stored one, with
    /// the entity's data events around it.
    /// </summary>
    public void Save<R>(R record) where R : IRecord<R>
    {
        if (!R.HasEvents)
        {
            Write(record);
            return;
        }
        var id = record.Id;
        try
        {
            Atomic(R.SourceName, () =>
            {
                record.BeforeSave(this);
                Write(record);
                record.AfterSave(this);
            });
        }
        catch
        {
            // Nothing was kept, so a new record is still new.
            record.Id = id;
            throw;
        }
    }

    void Write<R>(R record) where R : IRecord<R>
    {
        var conn = Conn(R.SourceName);
        var d = conn.Dialect;
        var values = record.ToValues();
        if (record.Id == 0)
        {
            record.Id = Unfit(() => conn.Insert(d.Insert(R.TableName, R.KeyColumn, R.ColumnNames), values));
            return;
        }
        var sets = string.Join(", ", R.ColumnNames.Select((c, i) => $"{d.Quote(c)} = {d.Mark(i + 1)}"));
        var sql = $"UPDATE {d.Quote(R.TableName)} SET {sets} WHERE {d.Quote(R.KeyColumn)} = {d.Mark(values.Length + 1)}";
        object?[] parameters = [.. values, record.Id];
        // MySQL counts only the rows whose values changed, so an update that
        // changes nothing is told apart from a missing record.
        if (Unfit(() => conn.Execute(sql, parameters)) == 0 && !Exists<R>(conn, record.Id))
            throw new NotFoundException(R.TableName, record.Id);
    }

    /// <summary>A failed save: a reference to a record that is not there reads better than the database's own words.</summary>
    static T Unfit<T>(Func<T> run)
    {
        try
        {
            return run();
        }
        catch (ReferenceFailure e)
        {
            throw new InvalidValueException($"a reference does not fit: {e.Message}");
        }
    }

    /// <summary>Removes a stored record, with the entity's data events around it.</summary>
    public void Delete<R>(R record) where R : IRecord<R>
    {
        if (!R.HasEvents)
        {
            Remove(record);
            return;
        }
        Atomic(R.SourceName, () =>
        {
            record.BeforeDelete(this);
            Remove(record);
            record.AfterDelete(this);
        });
    }

    void Remove<R>(R record) where R : IRecord<R>
    {
        var conn = Conn(R.SourceName);
        var d = conn.Dialect;
        try
        {
            conn.Execute($"DELETE FROM {d.Quote(R.TableName)} WHERE {d.Quote(R.KeyColumn)} = {d.Mark(1)}", [record.Id]);
        }
        catch (ReferenceFailure)
        {
            throw new InUseException(R.TableName, record.Id);
        }
    }

    /// <summary>The records a query finds, in its order.</summary>
    public List<R> Find<R>(Query query) where R : IRecord<R>
    {
        var conn = Conn(R.SourceName);
        var (sql, parameters) = query.Sql(conn.Dialect, R.TableName, R.KeyColumn, SelectColumns<R>(conn.Dialect));
        return Records<R>(conn, sql, parameters);
    }

    /// <summary>Every stored record of an entity, in the order they were stored.</summary>
    public List<R> All<R>() where R : IRecord<R> => Find<R>(new Query());

    public R Get<R>(long id) where R : IRecord<R>
    {
        var conn = Conn(R.SourceName);
        var d = conn.Dialect;
        var sql = $"SELECT {SelectColumns<R>(d)} FROM {d.Quote(R.TableName)} WHERE {d.Quote(R.KeyColumn)} = {d.Mark(1)}";
        return Records<R>(conn, sql, [id]).LastOrDefault() ?? throw new NotFoundException(R.TableName, id);
    }

    /// <summary>The record a reference field points at.</summary>
    public R Load<R>(long? id) where R : IRecord<R> =>
        id is { } n ? Get<R>(n) : throw new NotFoundException(R.TableName, null);

    /// <summary>The key and the stored columns of a record, for SELECT.</summary>
    static string SelectColumns<R>(Dialect d) where R : IRecord<R> =>
        string.Join(", ", R.ColumnNames.Prepend(R.KeyColumn).Select(c => d.Quote(c)));

    static List<R> Records<R>(IDriver conn, string sql, IReadOnlyList<object?> parameters) where R : IRecord<R> =>
        conn.Rows(sql, parameters).Select(row => R.FromValues(Sql.GetInteger(R.KeyColumn, row[0]), row[1..])).ToList();

    static bool Exists<R>(IDriver conn, long id) where R : IRecord<R>
    {
        var d = conn.Dialect;
        var sql = $"SELECT {d.Quote(R.KeyColumn)} FROM {d.Quote(R.TableName)} WHERE {d.Quote(R.KeyColumn)} = {d.Mark(1)}";
        return conn.Rows(sql, [id]).Count > 0;
    }

    static int SchemaVersion(IDriver conn)
    {
        conn.Batch(conn.Dialect.VersionTable());
        var rows = conn.Rows("SELECT MAX(version) FROM clarity_schema", []);
        return (int)Sql.GetInteger("version", rows.FirstOrDefault()?[0]);
    }

    public void Dispose()
    {
        foreach (var source in sources)
            source.Driver.Dispose();
    }
}
