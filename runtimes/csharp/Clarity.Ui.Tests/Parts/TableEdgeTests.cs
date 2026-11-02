// The line between two headers is grabbed from either side, as a DataGrid's
// resize grip straddles it: a press just right of the line resizes the
// column left of it, not sorts the one it is in (seen live in
// examples/agents/drawing, shots/c1-data-22.png).

namespace Clarity.Ui.Tests;

public class TableEdgeTests
{
    public sealed class Row
    {
        public string Title { get; set; } = "";
        public long Hours { get; set; }
    }

    [Fact]
    public void A_press_just_right_of_a_header_line_resizes_the_column_left_of_it()
    {
        var host = new DataHost();
        host.Lists["orders"] = [new Row { Title = "b", Hours = 2 }, new Row { Title = "a", Hours = 1 }];
        var (screen, _, parts) = Made.Window("""<Table at="1,1" span="12" rows="6" name="orders" bind="orders" columns="title 150, hours" resizable="true" sortable="true" />""", host);
        var table = Assert.IsType<TablePart>(parts[0]);
        var title = table.HeaderBox("title")!.Value;
        screen.PointerMoved(title.Right + 3, title.Y + 10);
        Assert.Equal(Cursor.ResizeLeftRight, screen.Cursor);
        screen.PointerPressed(title.Right + 3, title.Y + 10, PointerButton.Left);
        screen.PointerMoved(title.Right + 53, title.Y + 10);
        screen.PointerReleased(title.Right + 53, title.Y + 10, PointerButton.Left);
        Assert.Equal(200, table.HeaderBox("title")!.Value.Width);
        // No sort: the rows keep their order.
        Assert.Equal("b", ((Row)host.Lists["orders"][0]).Title);
        Assert.Null(table.Order);
    }
}
