// Undo (DESIGN 5.29): every edit of a part goes over its window's edit bus
// as a message: the part, the scope it is undone in (the form or panel it is
// in, or the part itself), what changed in the window's data (each name with
// its value before and after), and the id of the database step its handler
// made. The screen takes back the last message of the scope with the focus.

using System.Text.Json.Nodes;

namespace Clarity.Ui;

/// <summary>A value of the window's data an edit changed.</summary>
public sealed record Changed(string Name, JsonNode? Before, JsonNode? After);

/// <summary>One edit of a part.</summary>
public sealed class Edit(string part, string scope, IReadOnlyList<Changed> changes, long step)
{
    public string Part { get; } = part;
    public string Scope { get; } = scope;
    public List<Changed> Changes { get; } = [.. changes];
    public long Step { get; set; } = step;
}

/// <summary>The edits of one window, in order.</summary>
public sealed class EditBus
{
    readonly List<Edit> done = [];
    readonly List<Edit> undone = [];

    /// <summary>An edit happened; one that changed nothing is no message. Ends what could be done again in its scope.</summary>
    public void Push(Edit edit)
    {
        if (edit.Changes.Count == 0 && edit.Step == 0)
            return;
        done.Add(edit);
        undone.RemoveAll(e => e.Scope == edit.Scope);
    }

    /// <summary>The last edit, when it is of <paramref name="part"/>: what a change handler right after it joins.</summary>
    public Edit? LastOf(string part) => done.Count > 0 && done[^1].Part == part ? done[^1] : null;

    public bool CanUndo(string scope) => done.Any(e => e.Scope == scope);

    public bool CanRedo(string scope) => undone.Any(e => e.Scope == scope);

    /// <summary>The last edit of <paramref name="scope"/>, to take back; it is then undone.</summary>
    public Edit? Undo(string scope) => Move(done, undone, scope);

    /// <summary>The last undone edit of <paramref name="scope"/>, to do again.</summary>
    public Edit? Redo(string scope) => Move(undone, done, scope);

    /// <summary>Taking an edit back or doing it again was refused: it stays where it was.</summary>
    public void Refused(Edit edit, bool wasUndo)
    {
        var (from, to) = wasUndo ? (undone, done) : (done, undone);
        from.Remove(edit);
        to.Add(edit);
    }

    static Edit? Move(List<Edit> from, List<Edit> to, string scope)
    {
        var at = from.FindLastIndex(e => e.Scope == scope);
        if (at < 0)
            return null;
        var edit = from[at];
        from.RemoveAt(at);
        to.Add(edit);
        return edit;
    }

    /// <summary>What changed between two copies of the window's data, by name.</summary>
    public static List<Changed> Between(JsonObject before, JsonObject after) =>
        [.. before.Select(p => p.Key).Union(after.Select(p => p.Key))
            .Where(name => !JsonNode.DeepEquals(before[name], after[name]))
            .Select(name => new Changed(name, before[name]?.DeepClone(), after[name]?.DeepClone()))];
}
