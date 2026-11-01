// How a window's grid places its parts, as every version of Clarity.UI does
// (conformance/grid-layout.json): the columns share the width; a row is 36
// high, and the rows of a part that grows share the height left over, never
// less than 36; a part that does not grow keeps its own height at the top of
// its place.

namespace Clarity.Ui.Drawing;

/// <summary>A part's place in a grid: at <paramref name="Col"/>, <paramref name="Row"/> (from 1), so many columns wide and rows high.</summary>
public readonly record struct GridItem(int Col, int Row, int Width, int Height, bool Grows);

/// <summary>Where the parts are, in order, and how high the content is (more than the grid when it scrolls).</summary>
public sealed record GridPlacement(IReadOnlyList<Box> Boxes, double Content);

public static class GridLayout
{
    public const double RowHeight = 36;
    public const double Gap = 8;

    public static GridPlacement Place(IReadOnlyList<GridItem> items, int columns, double width, double height, double gap = Gap)
    {
        columns = Math.Max(1, columns);
        var rows = Math.Max(1, items.Count == 0 ? 1 : items.Max(i => i.Row + Math.Max(1, i.Height) - 1));
        var growing = new HashSet<int>();
        foreach (var item in items.Where(i => i.Grows))
            for (var r = item.Row; r < item.Row + Math.Max(1, item.Height); r++)
                growing.Add(r);
        var heights = new double[rows];
        var spare = height - gap * (rows - 1) - RowHeight * (rows - growing.Count);
        var each = growing.Count == 0 ? RowHeight : Math.Max(RowHeight, spare / growing.Count);
        for (var r = 0; r < rows; r++)
            heights[r] = growing.Contains(r + 1) ? each : RowHeight;
        var tops = new double[rows];
        for (var r = 1; r < rows; r++)
            tops[r] = tops[r - 1] + heights[r - 1] + gap;
        var column = Math.Max(0, (width - gap * (columns - 1)) / columns);
        var boxes = new List<Box>(items.Count);
        foreach (var item in items)
        {
            var row = Math.Clamp(item.Row, 1, rows) - 1;
            var high = Math.Clamp(Math.Max(1, item.Height), 1, rows - row);
            var wide = Math.Max(1, item.Width);
            var x = (item.Col - 1) * (column + gap);
            var w = wide * column + (wide - 1) * gap;
            var h = item.Grows ? heights.Skip(row).Take(high).Sum() + gap * (high - 1) : high * RowHeight + (high - 1) * gap;
            boxes.Add(new Box(x, tops[row], w, h));
        }
        return new GridPlacement(boxes, tops[rows - 1] + heights[rows - 1]);
    }
}
