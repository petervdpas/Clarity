// A data source that is a file in a format (docs/FORMATS-PLAN.md, "A file
// as a data source"): `datasource logins csv "imports/logins.csv" external`.
// The file is read through Clarity.Formats into one table of an in-memory
// SQLite database (one per entity when its mapping holds records inside
// records, each linked to the row it sits in by that row's key), so the generated code finds, saves and deletes as it does
// in any SQLite database. After each change that is kept (outside a
// transaction, or when it commits) the table is written back to a new file
// that then replaces the old one, so a crash never leaves half a file; the
// file's own column order and the columns no entity maps are kept.

using System.Globalization;
using System.Text;
using System.Text.Json;
using Clarity.Runtime;

namespace Clarity.Formats;

/// <summary>
/// What the generator says of a file source: its format and options (a JSON object, empty for none),
/// the entity's table and key column, and the columns the entity maps with their kinds
/// (JSON <c>[{"name":"site","kind":"text"},...]</c>). A readonly source never writes its file.
/// </summary>
/// <param name="Name">The data source's name.</param>
/// <param name="Format">The format, as the catalog names it (csv, json).</param>
/// <param name="Path">Where the file is, written as a SQLite file's location is.</param>
/// <param name="Options">The format's options, a JSON object; empty for none.</param>
/// <param name="Table">The entity's table.</param>
/// <param name="Key">The entity's key column.</param>
/// <param name="Fields">The columns the entity maps with their kinds, as JSON.</param>
/// <param name="Readonly">Whether the file is never written.</param>
public sealed record FileSource(string Name, string Format, string Path, string Options, string Table, string Key, string Fields, bool Readonly);

/// <summary>File sources on a program's databases.</summary>
public static class FileSources
{
    /// <summary>
    /// Reads the file of <paramref name="source"/> into its table and adds it as the data source of its
    /// name. The file is found as a SQLite file's location is; the program turns SQLite on first (Sqlite.Use()).
    /// </summary>
    public static void AddFile(this Db db, FileSource source)
    {
        var at = Store.Location(DataFolder.Sqlite(source.Path));
        db.AddSource(source.Name, FileDriver.Open(source, at));
    }
}

/// <summary>One table read from the file and kept in SQLite.</summary>
/// <param name="Name">Its name in the file's tables: empty for the source's own, <c>lines</c> or <c>lines/notes</c> for records inside.</param>
/// <param name="Table">Its SQLite table.</param>
/// <param name="Key">Its key column.</param>
/// <param name="Columns">The file's columns, in its order; for records inside, the second links a row to its row.</param>
sealed record FileTable(string Name, string Table, string Key, List<string> Columns)
{
    /// <summary>Whether the second column is a link: the place of the row in the table it sits in.</summary>
    public bool Linked => Name.Length > 0;

    /// <summary>The name of the table its rows sit in: <c>lines/notes</c> in <c>lines</c>, <c>lines</c> in the source's own.</summary>
    public string Parent => Name.LastIndexOf('/') is var at and >= 0 ? Name[..at] : "";
}

internal sealed class FileDriver : IDriver
{
    readonly IDriver db;
    readonly string path;
    readonly FileSource source;

    /// <summary>The tables the file is read into: one, or with records inside (a mapping's <c>nested</c>) one per entity.</summary>
    readonly List<FileTable> tables;

    /// <summary>Whether the file holds records inside its records, so its tables go back as several.</summary>
    readonly bool several;

    /// <summary>The file as it was last read or written, for formats that keep more than the table.</summary>
    byte[] original;

    bool dirty;

    /// <summary>The file's size and modification time when it was last read or written: a change by another program refuses the next save.</summary>
    (long Size, DateTime Modified)? seen;

    FileDriver(IDriver db, string path, FileSource source, List<FileTable> tables, bool several, byte[] original)
    {
        this.db = db;
        this.path = path;
        this.source = source;
        this.tables = tables;
        this.several = several;
        this.original = original;
        seen = Stamp(path);
    }

    /// <summary>A file's size and modification time, or null when it is not there.</summary>
    static (long Size, DateTime Modified)? Stamp(string path)
    {
        var info = new FileInfo(path);
        return info.Exists ? (info.Length, info.LastWriteTimeUtc) : null;
    }

