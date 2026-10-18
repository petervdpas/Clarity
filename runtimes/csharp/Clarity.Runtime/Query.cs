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

    /// <summary>The column equals one of the values.</summary>
    sealed record OneOfCond(string Column, IReadOnlyList<object?> Values, bool Numeric) : Cond;

    /// <summary>A bool column that is true.</summary>
    sealed record IsTrueCond(string Column) : Cond;

    sealed record IsNoneCond(string Column) : Cond;

    sealed record AndCond(Cond Left, Cond Right) : Cond;

    sealed record OrCond(Cond Left, Cond Right) : Cond;

    sealed record NotCond(Cond Inner) : Cond;

    public static Cond Compare(string column, Op op, object? value) => new CompareCond(column, op, value, false);

    /// <summary>A comparison of a decimal or money column.</summary>
    public static Cond Number(string column, Op op, object? value) => new CompareCond(column, op, value, true);

    public static Cond Contains(string column, string text) => new ContainsCond(column, text);

    public static Cond OneOf(string column, IReadOnlyList<object?> values) => new OneOfCond(column, values, false);

    /// <summary><see cref="OneOf"/> for a decimal or money column.</summary>
    public static Cond OneOfNumbers(string column, IReadOnlyList<object?> values) => new OneOfCond(column, values, true);

    public static Cond IsTrue(string column) => new IsTrueCond(column);

    /// <summary>A column with no value: an empty date or reference (<c>none</c>).</summary>
    public static Cond IsNone(string column) => new IsNoneCond(column);

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
            OneOfCond c => $"{Query.ColumnSql(dialect, c.Column, c.Numeric)} IN ({string.Join(", ", c.Values.Select(v => Param(v, c.Numeric)))})",
            IsTrueCond c => dialect.IsTrue(c.Column),
            IsNoneCond c => $"{dialect.Quote(c.Column)} IS NULL",
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
    readonly List<(string Column, bool Descending, bool Numeric)> order = [];

    public Query Filter(Cond cond)
    {
        filter = cond;
        return this;
    }

    public Query Order(string column, bool descending)
    {
        order.Add((column, descending, false));
        return this;
    }

    /// <summary><see cref="Order"/> for a decimal or money column.</summary>
    public Query OrderNumber(string column, bool descending)
    {
        order.Add((column, descending, true));
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
            .Select(o => ColumnSql(dialect, o.Column, o.Numeric) + (o.Descending ? " DESC" : ""))
            .Append(dialect.Quote(key));
        sql += " ORDER BY " + string.Join(", ", keys);
        return (sql, parameters);
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
