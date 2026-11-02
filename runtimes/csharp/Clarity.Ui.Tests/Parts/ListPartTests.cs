// A list of records (DESIGN 5.x, list): its rows show the fields of `show`,
// or as cards; the person chooses a row with the pointer or Up and Down,
// which runs `select` only when another row is chosen; rows read again keep
// no row that is gone. Its border inside a container: FramesTests.

namespace Clarity.Ui.Tests;

public class ListPartTests
{
    static (Screen Screen, ListPart List, DataHost Host) Made(string fields = "", int count = 3)
    {
        var host = new DataHost();
        host.Lists["folders"] = [.. Enumerable.Range(1, count).Select(i => (object)new Folder { Id = i, Name = $"Folder {i}", Position = i })];
        var (screen, _, parts) = Clarity.Ui.Tests.Made.Window($"""<List at="1,1" span="6" rows="5" name="folders" bind="folders" show="name, position" onselect="picked" onopen="opened" {fields} />""", host);
        return (screen, Assert.IsType<ListPart>(parts[0]), host);
    }

    static void Click(Screen screen, Box box, int clicks = 1)
    {
        screen.PointerPressed(box.X + box.Width / 2, box.Y + box.Height / 2, PointerButton.Left, clicks);
        screen.PointerReleased(box.X + box.Width / 2, box.Y + box.Height / 2, PointerButton.Left);
    }

    [Fact]
    public void A_row_shows_the_fields_of_show()
    {
        var (_, list, _) = Made();
        Assert.True(list.Grows);
        Assert.Equal(["Folder 1 | 1", "Folder 2 | 2", "Folder 3 | 3"], list.Records.Select(list.TextOf));
    }

    [Fact]
    public void Choosing_a_row_selects_it_once()
    {
        var (screen, list, host) = Made();
        Click(screen, list.RowBox(1)!.Value);
        Click(screen, list.RowBox(1)!.Value);
        Assert.Same(host.Lists["folders"][1], host.State("folders")!.SelectedItem);
        Assert.Equal(["run folders select picked"], host.Log);
        Click(screen, list.RowBox(0)!.Value, clicks: 2);
        Assert.Equal(["select", "select", "open"], host.Events);
    }

    [Fact]
    public void Up_and_down_choose_the_row_before_and_after()
    {
        var (screen, list, host) = Made();
        screen.Focus(list);
        screen.KeyPressed(new KeyInput(Key.Down, Modifiers.None));
        Assert.Equal(0, list.ChosenPlace);
        screen.KeyPressed(new KeyInput(Key.Down, Modifiers.None));
        screen.KeyPressed(new KeyInput(Key.Down, Modifiers.None));
        screen.KeyPressed(new KeyInput(Key.Down, Modifiers.None));
        Assert.Equal(2, list.ChosenPlace);
        screen.KeyPressed(new KeyInput(Key.Up, Modifiers.None));
        Assert.Equal(1, list.ChosenPlace);
        Assert.Equal(["select", "select", "select", "select"], host.Events);
    }

    [Fact]
    public void Rows_read_again_have_no_row_chosen_and_that_is_no_select()
    {
        var (screen, list, host) = Made();
        Click(screen, list.RowBox(0)!.Value);
        // The same records again keep the chosen one.
        host.Change();
        Assert.Equal(0, list.ChosenPlace);
        host.Lists["folders"] = [new Folder { Id = 1, Name = "Folder 1" }];
        host.Change();
        Assert.Equal(-1, list.ChosenPlace);
        Assert.Null(host.State("folders")!.SelectedItem);
        Assert.Equal(["select"], host.Events);
    }

    [Fact]
    public void A_row_chosen_by_code_is_chosen_without_select()
    {
        var (_, list, host) = Made();
        host.State("folders")!.SelectedItem = host.Lists["folders"][2];
        Assert.Equal(2, list.ChosenPlace);
        Assert.Empty(host.Events);
    }

