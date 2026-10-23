// Queries: which records of an entity to read, and in which order.
// Generated code builds a Query from Clarity's find; this file turns it into
// SQL with parameters, in the dialect of the database the records are in.

namespace Clarity.Runtime;

/// <summary>A comparison between a column and a value.</summary>
public enum Op
{
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
}

/// <summary>
/// A step through a reference: the column that holds it, and the table and
/// key column of the records it refers to.
/// </summary>
public sealed record Hop(string Column, string Table, string Key);

/// <summary>
/// A condition on the columns of one table. Values are what ADO.NET takes:
/// string, long, decimal, bool, DateOnly, DateTime or null.
/// </summary>
public abstract record Cond
{
    Cond() { }

    /// <summary><c>column op value</c>. Numbers stored as text (decimals and money) are compared as numbers.</summary>
    sealed record CompareCond(string Column, Op Op, object? Value, bool Numeric) : Cond;

    /// <summary>The column's text contains the text, ignoring case.</summary>
    sealed record ContainsCond(string Column, string Text) : Cond;

    /// <summary>The JSON list kept in the column holds the item, exactly (a list field).</summary>
    sealed record HoldsCond(string Column, object? Item) : Cond;

    /// <summary>Full-text search: the record holds every word, each as the start of a word, in the searched columns.</summary>
    sealed record MatchesCond(string Table, string Key, IReadOnlyList<string> Columns, IReadOnlyList<string> Typed) : Cond;

    /// <summary>The column equals one of the values.</summary>
    sealed record OneOfCond(string Column, IReadOnlyList<object?> Values, bool Numeric) : Cond;

    /// <summary>A bool column that is true.</summary>
    sealed record IsTrueCond(string Column) : Cond;

    sealed record IsNoneCond(string Column) : Cond;

    /// <summary>The record the reference in the hop's column refers to meets the condition.</summary>
    sealed record ThroughCond(Hop Hop, Cond Inner) : Cond;

    sealed record AndCond(Cond Left, Cond Right) : Cond;

    sealed record OrCond(Cond Left, Cond Right) : Cond;

    sealed record NotCond(Cond Inner) : Cond;

    public static Cond Compare(string column, Op op, object? value) => new CompareCond(column, op, value, false);

    /// <summary>A comparison of a decimal or money column.</summary>
    public static Cond Number(string column, Op op, object? value) => new CompareCond(column, op, value, true);

    public static Cond Contains(string column, string text) => new ContainsCond(column, text);

    /// <summary><c>a.tags contains "live"</c>: whether a list field holds an item.</summary>
    public static Cond Holds(string column, object? item) => new HoldsCond(column, item);

    /// <summary>
    /// Full-text search, <c>a matches x</c>: the record of <paramref name="table"/> holds every
    /// word of <paramref name="text"/>, each as the start of a word, in the columns marked
    /// <c>search</c>, through the table's full-text index.
    /// </summary>
    public static Cond Matches(string table, string key, IReadOnlyList<string> columns, string text) =>
        new MatchesCond(table, key, columns, Words(text));

    /// <summary>
    /// The words of a search: runs of ASCII letters and digits and of any characters
    /// beyond ASCII. Everything else parts them.
    /// </summary>
    public static IReadOnlyList<string> Words(string text)
    {
        var words = new List<string>();
        var word = new System.Text.StringBuilder();
        foreach (var c in text)
        {
            if (c >= 0x80 || char.IsAsciiLetterOrDigit(c))
            {
                word.Append(c);
            }
            else if (word.Length > 0)
            {
                words.Add(word.ToString());
                word.Clear();
            }
        }
        if (word.Length > 0)
            words.Add(word.ToString());
        return words;
    }

    /// <summary>The shortest word MySQL's full-text index keeps (innodb_ft_min_token_size).</summary>
    const int MysqlShortest = 3;