    /// <summary>Refuses a change when another program changed the file since it was read or written: its size or modification time differ, and so do its bytes.</summary>
    void UnchangedOnDisk()
    {
        if (source.Readonly)
            return;
        var now = Stamp(path);
        if (now == seen)
            return;
        byte[] bytes;
        try
        {
            bytes = now is null ? [] : File.ReadAllBytes(path);
        }
        catch (Exception e) when (e is IOException or UnauthorizedAccessException)
        {
            throw new RefusedException(Db.SavedSinceText());
        }
        if (!bytes.AsSpan().SequenceEqual(original))
            throw new RefusedException(Db.SavedSinceText());
        // The same bytes written again: no change.
        seen = now;
    }

    public Dialect Dialect => Dialect.Sqlite;

    static string Quote(string name) => "\"" + name.Replace("\"", "\"\"") + "\"";

    /// <summary>The SQLite type a mapped column of <paramref name="kind"/> gets, so its values are kept as the generated code reads them.</summary>
    static string ColumnType(string? kind) => kind switch
    {
        "integer" or "bool" => " INTEGER",
        "text" or "decimal" or "date" or "datetime" or "enum" or "values" => " TEXT",
        _ => "",
    };

    /// <summary>A name as names are matched: without <c>_ - </c> and spaces, in lower case.</summary>
    static string Plain(string name) => string.Concat(name.Where(c => c is not ('_' or '-' or ' '))).ToLowerInvariant();

    static JsonDocument? Parsed(string text)
    {
        try
        {
            return JsonDocument.Parse(text);
        }
        catch (JsonException)
        {
            return null;
        }
    }

    /// <summary>The kinds of the mapped columns, by name in their order.</summary>
    static List<(string Name, string? Kind)> Kinds(JsonElement fields)
    {
        if (fields.ValueKind != JsonValueKind.Array)
            return [];
        return [.. fields.EnumerateArray()
            .Where(f => f.ValueKind == JsonValueKind.Object && f.TryGetProperty("name", out var n) && n.ValueKind == JsonValueKind.String)
            .Select(f => (f.GetProperty("name").GetString()!, f.TryGetProperty("kind", out var k) && k.ValueKind == JsonValueKind.String ? k.GetString() : null))];
    }

    /// <summary>The records field the table name <paramref name="name"/> (<c>lines/notes</c>) leads to among <paramref name="fields"/>.</summary>
    static JsonElement? RecordsField(JsonElement fields, string name)
    {
        JsonElement? found = null;
        foreach (var step in name.Split('/'))
        {
            if (fields.ValueKind != JsonValueKind.Array)
                return null;
            found = fields.EnumerateArray().Cast<JsonElement?>().FirstOrDefault(f =>
                f!.Value.ValueKind == JsonValueKind.Object && f.Value.TryGetProperty("name", out var n) && n.ValueKind == JsonValueKind.String && n.GetString() == step);
            if (found is not { } field || !field.TryGetProperty("fields", out fields))
                return null;
        }
        return found;
    }

    static string? Text(JsonElement element, string name) =>
        element.TryGetProperty(name, out var value) && value.ValueKind == JsonValueKind.String ? value.GetString() : null;

    /// <summary>A cell read, as SQLite keeps it.</summary>
    static object? Cell(JsonElement value) => value.ValueKind switch
    {
        JsonValueKind.Null => null,
        JsonValueKind.True => 1L,
        JsonValueKind.False => 0L,
        JsonValueKind.Number => long.TryParse(value.GetRawText(), NumberStyles.AllowLeadingSign, CultureInfo.InvariantCulture, out var n) ? n : value.GetRawText(),
        JsonValueKind.String => value.GetString(),
        _ => value.GetRawText(),
    };

    /// <summary>A value SQLite gives, as a cell of the table written back.</summary>
    static void WriteCell(Utf8JsonWriter w, object? value)
    {
        switch (value)
        {
            case long n:
                w.WriteNumberValue(n);
                break;
            case double d when double.IsFinite(d):
                w.WriteNumberValue(d);
                break;
            case string t:
                w.WriteStringValue(t);
                break;
            case bool b:
                w.WriteNumberValue(b ? 1 : 0);
                break;
            case decimal d:
                w.WriteStringValue(d.ToString(CultureInfo.InvariantCulture));
                break;
            case DateOnly d:
                w.WriteStringValue(d.ToString("yyyy-MM-dd", CultureInfo.InvariantCulture));
                break;
            case DateTime d:
                w.WriteStringValue(d.ToString("yyyy-MM-ddTHH:mm:ss", CultureInfo.InvariantCulture));
                break;
            default:
                w.WriteNullValue();
                break;
        }
    }

