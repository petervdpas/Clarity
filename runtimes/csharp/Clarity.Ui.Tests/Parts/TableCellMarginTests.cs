// An edited cell of a table takes a press on its input; a press beside the
// input, in the cell's margin, chooses the row, as on the shared web screen
// where only a click inside the cell's input chooses no row. A table whose
// every column is edited (todo's tasks) can choose a row this way.

namespace Clarity.Ui.Tests;

public class TableCellMarginTests
{
    static (Screen Screen, TablePart Table, DataHost Host) Tasks()
    {
        var host = new DataHost();
        host.Lists["tasks"] = [new CaseTask { Title = "Milk" }, new CaseTask { Title = "Bread" }];
        var (screen, _, parts) = Made.Window("""<Table at="1,1" span="12" rows="6" name="grid" bind="tasks" columns="title, done" edit="title, done" switches="done" onselect="picked" />""", host, 800, 400);
        return (screen, Assert.IsType<TablePart>(parts[0]), host);
    }

    static void Press(Screen screen, double x, double y)
    {
        screen.PointerPressed(x, y, PointerButton.Left);
        screen.PointerReleased(x, y, PointerButton.Left);
    }

    [Fact]
    public void A_press_beside_a_switch_chooses_the_row_and_leaves_the_value()
    {
        var (screen, table, host) = Tasks();
        var cell = table.CellBox(1, "done")!.Value;
        Press(screen, cell.X + 2, cell.Y + cell.Height / 2);
        Assert.Equal(1, table.ChosenPlace);
        Assert.False(((CaseTask)host.Lists["tasks"][1]).Done);
    }

    [Fact]
    public void A_press_on_the_switch_turns_it()
    {
        var (screen, table, host) = Tasks();
        var cell = table.CellBox(1, "done")!.Value;
        Press(screen, cell.X + cell.Width / 2, cell.Y + cell.Height / 2);
        Assert.True(((CaseTask)host.Lists["tasks"][1]).Done);
    }

    [Fact]
    public void A_press_in_the_margin_of_a_typed_cell_chooses_the_row_without_typing()
    {
        var (screen, table, _) = Tasks();
        var cell = table.CellBox(1, "title")!.Value;
        Press(screen, cell.X + 1, cell.Y + cell.Height / 2);
        Assert.Equal(1, table.ChosenPlace);
        Assert.Null(table.CellEditor);
        Press(screen, cell.X + cell.Width / 2, cell.Y + cell.Height / 2);
        Assert.NotNull(table.CellEditor);
    }
}
