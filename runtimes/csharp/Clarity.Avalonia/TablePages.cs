// A table shown a page at a time (`page 50`, U1c), the same as on the web
// (conformance/table-pages.json): the ordered rows cut into pages, a page
// kept within the pages there are, and the text between the bar's buttons.

namespace Clarity.Avalonia;

/// <summary>A page of a table: the page shown, how many there are, and the first and last row shown (from 1).</summary>
public sealed record Paged(int Page, int Pages, int From, int To, string Text);

public static class TablePages
{
    /// <summary>Page <paramref name="page"/> of <paramref name="count"/> rows, <paramref name="size"/> to a page.</summary>
    public static Paged Of(int count, int size, int page)
    {
        var pages = Math.Max(1, (count + size - 1) / size);
        var shown = Math.Clamp(page, 1, pages);
        if (count == 0)
            return new Paged(1, 1, 0, 0, "0 / 0");
        var from = (shown - 1) * size + 1;
        var to = Math.Min(shown * size, count);
        return new Paged(shown, pages, from, to, $"{from}-{to} / {count}");
    }

    /// <summary>The page that shows the row at place <paramref name="at"/> of the shown order, counted from 0.</summary>
    public static int With(int size, int at) => at / size + 1;
}
