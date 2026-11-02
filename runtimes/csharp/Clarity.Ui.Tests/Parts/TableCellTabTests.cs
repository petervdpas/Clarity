// Tab in a table's cell editor takes the cell's text and goes on to the next
// typed cell of the row, then to the first of the next row; Shift+Tab goes
// back. Past the last cell the focus leaves the table, as on the shared web
// screen where every edited cell is an input of its own.

namespace Clarity.Ui.Tests;

public sealed class Relation
{
    public string Multiplicity { get; set; } = "";
    public string Label { get; set; } = "";
}

public class TableCellTabTests
{
    static (Screen Screen, TablePart Table, DataHost Host) Relations()
    {
        var host = new DataHost();
        host.Lists["relations"] = [new Relation { Multiplicity = "1" }, new Relation { Multiplicity = "2" }];
        var (screen, _, parts) = Made.Window("""<Table at="1,1" span="12" rows="6" name="grid" bind="relations" columns="multiplicity, label" edit="multiplicity, label" />""", host, 800, 400);
        return (screen, Assert.IsType<TablePart>(parts[0]), host);
    }

    static KeyInput K(Key key, Modifiers modifiers = Modifiers.None) => new(key, modifiers);

    static Relation Row(DataHost host, int place) => (Relation)host.Lists["relations"][place];

    [Fact]
    public void Tab_takes_the_text_and_types_into_the_next_cell()
    {
        var (screen, table, host) = Relations();
        Cases.Click(screen, table.CellBox(0, "multiplicity")!.Value);
        screen.TextTyped("1..*");
        screen.KeyPressed(K(Key.Tab));
        Assert.Equal("1..*", Row(host, 0).Multiplicity);
        Assert.Same(table.CellEditor, screen.Focused);
        screen.TextTyped("has");
        screen.KeyPressed(K(Key.Enter));
        Assert.Equal("has", Row(host, 0).Label);
    }

    [Fact]
    public void Tab_from_the_last_cell_of_a_row_goes_to_the_next_row_and_shift_tab_comes_back()
    {
        var (screen, table, host) = Relations();
        Cases.Click(screen, table.CellBox(0, "label")!.Value);
        screen.TextTyped("owns");
        screen.KeyPressed(K(Key.Tab));
        Assert.Equal("owns", Row(host, 0).Label);
        Assert.NotNull(table.CellEditor);
        Assert.Equal("2", table.CellEditor!.Text);
        screen.KeyPressed(K(Key.Tab, Modifiers.Shift));
        Assert.Equal("owns", table.CellEditor!.Text);
    }

    [Fact]
    public void Tab_from_the_last_cell_leaves_the_table()
    {
        var (screen, table, host) = Relations();
        Cases.Click(screen, table.CellBox(1, "label")!.Value);
        screen.TextTyped("ends");
        screen.KeyPressed(K(Key.Tab));
        Assert.Equal("ends", Row(host, 1).Label);
        Assert.Null(table.CellEditor);
    }
}
