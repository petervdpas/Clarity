// The databases of an application: one per data source. Records are saved,
// deleted and read through here, with the entity's data events around a
// save or delete, and migration steps are applied here at startup.

using System.Text.Json.Nodes;

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

    // The settings of the running handler: the window's data had them when the call began,
    // and its code sets them before they are kept; a file store whose folder is a setting
    // reads them first (conformance/settings.json, `now`).
    JsonObject heldSettings = [];

    Db(List<SourceDb> sources) => this.sources = sources;

    /// <summary>One database, used for every record: what Sqlite.Open and Sqlite.Memory give.</summary>
    internal static Db Of(IDriver driver) => new([new SourceDb("", driver)]);

    /// <summary>
    /// A database that keeps nothing: reads find no records and saves are
    /// dropped. What a test, or a screen that has no storage, works on.
    /// </summary>
    public static Db Empty() => Of(new EmptyDriver());

    /// <summary>Whether it was let go (disposed): its connections are closed.</summary>
    public bool Disposed { get; private set; }

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
    /// Adds the data source <paramref name="name"/> on <paramref name="driver"/>, in place of one
    /// of that name: what a file source (Clarity.Formats) does after connecting.
    /// </summary>
    internal void AddSource(string name, IDriver driver)
    {
        foreach (var old in sources.FindAll(s => s.Name == name))
        {
            sources.Remove(old);
            old.Driver.Dispose();
        }
        sources.Add(new SourceDb(name, driver));
    }

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

    /// <summary>
    /// The settings the running handler has, as its window's data had them when the call
    /// began: a file store whose folder is a setting reads them before what is kept.
    /// </summary>
    public void HoldSettings(JsonObject? settings) => heldSettings = settings?.DeepClone().AsObject() ?? [];

    /// <summary>A setting the running handler's code set, before the settings are kept.</summary>
    public void HoldSetting(string setting, string value) => heldSettings[setting] = value;

    string StoreFolder(string name)
    {
        if (storeSettings.TryGetValue(name, out var s))
        {
            var file = KeptSettings.File(s.Program) ?? throw new StorageException("this system names no folder for application settings");
            var settingsFolder = Path.GetDirectoryName(file)!.Replace('\\', '/');
            var folder = KeptSettings.StoreFolderIn(KeptSettings.Os, settingsFolder, name, KeptSettings.Now(heldSettings, KeptSettings.Read(s.Program), s.Setting), s.Fallback);
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
            if (at.Depth == 0)
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
        // A step that holds the transaction ends it itself; a commit that fails leaves nothing half kept.
        if (at.Depth == 0)
        {
            try
            {
                at.Driver.Batch(at.Driver.Dialect.Commit());
            }
            catch
            {
                try
                {
                    at.Driver.Batch(at.Driver.Dialect.Rollback());
                }
                catch (ClarityException)
                {
                }
                throw;
            }
        }
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
        // A database that already has a schema is backed up before it changes (docs/BACKUP-PLAN.md).
        if (done > 0 && done < steps.Count)
            Backup.BeforeMigration(conn, done + 1);
        for (var i = done; i < steps.Count; i++)
            conn.Apply(i + 1, steps[i]);
        return Math.Max(0, steps.Count - done);
    }

    /// <summary>
    /// Why a save or a delete is refused: someone saved the record since it was
    /// read (conformance/versions.json), in Clarity's own words with their English.
    /// </summary>
    public static Text SavedSinceText() =>
        new("Someone else changed this record since you opened it. Reload it to see their change.") { Key = "savedSince" };

    /// <summary>
    /// Inserts a new record (and gives it its id) or updates a stored one, with
    /// the entity's data events around it. A record someone saved since it was
    /// read is refused (<see cref="SavedSinceText"/>), changing nothing.
    /// </summary>
    public void Save<R>(R record) where R : IRecord<R> => SaveWith(record, anyway: false);

    /// <summary>
    /// Saves a record regardless of its version (<c>save ... anyway</c>): what
    /// someone saved since is written over, on purpose.
    /// </summary>
    public void SaveAnyway<R>(R record) where R : IRecord<R> => SaveWith(record, anyway: true);

    /// <summary>The record as it is stored now, with its version: <c>reload</c>.</summary>
    public R Reload<R>(R record) where R : IRecord<R> => Get<R>(record.Id);

    void SaveWith<R>(R record, bool anyway) where R : IRecord<R>
    {
        var (id, version) = (record.Id, record.Version);
        try
        {
            if (!R.HasEvents)
                Write(record, anyway);
            else
                Atomic(R.SourceName, () =>
                {
                    record.BeforeSave(this);
                    Write(record, anyway);
                    record.AfterSave(this);
                });
        }
        catch
        {
            // Nothing was kept, so a new record is still new, at the version it was read with.
            record.Id = id;
            record.Version = version;
            throw;
        }
    }

    /// <summary>The refusal of a save or delete of record <paramref name="id"/> someone saved since, in its entity's words about the record as stored now.</summary>
    RefusedException SavedSince<R>(long id) where R : IRecord<R>
    {
        try
        {
            return new RefusedException(R.SavedSince(this, Get<R>(id)));
        }
        catch (ClarityException)
        {
            return new RefusedException(SavedSinceText());
        }
    }

    void Write<R>(R record, bool anyway) where R : IRecord<R>
    {
        try
        {
            Requirements.Check(R.Required, R.ColumnNames, record.ToValues());
        }
        catch (RefusedException refused)
        {
            refused.Record = record;
            throw;
        }
        Enlist(R.SourceName);
        var secrets = SecretsBefore(R.SourceName, R.TableName, R.KeyColumn, record.Id);
        if (recording is null)
        {
            WriteRow(record, anyway);
            SecretsSaved(R.SourceName, R.TableName, R.KeyColumn, record.Id, secrets);
            return;
        }
        var (before, beforeVersion) = record.Id == 0 ? ((object?[]?)null, (long?)null) : State(R.SourceName, R.TableName, R.KeyColumn, R.ColumnNames, R.VersionColumn, record.Id);
        WriteRow(record, anyway);
        SecretsSaved(R.SourceName, R.TableName, R.KeyColumn, record.Id, secrets);
        var (after, afterVersion) = State(R.SourceName, R.TableName, R.KeyColumn, R.ColumnNames, R.VersionColumn, record.Id);
        recording.Add(new Change(R.SourceName, R.TableName, R.KeyColumn, R.ColumnNames, R.VersionColumn, record.Id, before, after) { BeforeVersion = beforeVersion, AfterVersion = afterVersion });
    }

    void WriteRow<R>(R record, bool anyway) where R : IRecord<R>
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
            // A new record starts at version 1.
            if (R.VersionColumn is { } column)
                kept.Add((column, 1L));
            var insert = d.Insert(R.TableName, R.KeyColumn, kept.Select(p => p.Column).ToList());
            record.Id = Unfit(() => conn.Insert(insert, kept.Select(p => p.Value).ToArray()));
            if (R.VersionColumn is not null)
                record.Version = 1;
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
        // Each save adds one to the version; a checked one writes only over the version it read.
        var version = R.VersionColumn;
        if (version is not null)
            sets.Add($"{d.Quote(version)} = {d.Quote(version)} + 1");
        if (sets.Count == 0)
        {
            if (!Exists<R>(conn, record.Id))
                throw new NotFoundException(R.TableName, record.Id);
            return;
        }
        parameters.Add(record.Id);
        var sql = $"UPDATE {d.Quote(R.TableName)} SET {string.Join(", ", sets)} WHERE {d.Quote(R.KeyColumn)} = {d.Mark(parameters.Count)}";
        var checkedVersion = version is not null && !anyway;
        if (checkedVersion)
        {
            parameters.Add(record.Version);
            sql += $" AND {d.Quote(version!)} = {d.Mark(parameters.Count)}";
        }
        // MySQL counts only the rows whose values changed, so an update that
        // changes nothing is told apart from a missing record (one with a
        // version always changes).
        if (Unfit(() => conn.Execute(sql, parameters)) == 0)
        {
            if (!Exists<R>(conn, record.Id))
                throw new NotFoundException(R.TableName, record.Id);
            if (checkedVersion)
                throw SavedSince<R>(record.Id);
        }
        if (version is not null)
            record.Version = anyway ? State(R.SourceName, R.TableName, R.KeyColumn, [], version, record.Id).Version ?? 0 : record.Version + 1;
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
        Enlist(R.SourceName);
        if (recording is null)
        {
            RemoveRow(record);
            return;
        }
        var (before, beforeVersion) = State(R.SourceName, R.TableName, R.KeyColumn, R.ColumnNames, R.VersionColumn, record.Id);
        RemoveRow(record);
        recording.Add(new Change(R.SourceName, R.TableName, R.KeyColumn, R.ColumnNames, R.VersionColumn, record.Id, before, null) { BeforeVersion = beforeVersion });
    }

    void RemoveRow<R>(R record) where R : IRecord<R>
    {
        var conn = Conn(R.SourceName);
        var d = conn.Dialect;
        var sql = $"DELETE FROM {d.Quote(R.TableName)} WHERE {d.Quote(R.KeyColumn)} = {d.Mark(1)}";
        List<object?> parameters = [record.Id];
        // A record someone saved since it was read is not deleted.
        if (R.VersionColumn is { } version)
        {
            sql += $" AND {d.Quote(version)} = {d.Mark(2)}";
            parameters.Add(record.Version);
        }
        int changed;
        try
        {
            changed = conn.Execute(sql, parameters);
        }
        catch (ReferenceFailure)
        {
            throw new InUseException(R.TableName, record.Id);
        }
        if (changed == 0 && R.VersionColumn is not null && Exists<R>(conn, record.Id))
            throw SavedSince<R>(record.Id);
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

    /// <summary>The key, the stored columns and the version of a record, for SELECT.</summary>
    static string SelectColumns<R>(Dialect d) where R : IRecord<R> =>
        string.Join(", ", R.ColumnNames.Prepend(R.KeyColumn).Select(c =>
            // Whether there is a picture, not the picture.
            R.Pictures.Any(p => p.Column == c && p.Store is null) ? Pictures.Presence(d, c) : d.Quote(c))
            .Concat(R.VersionColumn is { } version ? [d.Quote(version)] : []));

    static List<R> Records<R>(IDriver conn, string sql, IReadOnlyList<object?> parameters) where R : IRecord<R> =>
        conn.Rows(sql, parameters).Select(row =>
        {
            // The version, read last.
            if (R.VersionColumn is null)
                return R.FromValues(Sql.GetInteger(R.KeyColumn, row[0]), row[1..]);
            var record = R.FromValues(Sql.GetInteger(R.KeyColumn, row[0]), row[1..^1]);
            record.Version = Sql.GetInteger("version", row[^1]);
            return record;
        }).ToList();

    /// <summary>The rows SQL reads in the database of <paramref name="source"/>, for tools and tests.</summary>
    public List<object?[]> Rows(string source, string sql, IReadOnlyList<object?> parameters) => [.. Conn(source).Rows(sql, parameters)];

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
        Disposed = true;
        foreach (var source in sources)
            source.Driver.Dispose();
    }
}