    static string MatchesSql(MatchesCond c, Dialect dialect, Func<object?, bool, string> param)
    {
        if (c.Typed.Count == 0)
            return "1 = 1";
        var quoted = c.Columns.Select(column => dialect.Quote(column)).ToList();
        var parts = new List<string>();
        switch (dialect)
        {
            case Dialect.Sqlite:
                var index = dialect.Quote(c.Table + "__search");
                var query = string.Join(" AND ", c.Typed.Select(w => $"\"{w}\"*"));
                return $"{dialect.Quote(c.Key)} IN (SELECT rowid FROM {index} WHERE {index} MATCH {param(query, false)})";
            case Dialect.Postgres:
                var document = string.Join(" || ' ' || ", quoted.Select(q => $"coalesce({q}, '')"));
                return $"to_tsvector('simple', {document}) @@ to_tsquery('simple', {param(string.Join(" & ", c.Typed.Select(w => w + ":*")), false)})";
            case Dialect.Mysql:
                // The index leaves out words shorter than 3 characters.
                var columns = string.Join(", ", quoted);
                var longer = c.Typed.Where(w => w.EnumerateRunes().Count() >= MysqlShortest).ToList();
                if (longer.Count > 0)
                    parts.Add($"MATCH ({columns}) AGAINST ({param(string.Join(" ", longer.Select(w => $"+{w}*")), false)} IN BOOLEAN MODE)");
                foreach (var w in c.Typed.Where(w => w.EnumerateRunes().Count() < MysqlShortest))
                    parts.Add($"CONCAT_WS(' ', {columns}) REGEXP {param("(^|[^[:alnum:]])" + w, false)}");
                break;
            default:
                var text = $"(N' ' + {string.Join(" + N' ' + ", quoted.Select(q => $"ISNULL({q}, N'')"))})";
                foreach (var w in c.Typed)
                    parts.Add($"{text} LIKE {param($"%[^0-9A-Za-z]{w}%", false)}");
                break;
        }
        return parts.Count > 1 ? $"({string.Join(" AND ", parts)})" : parts[0];
    }

    public static Cond OneOf(string column, IReadOnlyList<object?> values) => new OneOfCond(column, values, false);

    /// <summary><see cref="OneOf"/> for a decimal or money column.</summary>
    public static Cond OneOfNumbers(string column, IReadOnlyList<object?> values) => new OneOfCond(column, values, true);

    public static Cond IsTrue(string column) => new IsTrueCond(column);

    /// <summary>A column with no value: an empty date or reference (<c>none</c>).</summary>
    public static Cond IsNone(string column) => new IsNoneCond(column);

    /// <summary>A condition on the record that <paramref name="column"/> refers to (<c>a.artist.name contains x</c>).</summary>
    public static Cond Through(string column, string table, string key, Cond cond) => new ThroughCond(new Hop(column, table, key), cond);

    public Cond And(Cond other) => new AndCond(this, other);

    public Cond Or(Cond other) => new OrCond(this, other);

    public Cond Negated() => new NotCond(this);

    internal string Sql(Dialect dialect, List<object?> parameters)
    {
        string Param(object? value, bool numeric)
        {
            parameters.Add(value);
            var mark = dialect.Mark(parameters.Count);
            return numeric ? dialect.Number(mark) : mark;
        }
        return this switch
        {
            CompareCond c => $"{Query.ColumnSql(dialect, c.Column, c.Numeric)} {OpSql(c.Op)} {Param(c.Value, c.Numeric)}",
            ContainsCond c => dialect.Contains(c.Column, Param($"%{dialect.EscapeLike(c.Text)}%", false)),
            HoldsCond c => dialect.Holds(c.Column, Param(dialect.Item(c.Item), false)),
            MatchesCond c => MatchesSql(c, dialect, Param),
            OneOfCond c => $"{Query.ColumnSql(dialect, c.Column, c.Numeric)} IN ({string.Join(", ", c.Values.Select(v => Param(v, c.Numeric)))})",
            IsTrueCond c => dialect.IsTrue(c.Column),
            IsNoneCond c => $"{dialect.Quote(c.Column)} IS NULL",
            // The inner condition names the columns of the referred table,
            // which a subquery's own FROM resolves first, even for a table
            // that refers to itself.
            ThroughCond c => $"{dialect.Quote(c.Hop.Column)} IN (SELECT {dialect.Quote(c.Hop.Key)} FROM {dialect.Quote(c.Hop.Table)} WHERE {c.Inner.Sql(dialect, parameters)})",
            AndCond c => $"({c.Left.Sql(dialect, parameters)} AND {c.Right.Sql(dialect, parameters)})",
            OrCond c => $"({c.Left.Sql(dialect, parameters)} OR {c.Right.Sql(dialect, parameters)})",
            NotCond c => $"NOT ({c.Inner.Sql(dialect, parameters)})",
            _ => throw new InvalidOperationException("unknown condition"),
        };
    }

