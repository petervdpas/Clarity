// Columns wider together than the table scroll sideways, as Avalonia's
// DataGrid did: a bar across, the wheel turned sideways (or with Shift), the
// header going with the rows, and the cell typed into scrolled into sight.

namespace Clarity.Ui.Tests;

public class TableSidewaysTests
{
    static (Screen Screen, TablePart Table) Wide(string columns = "title 400, hours 400, owner 300")
    {
        var host = new DataHost();
        host.Lists["orders"] = [new OrderRow { Title = "One", Hours = 1, Owner = 3 }, new OrderRow { Title = "Two", Hours = 2 }];
        var (screen, _, parts) = Made.Window($"""<Table at="1,1" span="12" rows="6" name="orders" bind="orders" columns="{columns}" edit="title, owner" />""", host);
        return (screen, Assert.IsType<TablePart>(parts[0]));
    }

    [Fact]
    public void Columns_wider_than_the_table_scroll_sideways_with_their_header()
    {
        var (screen, table) = Wide();
        var view = table.Scroller;
        Assert.Equal(1100, view.ContentWidth);
        Assert.True(view.MaxScrollX > 0);
        Assert.True(table.HeaderBox("owner")!.Value.Right > view.Bounds.Right);
        var title = table.HeaderBox("title")!.Value;
        // Shift and the wheel scroll sideways, and the header goes with the rows.
        screen.Wheel(title.X + 20, title.Bottom + 20, 0, 100, Modifiers.Shift);
        Assert.Equal(100, view.ScrollX);
        Assert.Equal(title.X - 100, table.HeaderBox("title")!.Value.X);
        Assert.Equal(table.HeaderBox("title")!.Value.X, table.CellBox(0, "title")!.Value.X);
        // A wheel that turns sideways by itself too, over the header as over the rows.
        screen.Wheel(title.X + 200, title.Y + 5, 50, 0);
        Assert.Equal(150, view.ScrollX);
        using var pixels = Shot.Of(screen, 800, 400);
    }

    [Fact]
    public void The_cell_typed_into_scrolls_into_sight()
    {
        var (screen, table) = Wide();
        Cases.Click(screen, table.CellBox(0, "title")!.Value);
        screen.KeyPressed(new KeyInput(Key.Tab, Modifiers.None));
        var editor = table.CellEditor!;
        var view = table.Scroller.Bounds;
        Assert.True(table.Scroller.ScrollX > 0);
        Assert.True(editor.Bounds.Right <= view.Right + 0.5 && editor.Bounds.X >= view.X - 0.5, $"editor at {editor.Bounds}, view {view}");
    }

    [Fact]
    public void Columns_that_fit_do_not_scroll_sideways()
    {
        var (_, table) = Wide("title 200, hours");
        Assert.Equal(0, table.Scroller.MaxScrollX);
        Assert.Equal(table.Scroller.Bounds.Right, table.HeaderBox("hours")!.Value.Right, 3);
    }
}
