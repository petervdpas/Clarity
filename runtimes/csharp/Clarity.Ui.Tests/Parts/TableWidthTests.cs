// The widths a person gives a table's columns by dragging a header edge are
// kept for the next start, per window and table (Remembered, key
// "columns.<Window>.<table>"), and a double click on an edge gives the
// designed widths back and forgets the person's.

using Clarity.Runtime;

namespace Clarity.Ui.Tests;

[CollectionDefinition("Remembered", DisableParallelization = true)]
public sealed class RememberedCollection;

[Collection("Remembered")]
public class TableWidthTests
{
    static (Screen Screen, TablePart Table) Shown(string window, DataHost host)
    {
        var spec = WindowMarkup.Read($"""<Window name="{window}"><Table at="1,1" span="12" rows="6" name="grid" bind="orders" columns="title 150, hours" resizable="true" /></Window>""");
        var view = new WindowView(spec, host);
        var screen = new Screen(view, ThemeLook.Clarity(dark: false));
        screen.Resize(800, 400, 1);
        return (screen, Assert.IsType<TablePart>(view.Parts[0]));
    }

    static DataHost Host()
    {
        var host = new DataHost();
        host.Lists["orders"] = [new OrderRow { Title = "One", Hours = 1 }];
        return host;
    }

    [Fact]
    public void The_persons_widths_are_remembered_per_window_and_table_and_a_reset_gives_the_designed_ones()
    {
        Remembered.Folder = null;
        Remembered.Set("columns.Other.grid", "");
        Remembered.Set("columns.Main.grid", """{"title":220,"hours":90}""");
        var (screen, table) = Shown("Main", Host());
        Assert.Equal(220, table.HeaderBox("title")!.Value.Width);
        Assert.Equal(90, table.HeaderBox("hours")!.Value.Width);
        Assert.Equal(150, Shown("Other", Host()).Table.HeaderBox("title")!.Value.Width);

        // A double click on an edge: the designed widths, and none kept.
        var edge = table.HeaderBox("title")!.Value;
        screen.PointerPressed(edge.Right - 2, edge.Y + 10, PointerButton.Left, 2);
        screen.PointerReleased(edge.Right - 2, edge.Y + 10, PointerButton.Left);
        Assert.Equal(150, table.HeaderBox("title")!.Value.Width);
        Assert.Equal(150, Shown("Main", Host()).Table.HeaderBox("title")!.Value.Width);
        Assert.Equal("", Remembered.Get("columns.Main.grid"));
    }

    [Fact]
    public void Widths_the_person_drags_are_kept_when_the_edge_is_let_go()
    {
        Remembered.Folder = null;
        Remembered.Set("columns.Drag.grid", "");
        var (screen, table) = Shown("Drag", Host());
        var title = table.HeaderBox("title")!.Value;
        screen.PointerPressed(title.Right - 2, title.Y + 10, PointerButton.Left);
        screen.PointerMoved(title.Right + 30, title.Y + 10);
        // Kept only once the edge is let go.
        Assert.Equal("", Remembered.Get("columns.Drag.grid"));
        screen.PointerReleased(title.Right + 30, title.Y + 10, PointerButton.Left);
        Assert.Equal(182, Shown("Drag", Host()).Table.HeaderBox("title")!.Value.Width);
        // A table of a window without a name, or without a name of its own, keeps nothing.
        var (other, unnamed) = Made.Window("""<Table at="1,1" span="12" rows="6" bind="orders" columns="title 150, hours" resizable="true" />""", Host()) is var made ? (made.Screen, (TablePart)made.Parts[0]) : default;
        var box = unnamed.HeaderBox("title")!.Value;
        other.PointerPressed(box.Right - 2, box.Y + 10, PointerButton.Left);
        other.PointerMoved(box.Right + 30, box.Y + 10);
        other.PointerReleased(box.Right + 30, box.Y + 10, PointerButton.Left);
        Assert.Equal(182, unnamed.HeaderBox("title")!.Value.Width);
    }
}
