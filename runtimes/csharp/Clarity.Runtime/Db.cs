// The databases of an application: one per data source. Records are saved,
// deleted and read through here, with the entity's data events around a
// save or delete, and migration steps are applied here at startup.

namespace Clarity.Runtime;

public sealed partial class Db : IDisposable
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

    /// <summary>The folders of the file stores, by name.</summary>
    readonly Dictionary<string, string> stores = [];

    /// <summary>The file stores whose folder is a folder setting, by name.</summary>
    readonly Dictionary<string, (string Program, string Setting, string Fallback)> storeSettings = [];

    Db(List<SourceDb> sources) => this.sources = sources;

    /// <summary>One database, used for every record: what Sqlite.Open and Sqlite.Memory give.</summary>
    internal static Db Of(IDriver driver) => new([new SourceDb("", driver)]);

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
    /// Adds the file store <paramref name="name"/> at <paramref name="location"/>, a folder
    /// (made when it is not there yet); <c>env:NAME</c> takes it from that environment variable.
    /// </summary>
    public void AddStore(string name, string location)
    {
        var folder = Store.Location(location);
        try
        {
            Directory.CreateDirectory(folder);
        }
        catch (Exception e) when (e is IOException or UnauthorizedAccessException)
        {
            throw new StorageException($"cannot make the file store {folder}: {e.Message}", e);
        }
        storeSettings.Remove(name);
        stores[name] = folder;
    }

    /// <summary>
    /// Adds the file store <paramref name="name"/> whose folder is the folder setting
    /// <paramref name="setting"/> of <paramref name="program"/>, or <paramref name="fallback"/>
    /// (the setting's default) while it is not kept; read again at every use, so a folder
    /// chosen while the application runs takes effect at once.
    /// </summary>
    public void AddStoreSetting(string name, string program, string setting, string fallback)
    {
        stores.Remove(name);
        storeSettings[name] = (program, setting, fallback);
        StoreFolder(name);
    }

    string StoreFolder(string name)
    {
        if (storeSettings.TryGetValue(name, out var s))
        {
            var file = KeptSettings.File(s.Program) ?? throw new StorageException("this system names no folder for application settings");
            var settingsFolder = Path.GetDirectoryName(file)!.Replace('\\', '/');
            var folder = KeptSettings.StoreFolderIn(KeptSettings.Os, settingsFolder, name, KeptSettings.Read(s.Program)[s.Setting], s.Fallback);
            try
            {
                Directory.CreateDirectory(folder);
            }
            catch (Exception e) when (e is IOException or UnauthorizedAccessException)
            {
                throw new StorageException($"cannot make the file store {folder}: {e.Message}", e);
            }
            return folder;
        }
        return stores.TryGetValue(name, out var found) ? found : throw new StorageException($"there is no file store '{name}'");
    }

    /// <summary>
    /// The values of a record as they are saved: a new picture checked, and
    /// written to its file store when it has one, its column then holding the
    /// file's name.
    /// </summary>
    object?[] SavedValues<R>(R record) where R : IRecord<R>
    {
        var values = record.ToValues();
        foreach (var picture in R.Pictures)
        {
            var at = R.ColumnNames.ToList().IndexOf(picture.Column);
            if (at < 0 || values[at] is not byte[] bytes)
                continue;
            // A file of a file field is kept under its own name, in a folder named by its contents.
            if (picture.Name is { } nameColumn)
            {
                if (picture.Store is not { } fileStore)
                    continue;
                var named = R.ColumnNames.ToList().IndexOf(nameColumn) is var n and >= 0 ? values[n] as string : null;
                var path = RecordFiles.StorePath(bytes, named);
                var kept = Pictures.InStore(StoreFolder(fileStore), path);
                try
                {
                    if (!File.Exists(kept))
                    {
                        Directory.CreateDirectory(Path.GetDirectoryName(kept)!);
                        File.WriteAllBytes(kept, bytes);
                    }
                }
                catch (Exception e) when (e is IOException or UnauthorizedAccessException)
                {
                    throw new StorageException($"cannot write {kept}: {e.Message}", e);
                }
                values[at] = path;
                continue;
            }
            var name = Pictures.FileName(bytes);
            if (picture.Store is not { } store)
                continue;
            var file = Path.Combine(StoreFolder(store), name);
            try
            {
                if (!File.Exists(file))
                    File.WriteAllBytes(file, bytes);
            }
            catch (Exception e) when (e is IOException or UnauthorizedAccessException)
            {
                throw new StorageException($"cannot write {file}: {e.Message}", e);
            }
            values[at] = name;
        }
        return values;
    }

    /// <summary>
    /// The picture of field <paramref name="column"/> of record <paramref name="id"/>, read
    /// from the database or from its file store; null when the record has none.
    /// </summary>
    public byte[]? PictureOf<R>(long id, string column) where R : IRecord<R>
    {
        var picture = R.Pictures.FirstOrDefault(p => p.Column == column)
            ?? throw new StorageException($"{R.TableName} has no picture '{column}'");
        var conn = Conn(R.SourceName);
        var d = conn.Dialect;
        var rows = conn.Rows($"SELECT {d.Quote(column)} FROM {d.Quote(R.TableName)} WHERE {d.Quote(R.KeyColumn)} = {d.Mark(1)}", [id]);
        if (rows.Count == 0)
            throw new NotFoundException(R.TableName, id);
        switch (picture.Store, rows[0][0])
        {
            case (_, null):
                return null;
            case (null, byte[] bytes):
                return bytes;
            case ({ } store, string path) when path.Length > 0:
                var file = Pictures.InStore(StoreFolder(store), path);
                try
                {
                    return File.ReadAllBytes(file);
                }
                catch (Exception e) when (e is IOException or UnauthorizedAccessException)
                {
                    throw new StorageException($"cannot read {file}: {e.Message}", e);
                }
            case (_, string):
                return null;
            case (_, var other):
                throw new StorageException($"{column}: {other} is no picture");
        }
    }

    /// <summary>
    /// Another record's picture for a field kept in <paramref name="to"/> (null: the
    /// database), as code copies it (album.cover = other.cover). <paramref name="value"/> is
    /// field <paramref name="column"/> of record <paramref name="id"/>, kept in
    /// <paramref name="from"/>. A picture in hand is copied as it is, a stored one is read
    /// first, and a file of a store stays linked only in that same store.
    /// </summary>
    public Picture? CopiedPicture<R>(Picture? value, long id, string column, string? from, string? to) where R : IRecord<R>
    {
        if (value is null)
            return null;
        if (value.Bytes is { } bytes)
            return Picture.New(bytes);
        if (value.FilePath is { } path)
        {
            if (from is null)
                throw new StorageException($"{column} is kept in the database, not in a file store");
            return from == to ? Picture.InStore(path) : Picture.New(StoreFile(from, path));
        }
        return PictureOf<R>(id, column) is { } read ? Picture.New(read) : null;
    }

    /// <summary>Another record's file, with its name, as <see cref="CopiedPicture{R}"/> copies a picture.</summary>
    public RecordFile? CopiedFile<R>(RecordFile? value, long id, string column, string? from, string? to) where R : IRecord<R> =>
        value is null ? null : CopiedPicture<R>(value.Content, id, column, from, to) is { } content ? new RecordFile(value.Name, content) : null;

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
        if (recording is null)
        {
            WriteRow(record);
            return;
        }
        var before = record.Id == 0 ? null : Row(R.SourceName, R.TableName, R.KeyColumn, R.ColumnNames, record.Id);
        WriteRow(record);
        recording.Add(new Change(R.SourceName, R.TableName, R.KeyColumn, R.ColumnNames, record.Id, before, Row(R.SourceName, R.TableName, R.KeyColumn, R.ColumnNames, record.Id)));
    }

    void WriteRow<R>(R record) where R : IRecord<R>
    {
        var values = SavedValues(record);
        var conn = Conn(R.SourceName);
        var d = conn.Dialect;
        bool IsPicture(string c) => R.Pictures.Any(p => p.Column == c);
        if (record.Id == 0)
        {
            // An empty or kept picture is left to the column's NULL, so no NULL
            // is bound to a binary column (SQL Server refuses that).
            var kept = R.ColumnNames.Select((c, i) => (Column: c, Value: values[i]))
                .Where(p => !(IsPicture(p.Column) && (p.Value is null || ReferenceEquals(p.Value, Sql.Kept))))
                .ToList();
            var insert = d.Insert(R.TableName, R.KeyColumn, kept.Select(p => p.Column).ToList());
            record.Id = Unfit(() => conn.Insert(insert, kept.Select(p => p.Value).ToArray()));
            return;
        }
        // A kept picture stays as it is; an emptied one is set to NULL as text.
        var sets = new List<string>();
        var parameters = new List<object?>();
        for (var i = 0; i < values.Length; i++)
        {
            var column = R.ColumnNames[i];
            if (ReferenceEquals(values[i], Sql.Kept))
                continue;
            if (values[i] is null && IsPicture(column))
            {
                sets.Add($"{d.Quote(column)} = NULL");
                continue;
            }
            parameters.Add(values[i]);
            sets.Add($"{d.Quote(column)} = {d.Mark(parameters.Count)}");
        }
        if (sets.Count == 0)
        {
            if (!Exists<R>(conn, record.Id))
                throw new NotFoundException(R.TableName, record.Id);
            return;
        }
        var sql = $"UPDATE {d.Quote(R.TableName)} SET {string.Join(", ", sets)} WHERE {d.Quote(R.KeyColumn)} = {d.Mark(parameters.Count + 1)}";
        parameters.Add(record.Id);
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
        if (!R.HasEvents && !R.HasCascades)
        {
            Remove(record);
            return;
        }
        Atomic(R.SourceName, () =>
        {
            record.BeforeDelete(this);
            // What cascades from it goes first, so no reference is left behind.
            record.Cascade(this);
            Remove(record);
            record.AfterDelete(this);
        });
    }

    void Remove<R>(R record) where R : IRecord<R>
    {
        if (recording is null)
        {
            RemoveRow(record);
            return;
        }
        var before = Row(R.SourceName, R.TableName, R.KeyColumn, R.ColumnNames, record.Id);
        RemoveRow(record);
        recording.Add(new Change(R.SourceName, R.TableName, R.KeyColumn, R.ColumnNames, record.Id, before, null));
    }

    void RemoveRow<R>(R record) where R : IRecord<R>
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
        string.Join(", ", R.ColumnNames.Prepend(R.KeyColumn).Select(c =>
            // Whether there is a picture, not the picture.
            R.Pictures.Any(p => p.Column == c && p.Store is null) ? Pictures.Presence(d, c) : d.Quote(c)));

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
