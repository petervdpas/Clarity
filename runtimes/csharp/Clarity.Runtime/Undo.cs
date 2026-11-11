// The database side of undo (DESIGN 5.29): the changes of each handler run
// that changed something, by the step's id. The window keeps the edits on
// its bus, each with the id of the step its handler made, and asks for that
// step to be undone or done again. Kept until the application closes.

namespace Clarity.Runtime;

public sealed partial class Db
{
    /// <summary>How many undo steps are kept: the oldest goes when a newer one comes, so a long session does not keep every row it ever changed.</summary>
    public const int UndoSteps = 100;

    /// <summary>Why an undo is refused: what it would take back was changed since, in Clarity's own words with their English.</summary>
    public static Text UndoChangedSinceText() => new("This was changed since, so it cannot be taken back") { Key = "undoChangedSince" };

    /// <summary>Why an undo is refused when its step is older than the ones kept.</summary>
    public static Text UndoTooOldText() => new("This was done too long ago to take back") { Key = "undoTooOld" };

    /// <summary>
    /// A row of a table as it was before a change and as the change left it; null where it was
    /// not there. A versioned row (<paramref name="Version"/>) carries its version in each state,
    /// which undo compares instead of the whole row.
    /// </summary>
    sealed record Change(string Source, string Table, string Key, IReadOnlyList<string> Columns, string? Version, long Id, object?[]? Before, object?[]? After)
    {
        public long? BeforeVersion { get; set; }
        public long? AfterVersion { get; set; }
    }

    sealed class UndoStep(List<Change> changes)
    {
        public List<Change> Changes { get; } = changes;
        public bool Undone { get; set; }
    }

    /// <summary>The changes of the step that runs; null outside a step.</summary>
    List<Change>? recording;

    /// <summary>The sources the running step holds a transaction in, by name: what one handler changes is kept or left whole.</summary>
    readonly List<string> held = [];

    readonly SortedDictionary<long, UndoStep> steps = [];

    /// <summary>The id of the last step a handler made, to undo it later by; 0 before any.</summary>
    public long LastStep { get; private set; }

