// What a table says where the pointer rests, in Clarity's own words: the
// buttons of its page bar (First page, Next page...) and a tree row's toggle
// (Expand, Collapse); and the pointer's shape over its header edges and buttons.

namespace Clarity.Ui.Tests;

public class TableTipTests
{
    [Fact]
    public void The_page_bar_buttons_say_what_they_do_and_show_a_hand()
    {
        var host = new DataHost();
        host.Lists["albums"] = [.. Enumerable.Range(1, 7).Select(i => (object)new Album { Title = $"Album {i}" })];
        var (screen, _, parts) = Made.Window("""<Table at="1,1" span="12" rows="6" name="albums" bind="albums" columns="title 200" page="3" resizable="true" />""", host);
        var table = Assert.IsType<TablePart>(parts[0]);
        var now = new DateTime(2026, 11, 2, 12, 0, 0);
        screen.Now = () => now;
        var next = table.PagerBox(PageButton.Next)!.Value;
        screen.PointerMoved(next.X + next.Width / 2, next.Y + next.Height / 2);
        screen.WakeDue(now + Screen.TipPause);
        Assert.Equal("Next page", screen.Tip?.Text);
        var first = table.PagerBox(PageButton.First)!.Value;
        Assert.Equal("First page", table.TooltipAt(first.X + 4, first.Y + 4));
        Assert.Equal(Cursor.Hand, table.CursorAt(next.X + 4, next.Y + 4));
        // A header edge of a resizable table shows the pointer that resizes.
        var header = table.HeaderBox("title")!.Value;
        Assert.Equal(Cursor.ResizeLeftRight, table.CursorAt(header.Right - 2, header.Y + 5));
        Assert.Null(table.CursorAt(header.X + 20, header.Y + 5));
        Assert.Null(table.TooltipAt(header.X + 20, header.Y + 5));
    }

    [Fact]
    public void A_tree_toggle_says_whether_it_expands_or_collapses()
    {
        var host = new DataHost();
        host.Lists["folders"] = [new Folder { Id = 1, Name = "Documents" }, new Folder { Id = 2, Name = "Invoices", Parent = 1 }];
        var (screen, _, parts) = Made.Window("""<Tree at="1,1" span="12" rows="8" name="folders" bind="folders" show="name" />""", host);
        var tree = Assert.IsType<TablePart>(parts[0]);
        var now = new DateTime(2026, 11, 2, 12, 0, 0);
        screen.Now = () => now;
        var toggle = tree.ToggleBox(0)!.Value;
        screen.PointerMoved(toggle.X + toggle.Width / 2, toggle.Y + toggle.Height / 2);
        screen.WakeDue(now + Screen.TipPause);
        Assert.Equal("Expand", screen.Tip?.Text);
        tree.Toggle(0);
        Assert.Equal("Collapse", tree.TooltipAt(toggle.X + toggle.Width / 2, toggle.Y + toggle.Height / 2));
        Assert.Equal(Cursor.Hand, tree.CursorAt(toggle.X + toggle.Width / 2, toggle.Y + toggle.Height / 2));
        var name = tree.CellBox(1, "name")!.Value;
        Assert.Null(tree.TooltipAt(name.Right - 10, name.Y + 5));
    }
}