    static string OpSql(Op op) => op switch
    {
        Op.Eq => "=",
        Op.NotEq => "!=",
        Op.Lt => "<",
        Op.LtEq => "<=",
        Op.Gt => ">",
        _ => ">=",
    };
}

/// <summary>
/// The records to read: a condition and an order. Without an order, records
/// come in the order they were stored.
/// </summary>
public sealed record Query
{
    Cond? filter;
    readonly List<(Hop[] Through, string Column, bool Descending, bool Numeric)> order = [];

    public Query Filter(Cond cond)
    {
        filter = cond;
        return this;
    }

    public Query Order(string column, bool descending) => OrderThrough([], column, descending, false);

    /// <summary><see cref="Order"/> for a decimal or money column.</summary>
    public Query OrderNumber(string column, bool descending) => OrderThrough([], column, descending, true);

    /// <summary>
    /// <see cref="Order"/> by a column of the record the references in
    /// <paramref name="through"/> lead to (<c>order by a.artist.name</c>);
    /// <paramref name="numeric"/> for a decimal or money column.
    /// </summary>
    public Query OrderThrough(Hop[] through, string column, bool descending, bool numeric)
    {
        order.Add((through, column, descending, numeric));
        return this;
    }

    /// <summary>
    /// The SELECT statement for <paramref name="table"/> in <paramref name="dialect"/>,
    /// reading <paramref name="columns"/>, and its parameters. Records that sort
    /// the same come in the order of <paramref name="key"/>.
    /// </summary>
    public (string Sql, List<object?> Parameters) Sql(Dialect dialect, string table, string key, string columns)
    {
        var parameters = new List<object?>();
        var sql = $"SELECT {columns} FROM {dialect.Quote(table)}";
        if (filter is not null)
            sql += " WHERE " + filter.Sql(dialect, parameters);
        var keys = order
            .Select(o => OrderSql(dialect, table, o.Through, o.Column, o.Numeric) + (o.Descending ? " DESC" : ""))
            .Append(dialect.Quote(key));
        sql += " ORDER BY " + string.Join(", ", keys);
        return (sql, parameters);
    }

    /// <summary>
    /// An order key. Through references, each step is a subquery on its own
    /// alias (t1, t2, ...) that finds the referred record by the column of the
    /// step before.
    /// </summary>
    static string OrderSql(Dialect dialect, string table, Hop[] through, string column, bool numeric)
    {
        if (through.Length == 0)
            return ColumnSql(dialect, column, numeric);
        string Step(string outer, int at)
        {
            if (at == through.Length)
            {
                var qualified = $"{outer}.{dialect.Quote(column)}";
                return numeric ? dialect.Number(qualified) : qualified;
            }
            var hop = through[at];
            var alias = dialect.Quote($"t{at + 1}");
            return $"(SELECT {Step(alias, at + 1)} FROM {dialect.Quote(hop.Table)} {alias} WHERE {alias}.{dialect.Quote(hop.Key)} = {outer}.{dialect.Quote(hop.Column)})";
        }
        return Step(dialect.Quote(table), 0);
    }

    /// <summary>A column as a comparison reads it.</summary>
    internal static string ColumnSql(Dialect dialect, string column, bool numeric) =>
        numeric ? dialect.Number(dialect.Quote(column)) : dialect.Quote(column);
}

public static class Texts
{
    /// <summary>Text contains a part, ignoring case: Clarity's <c>contains</c> outside a query.</summary>
    public static bool Contains(string text, string part) =>
        text.Contains(part, StringComparison.CurrentCultureIgnoreCase);
}
