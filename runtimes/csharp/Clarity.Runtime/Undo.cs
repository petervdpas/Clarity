// The database side of undo (DESIGN 5.29): the changes of each handler run
// that changed something, by the step's id. The window keeps the edits on
// its bus, each with the id of the step its handler made, and asks for that
// step to be undone or done again. Kept until the application closes.

namespace Clarity.Runtime;

public sealed partial class Db
{
    /// <summary>A row of a table as it was before a change and as the change left it; null where it was not there.</summary>
    sealed record Change(string Source, string Table, string Key, IReadOnlyList<string> Columns, long Id, object?[]? Before, object?[]? After);

    sealed class UndoStep(List<Change> changes)
    {
        public List<Change> Changes { get; } = changes;
        public bool Undone { get; set; }
    }

    /// <summary>The changes of the step that runs; null outside a step.</summary>
    List<Change>? recording;

    readonly Dictionary<long, UndoStep> steps = [];

    /// <summary>The id of the last step a handler made, to undo it later by; 0 before any.</summary>
    public long LastStep { get; private set; }

    /// <summary>
    /// Runs a handler as one step, which undo takes back whole: its id, when it
    /// changed anything, is <see cref="LastStep"/>. When it fails, what it changed is
    /// put back at once, so it leaves nothing behind. A step inside a step is part of it.
    /// </summary>
    public void Step(Action run) => Step<object?>(() =>
    {
        run();
        return null;
    });

    /// <inheritdoc cref="Step(Action)"/>
    public T Step<T>(Func<T> run)
    {
        if (recording is not null)
            return run();
        var changes = new List<Change>();
        recording = changes;
        T result;
        try
        {
            result = run();
        }
        catch
        {
            recording = null;
            for (var i = changes.Count - 1; i >= 0; i--)
            {
                try
                {
                    Put(changes[i], changes[i].Before);
                }
                catch (ClarityException)
                {
                }
            }
            throw;
        }
        recording = null;
        if (changes.Count > 0)
            steps[++LastStep] = new UndoStep(changes);
        return result;
    }

    /// <summary>
    /// Takes back step <paramref name="id"/>: every row it changed as it was, with its own
    /// id. Refused, changing nothing, when one of its rows changed since; false when
    /// there is no such step, or it is undone already.
    /// </summary>
    public bool Undo(long id) => ReplayStep(id, back: true);

    /// <summary>Does step <paramref name="id"/> again after an undo; false when it is not undone.</summary>
    public bool Redo(long id) => ReplayStep(id, back: false);

    bool ReplayStep(long id, bool back)
    {
        if (!steps.TryGetValue(id, out var step) || step.Undone == back)
            return false;
        Replay(step.Changes, back);
        step.Undone = back;
        return true;
    }

    /// <summary>Puts a step's rows back or forward, after checking that each row is still as the step left it, in one transaction per source.</summary>
    void Replay(List<Change> changes, bool back)
    {
        var order = back ? Enumerable.Reverse(changes).ToList() : changes;
        // Each row as the step left it (its last change undone, its first redone) must still be so.
        var seen = new HashSet<(string, string, long)>();
        foreach (var change in order)
        {
            if (!seen.Add((change.Source, change.Table, change.Id)))
                continue;
            var expected = back ? change.After : change.Before;
            if (!SameRow(Row(change.Source, change.Table, change.Key, change.Columns, change.Id), expected))
                throw new RefusedException(new Text("This was changed since, so it cannot be taken back") { Key = "undoChangedSince" });
        }
        Within([.. order.Select(c => c.Source).Distinct()], () =>
        {
            foreach (var change in order)
                Put(change, back ? change.Before : change.After);
        });
    }

    static bool SameRow(object?[]? a, object?[]? b) =>
        a is null || b is null ? a is null && b is null : a.Length == b.Length && a.Zip(b).All(p => Equals(p.First, p.Second) || (p.First is byte[] x && p.Second is byte[] y && x.AsSpan().SequenceEqual(y)));

    /// <summary>Runs <paramref name="run"/> in a transaction of each of <paramref name="sources"/>.</summary>
    void Within(IReadOnlyList<string> sources, Action run)
    {
        if (sources.Count == 0)
            run();
        else
            Atomic(sources[0], () => Within([.. sources.Skip(1)], run));
    }

    /// <summary>A row's raw values (its columns), or null when it is not there.</summary>
    object?[]? Row(string source, string table, string key, IReadOnlyList<string> columns, long id)
    {
        var conn = Conn(source);
        var d = conn.Dialect;
        var names = columns.Count == 0 ? d.Quote(key) : string.Join(", ", columns.Select(c => d.Quote(c)));
        var rows = conn.Rows($"SELECT {names} FROM {d.Quote(table)} WHERE {d.Quote(key)} = {d.Mark(1)}", [id]);
        return rows.Count == 0 ? null : columns.Count == 0 ? [] : rows[0];
    }

    /// <summary>Makes a row so: gone, or with these values and its own id.</summary>
    void Put(Change change, object?[]? values)
    {
        var there = Row(change.Source, change.Table, change.Key, change.Columns, change.Id) is not null;
        var conn = Conn(change.Source);
        var d = conn.Dialect;
        var table = d.Quote(change.Table);
        var key = d.Quote(change.Key);
        if (values is null)
        {
            if (there)
                conn.Execute($"DELETE FROM {table} WHERE {key} = {d.Mark(1)}", [change.Id]);
            return;
        }
        if (there)
        {
            if (change.Columns.Count == 0)
                return;
            var sets = string.Join(", ", change.Columns.Select((c, i) => $"{d.Quote(c)} = {d.Mark(i + 1)}"));
            conn.Execute($"UPDATE {table} SET {sets} WHERE {key} = {d.Mark(change.Columns.Count + 1)}", [.. values, change.Id]);
            return;
        }
        var names = string.Join(", ", new[] { key }.Concat(change.Columns.Select(c => d.Quote(c))));
        var marks = string.Join(", ", Enumerable.Range(1, change.Columns.Count + 1).Select(i => d.Mark(i)));
        var insert = $"INSERT INTO {table} ({names}) VALUES ({marks})";
        // SQL Server takes a key of its own counting only when asked to.
        if (d == Dialect.Mssql)
        {
            conn.Batch($"SET IDENTITY_INSERT {table} ON");
            try
            {
                conn.Execute(insert, [change.Id, .. values]);
            }
            finally
            {
                conn.Batch($"SET IDENTITY_INSERT {table} OFF");
            }
        }
        else
            conn.Execute(insert, [change.Id, .. values]);
    }
}
