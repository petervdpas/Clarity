// Moving a row of a list, table or tree (DESIGN 5.x, reordering): the rows
// in their new order, and where a drop lands on a row. What draws the rows
// (Clarity.Avalonia's RowMoves, Clarity.UI's parts) calls these.

using System.Collections;

namespace Clarity.Ui;

public static class RowOrder
{
    static object? Read(object row, string property) => row.GetType().GetProperty(property)?.GetValue(row);

    static void Write(object row, string property, object? value)
    {
        if (row.GetType().GetProperty(property) is not { CanWrite: true } p)
            return;
        var type = Nullable.GetUnderlyingType(p.PropertyType) ?? p.PropertyType;
        p.SetValue(row, value is null ? null : Convert.ChangeType(value, type));
    }

    /// <summary>
    /// Applies a move to <paramref name="rows"/>: each record gets its number in
    /// <paramref name="by"/> (and its parent in <paramref name="parent"/>), and
    /// the list its new order. The moved record, or null when nothing changed.
    /// </summary>
    public static object? Apply(IList rows, string by, string? parent, Func<IReadOnlyList<TreeRow>, Reordered?> move, int row)
    {
        var list = rows.Cast<object>().ToList();
        TreeRow[] tree = [.. list.Select(r => new TreeRow(Read(r, "Id"), parent is null ? null : Read(r, parent)))];
        if (move(tree) is not { } result)
            return null;
        for (var i = 0; i < result.Order.Length; i++)
        {
            var record = list[result.Order[i]];
            Write(record, by, result.Numbers[i]);
            if (parent is not null)
                Write(record, parent, result.Parents[i]);
        }
        // Moved in place, so the list keeps its records and the screen its rows.
        var shift = rows.GetType().GetMethod("Move", [typeof(int), typeof(int)]);
        for (var i = 0; i < result.Order.Length; i++)
        {
            var record = list[result.Order[i]];
            var now = rows.IndexOf(record);
            if (now == i)
                continue;
            if (shift is not null)
                shift.Invoke(rows, [now, i]);
            else
            {
                rows.RemoveAt(now);
                rows.Insert(i, record);
            }
        }
        return list[row];
    }

    /// <summary>Where a drop at <paramref name="y"/> on a row of <paramref name="height"/> lands.</summary>
    public static DropAt Where(double y, double height, bool tree)
    {
        var part = y / Math.Max(height, 1);
        if (!tree)
            return part < 0.5 ? DropAt.Before : DropAt.After;
        return part < 0.25 ? DropAt.Before : part > 0.75 ? DropAt.After : DropAt.Into;
    }
}