    /// <summary>Reads the file at <paramref name="path"/> (already found as a SQLite file's location is) into its table.</summary>
    internal static FileDriver Open(FileSource source, string path)
    {
        StorageException Fail(string what, Exception? inner = null) => new($"data source {source.Name}: {what}", inner);
        byte[] original;
        try
        {
            // A file not there yet is a new one, made at the first save; a readonly source needs it.
            original = !source.Readonly && !File.Exists(path) ? [] : File.ReadAllBytes(path);
        }
        catch (Exception e) when (e is IOException or UnauthorizedAccessException)
        {
            throw Fail($"cannot read {path}: {e.Message}", e);
        }
        string read;
        try
        {
            read = FormatFunctions.ReadTable(source.Format, original, source.Options, source.Fields);
        }
        catch (ValueException e)
        {
            throw Fail(e.Message, e);
        }
        using var parsed = Parsed(read) ?? throw Fail("the table read is no table");
        using var fieldsDoc = Parsed(string.IsNullOrWhiteSpace(source.Fields) ? "[]" : source.Fields);
        var fields = fieldsDoc?.RootElement ?? default;
        var root = parsed.RootElement;
        var several = root.ValueKind == JsonValueKind.Object && root.TryGetProperty("tables", out var list) && list.ValueKind == JsonValueKind.Array;
        List<JsonElement> given = several ? [.. root.GetProperty("tables").EnumerateArray()] : [root];

        var db = Store.Open(Dialect.Sqlite, ":memory:");
        var tables = new List<FileTable>();
        // The keys of each table's rows, in the order read, so a link to a place becomes a key.
        var keys = new Dictionary<string, List<long>>();
        try
        {
            foreach (var table in given)
            {
                if (table.ValueKind != JsonValueKind.Object
                    || !table.TryGetProperty("columns", out var named) || named.ValueKind != JsonValueKind.Array
                    || named.EnumerateArray().Any(c => c.ValueKind != JsonValueKind.String)
                    || !table.TryGetProperty("rows", out var rows) || rows.ValueKind != JsonValueKind.Array
                    || rows.EnumerateArray().Any(r => r.ValueKind != JsonValueKind.Array))
                    throw Fail("the table read is no table");
                var columns = named.EnumerateArray().Select(c => c.GetString()!).ToList();
                var name = Text(table, "name") ?? "";
                var kept = new FileTable(name, source.Table, source.Key, columns);
                var kindsAt = fields;
                if (kept.Linked)
                {
                    var field = RecordsField(fields, name);
                    var sqlTable = field is { } f ? Text(f, "table") : null;
                    var key = field is { } g ? Text(g, "key") : null;
                    if (sqlTable is null || key is null)
                        throw Fail($"the records inside '{name}' are no field of the fields given");
                    kept = kept with { Table = sqlTable, Key = key };
                    kindsAt = field!.Value.GetProperty("fields");
                }
                var kinds = Kinds(kindsAt);
                string KindOf(string column) => ColumnType(kinds.Find(f => Plain(f.Name) == Plain(column)).Kind);
                var made = new List<string>();
                if (!columns.Contains(kept.Key))
                    // The file has no key column: the row's place keys it, never written back.
                    made.Add($"{Quote(kept.Key)} INTEGER PRIMARY KEY");
                for (var c = 0; c < columns.Count; c++)
                    made.Add(Quote(columns[c]) + (columns[c] == kept.Key ? " INTEGER PRIMARY KEY" : kept.Linked && c == 1 ? " INTEGER" : KindOf(columns[c])));
                db.Batch($"CREATE TABLE {Quote(kept.Table)} ({string.Join(", ", made)})");
                var names = string.Join(", ", columns.Select(Quote));
                var marks = string.Join(", ", Enumerable.Range(1, columns.Count).Select(n => Dialect.Sqlite.Mark(n)));
                var insert = $"INSERT INTO {Quote(kept.Table)} ({names}) VALUES ({marks})";
                List<long> parent = kept.Linked && keys.TryGetValue(kept.Parent, out var p) ? p : [];
                var inserted = new List<long>();
                var i = 0;
                foreach (var row in rows.EnumerateArray())
                {
                    i++;
                    var values = row.EnumerateArray().Select(Cell).ToArray();
                    if (kept.Linked && values.Length > 1)
                        // The place of the row it sits in, from 1, is that row's key.
                        values[1] = values[1] is long place && place >= 1 && place <= parent.Count ? parent[(int)place - 1] : null;
                    try
                    {
                        inserted.Add(db.Insert(insert, values));
                    }
                    catch (StorageException e)
                    {
                        throw Fail($"row {i}: {e.What}", e);
                    }
                }
                keys[name] = inserted;
                tables.Add(kept);
            }
        }
        catch
        {
            db.Dispose();
            throw;
        }
        return new FileDriver(db, path, source, tables, several, original);
    }

