// The driver of an empty database (Db.Empty): it keeps nothing. Statements
// change no rows, a query finds none, and each insert gives the next key.

namespace Clarity.Runtime;

sealed class EmptyDriver : IDriver
{
    long lastKey;

    public Dialect Dialect => Dialect.Sqlite;

    public int Execute(string sql, IReadOnlyList<object?> parameters) => 0;

    public long Insert(string sql, IReadOnlyList<object?> parameters) => ++lastKey;

    public List<object?[]> Rows(string sql, IReadOnlyList<object?> parameters) => [];

    public void Batch(string sql) { }

    public void Apply(int version, IReadOnlyList<string> statements) { }

    public void Dispose() { }
}