    [Fact]
    public void A_list_draws_its_rows_in_the_list_look()
    {
        var (screen, list, host) = Made();
        host.State("folders")!.SelectedItem = host.Lists["folders"][1];
        using var pixels = Shot.Of(screen, 800, 400);
        var row = list.RowBox(1)!.Value;
        Assert.Equal(screen.Look.Part("list", "selected").Background, pixels.GetPixel((int)row.Right - 20, (int)(row.Y + row.Height / 2)));
        var other = list.RowBox(2)!.Value;
        Assert.Equal(screen.Look.Part("list", null).Background, pixels.GetPixel((int)other.Right - 20, (int)(other.Y + other.Height / 2)));
        // Its border, in the list look.
        Assert.True(list.Framed);
        Assert.Equal(screen.Look.Part("list", null).Border, pixels.GetPixel((int)list.Bounds.X + 30, (int)list.Bounds.Y));
    }

    [Fact]
    public void Cards_show_each_row_in_the_card_look()
    {
        var (screen, list, host) = Made("cards=\"true\"");
        host.State("folders")!.SelectedItem = host.Lists["folders"][0];
        using var pixels = Shot.Of(screen, 800, 400);
        var chosen = list.RowBox(0)!.Value;
        var plain = list.RowBox(1)!.Value;
        // A card is higher than a row: the title, then its lines.
        Assert.True(chosen.Height > 40);
        Assert.Equal(screen.Look.Part("card", "selected").Background, pixels.GetPixel((int)chosen.Right - 10, (int)chosen.Y + 6));
        Assert.Equal(screen.Look.Part("card", null).Background, pixels.GetPixel((int)plain.Right - 10, (int)plain.Y + 6));
        // No frame of its own: between the cards, the window.
        Assert.Equal(screen.Look.Color("background"), pixels.GetPixel((int)plain.Right - 10, (int)plain.Y - 2));
    }

    [Fact]
    public void A_long_lists_rows_leave_room_for_its_bar()
    {
        var (_, list, _) = Made("cards=\"true\"", count: 40);
        Assert.True(list.Scroller.MaxScrollY > 0);
        Assert.Equal(list.Scroller.Bounds.Width - ScrollView.BarThickness, list.RowBox(0)!.Value.Width, 3);
        var (_, few, _) = Made("cards=\"true\"", count: 1);
        Assert.Equal(few.Scroller.Bounds.Width, few.RowBox(0)!.Value.Width, 3);
    }

    [Fact]
    public void A_long_list_scrolls_in_its_own_place()
    {
        var (screen, list, _) = Made(count: 40);
        Assert.NotNull(list.RowBox(0));
        Assert.Null(list.RowBox(39));
        var at = list.Bounds;
        screen.Wheel(at.X + 10, at.Y + 10, 0, 10000);
        Assert.Null(list.RowBox(0));
        Assert.NotNull(list.RowBox(39));
        // Down to the last row shows it.
        screen.Wheel(at.X + 10, at.Y + 10, 0, -10000);
        screen.Focus(list);
        screen.KeyPressed(new KeyInput(Key.End, Modifiers.None));
        Assert.Equal(39, list.ChosenPlace);
        Assert.NotNull(list.RowBox(39));
    }

    [Fact]
    public void A_long_list_scrolls_with_the_bar_of_a_scroll_view()
    {
        var (screen, list, _) = Made(count: 40);
        var view = Assert.Single(list.Children.OfType<ScrollView>());
        var x = view.Bounds.Right - 3;
        // A press on the track below the thumb shows the next page.
        screen.PointerPressed(x, view.Bounds.Bottom - 3, PointerButton.Left);
        screen.PointerReleased(x, view.Bounds.Bottom - 3, PointerButton.Left);
        Assert.Equal(view.Bounds.Height, list.Scrolled, 3);
        Assert.Null(list.RowBox(0));
        // A row under the pointer is still the list's: a press chooses it.
        var row = Enumerable.Range(0, 40).First(p => list.RowBox(p) is { } box && box.Y >= view.Bounds.Y);
        Click(screen, list.RowBox(row)!.Value);
        Assert.Equal(row, list.ChosenPlace);
        Assert.Same(list, screen.Focused);
        using var pixels = Shot.Of(screen, 800, 400);
    }
}