    /// <summary>Writes the table back when a change is kept: to a new file beside the old one, which then replaces it.</summary>
    void Keep()
    {
        if (!dirty || source.Readonly || (db is ITransactional t && t.InTransaction))
            return;
        dirty = false;
        // Each table's rows in their order, a link turned back into the place of its row among the rows of the table it sits in.
        var places = new Dictionary<string, Dictionary<long, long>>();
        using var json = new MemoryStream();
        using (var w = new Utf8JsonWriter(json))
        {
            if (several)
            {
                w.WriteStartObject();
                w.WriteStartArray("tables");
            }
            foreach (var table in tables)
            {
                Dictionary<long, long> parent = table.Linked && places.TryGetValue(table.Parent, out var p) ? p : [];
                var own = new Dictionary<long, long>();
                var names = string.Concat(table.Columns.Select(c => ", " + Quote(c)));
                var rows = db.Rows($"SELECT {Quote(table.Key)}{names} FROM {Quote(table.Table)} ORDER BY rowid", []);
                w.WriteStartObject();
                if (several)
                    w.WriteString("name", table.Name);
                w.WriteStartArray("columns");
                foreach (var column in table.Columns)
                    w.WriteStringValue(column);
                w.WriteEndArray();
                w.WriteStartArray("rows");
                foreach (var row in rows)
                {
                    if (row[0] is long key)
                        own[key] = own.Count + 1;
                    w.WriteStartArray();
                    for (var c = 1; c < row.Length; c++)
                    {
                        if (table.Linked && c == 2)
                            WriteCell(w, row[c] is long link && parent.TryGetValue(link, out var place) ? place : null);
                        else
                            WriteCell(w, row[c]);
                    }
                    w.WriteEndArray();
                }
                w.WriteEndArray();
                w.WriteEndObject();
                places[table.Name] = own;
            }
            if (several)
            {
                w.WriteEndArray();
                w.WriteEndObject();
            }
        }
        byte[] bytes;
        try
        {
            bytes = FormatFunctions.WriteTable(source.Format, original, Encoding.UTF8.GetString(json.ToArray()), source.Options, source.Fields);
        }
        catch (ValueException e)
        {
            throw new StorageException($"data source {source.Name}: {e.Message}");
        }
        var folder = System.IO.Path.GetDirectoryName(path);
        if (!string.IsNullOrEmpty(folder)) Directory.CreateDirectory(folder);
        var fresh = path + ".new";
        try
        {
            File.WriteAllBytes(fresh, bytes);
        }
        catch (Exception e) when (e is IOException or UnauthorizedAccessException)
        {
            throw new StorageException($"cannot write {fresh}: {e.Message}", e);
        }
        try
        {
            File.Move(fresh, path, overwrite: true);
        }
        catch (Exception e) when (e is IOException or UnauthorizedAccessException)
        {
            throw new StorageException($"cannot replace {path}: {e.Message}", e);
        }
        original = bytes;
        seen = Stamp(path);
    }

    public int Execute(string sql, IReadOnlyList<object?> parameters)
    {
        UnchangedOnDisk();
        var changed = db.Execute(sql, parameters);
        dirty |= changed > 0;
        Keep();
        return changed;
    }

    public long Insert(string sql, IReadOnlyList<object?> parameters)
    {
        UnchangedOnDisk();
        var id = db.Insert(sql, parameters);
        dirty = true;
        Keep();
        return id;
    }

    public List<object?[]> Rows(string sql, IReadOnlyList<object?> parameters) => db.Rows(sql, parameters);

    /// <summary>A transaction's end keeps what it changed; a file another program changed meanwhile rolls it back instead.</summary>
    public void Batch(string sql)
    {
        if (dirty && sql == Dialect.Sqlite.Commit())
        {
            try
            {
                UnchangedOnDisk();
            }
            catch (RefusedException)
            {
                db.Batch(Dialect.Sqlite.Rollback());
                dirty = false;
                throw;
            }
        }
        db.Batch(sql);
        Keep();
    }

    /// <summary>A file source is external: Clarity writes no steps for it.</summary>
    public void Apply(int version, IReadOnlyList<string> statements) { }

    public void Dispose() => db.Dispose();
}