    /// <summary>
    /// Runs a handler as one step, which undo takes back whole: its id, when it
    /// changed anything, is <see cref="LastStep"/>. Everything it changes is one
    /// transaction in each source it changes: kept when it ends, left whole when it
    /// fails, so it leaves nothing behind. A step inside a step is part of it. The
    /// last <see cref="UndoSteps"/> are kept.
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
            Release(keep: false);
            throw;
        }
        recording = null;
        Release(keep: true);
        if (changes.Count > 0)
        {
            steps[++LastStep] = new UndoStep(changes);
            while (steps.Count > UndoSteps)
                steps.Remove(steps.Keys.First());
        }
        return result;
    }

    /// <summary>Makes the running step hold a transaction in <paramref name="source"/>, from its first change there; outside a step, nothing.</summary>
    void Enlist(string source)
    {
        if (recording is null)
            return;
        var at = At(source);
        if (held.Contains(at.Name))
            return;
        if (at.Depth == 0)
            at.Driver.Batch(at.Driver.Dialect.Begin());
        at.Depth++;
        held.Add(at.Name);
    }

    /// <summary>Ends the transactions a step held: kept or rolled back. When one cannot be kept, the ones not kept yet are rolled back too.</summary>
    void Release(bool keep)
    {
        Exception? failed = null;
        foreach (var name in held)
        {
            var at = At(name);
            if (--at.Depth > 0)
                continue;
            if (keep)
            {
                try
                {
                    at.Driver.Batch(at.Driver.Dialect.Commit());
                    continue;
                }
                catch (Exception e)
                {
                    failed = e;
                    keep = false;
                }
            }
            try
            {
                at.Driver.Batch(at.Driver.Dialect.Rollback());
            }
            catch (ClarityException)
            {
            }
        }
        held.Clear();
        if (failed is not null)
            System.Runtime.ExceptionServices.ExceptionDispatchInfo.Throw(failed);
    }

    /// <summary>
    /// Takes back step <paramref name="id"/>: every row it changed as it was, with its own
    /// id. Refused, changing nothing, when one of its rows changed since; false when it is
    /// undone already.
    /// </summary>
    public bool Undo(long id) => ReplaySteps([id], back: true);

    /// <summary>Does step <paramref name="id"/> again after an undo; false when it is not undone.</summary>
    public bool Redo(long id) => ReplaySteps([id], back: false);

    /// <summary>
    /// Takes back (<paramref name="back"/>) or does again the steps of one edit, in the order
    /// given, as one: all of them or, refused, none. A step older than the ones kept is refused
    /// (<see cref="UndoTooOldText"/>); false when none of them is in the state to be replayed.
    /// </summary>
    public bool ReplaySteps(IReadOnlyList<long> ids, bool back)
    {
        var taken = new List<UndoStep>();
        foreach (var id in ids)
        {
            if (!steps.TryGetValue(id, out var step))
            {
                // A step never made is nothing to replay; one made and no longer kept is refused.
                if (id != 0 && id <= LastStep)
                    throw new RefusedException(UndoTooOldText());
                continue;
            }
            if (step.Undone != back)
                taken.Add(step);
        }
        if (taken.Count == 0)
            return false;
        // The changes in the order they are replayed: the steps as given, each one's changes backwards for an undo.
        var order = taken.SelectMany(step => back ? Enumerable.Reverse(step.Changes) : step.Changes).ToList();
        Replay(order, back);
        foreach (var step in taken)
            step.Undone = back;
        return true;
    }

    /// <summary>
    /// Puts the rows of steps back or forward, after checking that each row is still as the
    /// steps left it (by its version, or the whole row where there is none), all in one
    /// transaction per source, checks included. Each row's new version is noted where the next
    /// replay checks it.
    /// </summary>
    void Replay(List<Change> order, bool back)
    {
        Within([.. order.Select(c => c.Source).Distinct()], () =>
        {
            // Each row as the steps left it (its last change undone, its first redone) must still be so.
            var seen = new HashSet<(string, string, long)>();
            foreach (var change in order)
            {
                if (!seen.Add((change.Source, change.Table, change.Id)))
                    continue;
                var (row, version) = State(change.Source, change.Table, change.Key, change.Columns, change.Version, change.Id);
                var unchanged = change.Version is null
                    ? SameRow(row, back ? change.After : change.Before)
                    : version == (back ? change.AfterVersion : change.BeforeVersion);
                if (!unchanged)
                    throw new RefusedException(UndoChangedSinceText());
            }
            foreach (var change in order)
                Put(change, back ? change.Before : change.After, back ? change.BeforeVersion : change.AfterVersion);
            // The version each row has now is what the next replay finds: noted on its last change in this order.
            var noted = new HashSet<(string, string, long)>();
            foreach (var change in Enumerable.Reverse(order))
            {
                if (change.Version is null || !noted.Add((change.Source, change.Table, change.Id)))
                    continue;
                var version = State(change.Source, change.Table, change.Key, change.Columns, change.Version, change.Id).Version;
                if (back)
                    change.BeforeVersion = version;
                else
                    change.AfterVersion = version;
            }
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

    /// <summary>A row's raw values (its columns) and its version (by <paramref name="version"/>), or null when it is not there.</summary>
    (object?[]? Row, long? Version) State(string source, string table, string key, IReadOnlyList<string> columns, string? version, long id)
    {
        var conn = Conn(source);
        var d = conn.Dialect;
        var names = string.Join(", ", columns.Prepend(key).Concat(version is null ? [] : [version]).Select(c => d.Quote(c)));
        var rows = conn.Rows($"SELECT {names} FROM {d.Quote(table)} WHERE {d.Quote(key)} = {d.Mark(1)}", [id]);
        if (rows.Count == 0)
            return (null, null);
        var row = rows[0];
        return version is null ? (row[1..], null) : (row[1..^1], Sql.GetInteger("version", row[^1]));
    }

    /// <summary>A row's raw values (its columns), or null when it is not there.</summary>
    object?[]? Row(string source, string table, string key, IReadOnlyList<string> columns, long id) => State(source, table, key, columns, null, id).Row;

    /// <summary>
    /// Makes a row so: gone, or with these values and its own id. A versioned row put back gets a
    /// version past both its current one and <paramref name="version"/>, the one of the state it
    /// returns to, so a copy read before is refused.
    /// </summary>
    void Put(Change change, object?[]? values, long? version)
    {
        var (row, current) = State(change.Source, change.Table, change.Key, change.Columns, change.Version, change.Id);
        var there = row is not null;
        var next = Math.Max(current ?? 0, version ?? 0) + 1;
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
            List<string> sets = [.. change.Columns.Select((c, i) => $"{d.Quote(c)} = {d.Mark(i + 1)}")];
            List<object?> parameters = [.. values];
            if (change.Version is { } column)
            {
                parameters.Add(next);
                sets.Add($"{d.Quote(column)} = {d.Mark(parameters.Count)}");
            }
            if (sets.Count == 0)
                return;
            parameters.Add(change.Id);
            conn.Execute($"UPDATE {table} SET {string.Join(", ", sets)} WHERE {key} = {d.Mark(parameters.Count)}", parameters);
            return;
        }
        List<string> names = [key, .. change.Columns.Select(c => d.Quote(c))];
        List<object?> inserted = [change.Id, .. values];
        if (change.Version is { } versionColumn)
        {
            names.Add(d.Quote(versionColumn));
            inserted.Add(next);
        }
        var marks = string.Join(", ", Enumerable.Range(1, inserted.Count).Select(i => d.Mark(i)));
        var insert = $"INSERT INTO {table} ({string.Join(", ", names)}) VALUES ({marks})";
        // SQL Server takes a key of its own counting only when asked to.
        if (d == Dialect.Mssql)
        {
            conn.Batch($"SET IDENTITY_INSERT {table} ON");
            try
            {
                conn.Execute(insert, inserted);
            }
            finally
            {
                conn.Batch($"SET IDENTITY_INSERT {table} OFF");
            }
        }
        else
            conn.Execute(insert, inserted);
    }
}
