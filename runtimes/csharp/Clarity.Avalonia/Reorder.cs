// Reordering (DESIGN 5.30), the same as on the web (conformance/reorder.json):
// a row of a list, table, tree or tree table moved by a drag or by Alt and
// an arrow key. The result is the list's new order, each row's parent (in a
// tree) and each row's number at its level, which the part writes into the
// field of `reorder by`. A move that changes no parent or number is refused.

namespace Clarity.Avalonia;

/// <summary>Where a dragged row is dropped: before or after a row, or into it (in a tree).</summary>
public enum DropAt { Before, After, Into }

/// <summary>A key that moves the selected row, with Alt.</summary>
public enum MoveKey { Up, Down, Left, Right }

/// <summary>The list after a move: the old places in the new order, each row's parent and number in that order.</summary>
public sealed record Reordered(int[] Order, object?[] Parents, int[] Numbers);

public static class Reorder
{
    /// <summary>The rows after a drag of the row at <paramref name="row"/> onto the row at <paramref name="target"/>, or null when it changes nothing.</summary>
    public static Reordered? Drag(IReadOnlyList<TreeRow> rows, int row, DropAt where, int target, bool tree) =>
        Move(rows, row, tree, (levels, under, lastOf) =>
        {
            if (target < 0 || target >= rows.Count || target == row || (tree && under(target)))
                return null;
            if (where == DropAt.Into)
                return tree ? (lastOf(target), true, rows[target].Id) : null;
            return (target, where == DropAt.After, tree ? rows[target].Parent : null);
        });

    /// <summary>The rows after Alt and <paramref name="key"/> on the row at <paramref name="row"/>, or null when it changes nothing.</summary>
    public static Reordered? Key(IReadOnlyList<TreeRow> rows, int row, MoveKey key, bool tree) =>
        Move(rows, row, tree, (levels, under, lastOf) =>
        {
            var siblings = Level(levels, levels[row]);
            var k = siblings.IndexOf(row);
            switch (key)
            {
                case MoveKey.Up or MoveKey.Down:
                    var next = key == MoveKey.Up ? k - 1 : k + 1;
                    if (next < 0 || next >= siblings.Count)
                        return null;
                    return (siblings[next], key == MoveKey.Down, rows[row].Parent);
                case MoveKey.Left:
                    if (!tree || levels[row] == -1)
                        return null;
                    return (levels[row], true, rows[levels[row]].Parent);
                default:
                    if (!tree || k == 0)
                        return null;
                    var before = siblings[k - 1];
                    return (lastOf(before), true, rows[before].Id);
            }
        });

    /// <summary>The places of the rows whose parent is the row at <paramref name="place"/> (-1: the top), in the order of the list.</summary>
    static List<int> Level(int[] levels, int place) => [.. levels.Select((l, i) => (l, i)).Where(x => x.l == place).Select(x => x.i)];

    /// <summary>Each row's place at its level counted from 1, in the order of the list.</summary>
    static int[] Numbered(int[] levels, IEnumerable<int> order)
    {
        var counts = new Dictionary<int, int>();
        return [.. order.Select(place => counts[levels[place]] = counts.GetValueOrDefault(levels[place]) + 1)];
    }

    static Reordered? Move(
        IReadOnlyList<TreeRow> given, int row, bool tree,
        Func<int[], Func<int, bool>, Func<int, int>, (int Anchor, bool After, object? Parent)?> where)
    {
        if (row < 0 || row >= given.Count)
            return null;
        IReadOnlyList<TreeRow> rows = [.. given.Select(r => tree ? r : r with { Parent = null })];
        var levels = Trees.Parents(rows);
        bool Under(int place)
        {
            for (var p = place; p != -1; p = levels[p])
                if (p == row)
                    return true;
            return false;
        }
        int LastOf(int place) => Level(levels, place).Where(i => i != row).Append(place).Max();
        if (where(levels, Under, LastOf) is not var (anchor, after, parent))
            return null;
        var order = Enumerable.Range(0, rows.Count).Where(i => i != row).ToList();
        order.Insert(order.IndexOf(anchor) + (after ? 1 : 0), row);
        var moved = rows.Select((r, i) => i == row ? r with { Parent = parent } : r).ToArray();
        var newLevels = Trees.Parents([.. order.Select(i => moved[i])]);
        // Levels by old place, so both lists are numbered alike.
        var byPlace = new int[rows.Count];
        for (var i = 0; i < order.Count; i++)
            byPlace[order[i]] = newLevels[i] == -1 ? -1 : order[newLevels[i]];
        var numbers = Numbered(byPlace, order);
        var before = Numbered(levels, Enumerable.Range(0, rows.Count));
        var same = order.Select((place, i) => numbers[i] == before[place] && Equals(moved[place].Parent, rows[place].Parent)).All(x => x);
        return same ? null : new Reordered([.. order], [.. order.Select(i => moved[i].Parent)], numbers);
    }
}
