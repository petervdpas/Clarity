// Trees of U1c, the same as on the web (conformance/trees.json): the rows
// of a list arranged by their parent, a row under its parent when that is
// in the list and at the top otherwise, each level in the order of the
// list. A loop of parents is broken at its row that comes first in the list.

namespace Clarity.Avalonia;

/// <summary>A row as a tree places it: its id, and the id of its parent or none.</summary>
public readonly record struct TreeRow(object? Id, object? Parent);

/// <summary>A row shown: its place in the list, its depth, and whether rows are under it.</summary>
public readonly record struct TreeShown(int Place, int Depth, bool Children);

public static class Trees
{
    /// <summary>The place of each row's parent in the list, or -1 for a row at the top.</summary>
    public static int[] Parents(IReadOnlyList<TreeRow> rows)
    {
        var at = new Dictionary<object, int>();
        for (var i = 0; i < rows.Count; i++)
            if (rows[i].Id is { } id)
                at.TryAdd(id, i);
        var parent = rows.Select(r => r.Parent is { } p && at.TryGetValue(p, out var place) ? place : -1).ToArray();
        // Break each loop at its row that comes first in the list.
        for (var i = 0; i < rows.Count; i++)
        {
            var seen = new List<int>();
            var p = i;
            while (p != -1 && !seen.Contains(p))
            {
                seen.Add(p);
                p = parent[p];
            }
            if (p != -1)
                parent[seen.Skip(seen.IndexOf(p)).Min()] = -1;
        }
        return parent;
    }

    /// <summary>The rows shown, top to bottom, with <paramref name="open"/> the ids of the rows shown open.</summary>
    public static List<TreeShown> Shown(IReadOnlyList<TreeRow> rows, ISet<object?> open)
    {
        var parent = Parents(rows);
        var under = rows.Select(_ => new List<int>()).ToArray();
        var tops = new List<int>();
        for (var i = 0; i < rows.Count; i++)
            (parent[i] == -1 ? tops : under[parent[i]]).Add(i);
        var shown = new List<TreeShown>();
        void Walk(int place, int depth)
        {
            shown.Add(new TreeShown(place, depth, under[place].Count > 0));
            if (open.Contains(rows[place].Id))
                foreach (var child in under[place])
                    Walk(child, depth + 1);
        }
        foreach (var top in tops)
            Walk(top, 0);
        return shown;
    }

    /// <summary>The ids of the rows above the row at <paramref name="at"/>, top first: what a selected row opens.</summary>
    public static List<object?> Above(IReadOnlyList<TreeRow> rows, int at)
    {
        var parent = Parents(rows);
        var ids = new List<object?>();
        for (var p = parent[at]; p != -1; p = parent[p])
            ids.Insert(0, rows[p].Id);
        return ids;
    }
}
