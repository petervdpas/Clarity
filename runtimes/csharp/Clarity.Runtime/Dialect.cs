// The SQL dialects of the databases a data source can be. Statements that
// read and write records are written here, so the rest of the runtime works
// the same on every database. The Rust runtime has the same rules
// (clarity-runtime/src/dialect.rs); conformance/queries.json keeps them equal.

using System.Text;

namespace Clarity.Runtime;

/// <summary>The database a data source is.</summary>
public enum Dialect
{
    Sqlite,
    Postgres,
    Mysql,
    Mssql,
}

public static class DialectSql
{
    /// <summary>The database's name, for messages.</summary>
    public static string Label(this Dialect dialect) => dialect switch
    {
        Dialect.Sqlite => "SQLite",
        Dialect.Postgres => "PostgreSQL",
        Dialect.Mysql => "MySQL",
        _ => "SQL Server",
    };

    /// <summary>A table or column name as SQL needs it, so names like <c>order</c> work.</summary>
    public static string Quote(this Dialect dialect, string name) => dialect switch
    {
        Dialect.Mysql => $"`{name.Replace("`", "``")}`",
        Dialect.Mssql => $"[{name.Replace("]", "]]")}]",
        _ => $"\"{name.Replace("\"", "\"\"")}\"",
    };

    /// <summary>The mark of parameter <paramref name="n"/>, counted from 1.</summary>
    public static string Mark(this Dialect dialect, int n) => dialect switch
    {
        Dialect.Sqlite => $"?{n}",
        Dialect.Postgres => $"${n}",
        Dialect.Mysql => "?",
        _ => $"@P{n}",
    };

    /// <summary>
    /// A decimal value as a comparison reads it. SQLite stores decimals as
    /// text, so it compares them as numbers; the others store numbers.
    /// </summary>
    internal static string Number(this Dialect dialect, string expr) =>
        dialect == Dialect.Sqlite ? $"CAST({expr} AS REAL)" : expr;

    /// <summary>
    /// <paramref name="column"/> contains the LIKE pattern <paramref name="mark"/>,
    /// ignoring case. The pattern escapes its wildcards with <c>!</c>.
    /// </summary>
    internal static string Contains(this Dialect dialect, string column, string mark) => dialect switch
    {
        // SQLite's LIKE ignores case already.
        Dialect.Sqlite => $"{dialect.Quote(column)} LIKE {mark} ESCAPE '!'",
        _ => $"LOWER({dialect.Quote(column)}) LIKE LOWER({mark}) ESCAPE '!'",
    };

    /// <summary>
    /// Whether the JSON list kept in <paramref name="column"/> holds the item
    /// <paramref name="mark"/>, exactly (a list field, U1c), as <see cref="Item"/> gives it.
    /// </summary>
    internal static string Holds(this Dialect dialect, string column, string mark) => dialect switch
    {
        Dialect.Sqlite => $"EXISTS (SELECT 1 FROM json_each({dialect.Quote(column)}) WHERE value = {mark})",
        Dialect.Postgres => $"{dialect.Quote(column)}::jsonb @> {mark}::jsonb",
        Dialect.Mysql => $"JSON_CONTAINS({dialect.Quote(column)}, {mark})",
        _ => $"EXISTS (SELECT 1 FROM OPENJSON({dialect.Quote(column)}) WHERE [value] = {mark})",
    };

    /// <summary>
    /// An item of a list as <see cref="Holds"/> looks for it: SQLite compares the item
    /// itself, PostgreSQL a list of it as JSON, MySQL the item as JSON, and SQL Server
    /// its text. A date or a decimal is its text, as a list keeps it.
    /// </summary>
    internal static object Item(this Dialect dialect, object? item)
    {
        if (item is long n)
            return dialect switch
            {
                Dialect.Sqlite => n,
                Dialect.Postgres => $"[{n}]",
                _ => n.ToString(System.Globalization.CultureInfo.InvariantCulture),
            };
        var text = item switch
        {
            null => "",
            decimal d => d.ToString(System.Globalization.CultureInfo.InvariantCulture),
            DateOnly d => d.ToString(Sql.DateFormat, System.Globalization.CultureInfo.InvariantCulture),
            _ => item.ToString() ?? "",
        };
        var json = ListFields.Kept([text]);
        return dialect switch
        {
            Dialect.Postgres => json,
            Dialect.Mysql => json[1..^1],
            _ => text,
        };
    }

    /// <summary>Text that LIKE matches as it is: wildcards are escaped with <c>!</c>.</summary>
    internal static string EscapeLike(this Dialect dialect, string text)
    {
        var out_ = new StringBuilder();
        foreach (var c in text)
        {
            if (c is '!' or '%' or '_' || (c == '[' && dialect == Dialect.Mssql))
                out_.Append('!');
            out_.Append(c);
        }
        return out_.ToString();
    }

    /// <summary>A bool column that is true.</summary>
    internal static string IsTrue(this Dialect dialect, string column) =>
        dialect == Dialect.Postgres ? $"{dialect.Quote(column)} = TRUE" : $"{dialect.Quote(column)} = 1";

    /// <summary>
    /// INSERT of a new record into <paramref name="table"/>, with a mark per column.
    /// On PostgreSQL and SQL Server it returns the new key; SQLite and MySQL
    /// tell it afterwards.
    /// </summary>
    internal static string Insert(this Dialect dialect, string table, string key, IReadOnlyList<string> columns)
    {
        var t = dialect.Quote(table);
        var k = dialect.Quote(key);
        var names = string.Join(", ", columns.Select(c => dialect.Quote(c)));
        var marks = string.Join(", ", Enumerable.Range(1, columns.Count).Select(n => dialect.Mark(n)));
        return (dialect, columns.Count == 0) switch
        {
            (Dialect.Mysql, true) => $"INSERT INTO {t} () VALUES ()",
            (Dialect.Sqlite, true) => $"INSERT INTO {t} DEFAULT VALUES",
            (Dialect.Postgres, true) => $"INSERT INTO {t} DEFAULT VALUES RETURNING {k}",
            (Dialect.Mssql, true) => $"INSERT INTO {t} OUTPUT INSERTED.{k} DEFAULT VALUES",
            (Dialect.Postgres, false) => $"INSERT INTO {t} ({names}) VALUES ({marks}) RETURNING {k}",
            (Dialect.Mssql, false) => $"INSERT INTO {t} ({names}) OUTPUT INSERTED.{k} VALUES ({marks})",
            _ => $"INSERT INTO {t} ({names}) VALUES ({marks})",
        };
    }

    internal static string Begin(this Dialect dialect) => dialect switch
    {
        Dialect.Mysql => "START TRANSACTION",
        Dialect.Mssql => "BEGIN TRANSACTION",
        _ => "BEGIN",
    };

    internal static string Commit(this Dialect dialect) => dialect == Dialect.Mssql ? "COMMIT TRANSACTION" : "COMMIT";

    internal static string Rollback(this Dialect dialect) => dialect == Dialect.Mssql ? "ROLLBACK TRANSACTION" : "ROLLBACK";

    /// <summary>Creates the table that remembers how many migration steps a database has had.</summary>
    internal static string VersionTable(this Dialect dialect) => dialect == Dialect.Mssql
        ? "IF OBJECT_ID(N'clarity_schema', N'U') IS NULL CREATE TABLE clarity_schema (version INT NOT NULL)"
        : "CREATE TABLE IF NOT EXISTS clarity_schema (version INTEGER NOT NULL)";
}
