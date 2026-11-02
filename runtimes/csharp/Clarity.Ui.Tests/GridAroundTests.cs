// A part that holds a grid of its own (a splitter's panes, a tabs page under
// its strip, a panel under its header) adds room around that grid: the
// window's grid gives its rows that room too, so the inner grid fits and only
// the window scrolls.

namespace Clarity.Ui.Tests;

public class GridAroundTests
{
    [Fact]
    public void A_growing_part_too_low_gets_its_rows_and_the_room_around_them()
    {
        var placed = GridLayout.Place([new GridItem(1, 1, 12, 3, true, Around: 20)], 12, 600, 100);
        Assert.Equal(3 * 36 + 2 * 8 + 20, placed.Boxes[0].Height);
        Assert.Equal(3 * 36 + 2 * 8 + 20, placed.Content);
    }

    [Fact]
    public void A_growing_part_with_room_enough_fills_its_place()
    {
        var placed = GridLayout.Place([new GridItem(1, 1, 12, 3, true, Around: 20), new GridItem(1, 4, 12, 1, false)], 12, 600, 400);
        Assert.Equal(400 - 8 - 36, placed.Boxes[0].Height);
        Assert.Equal(400, placed.Content);
    }

    [Fact]
    public void A_part_that_does_not_grow_keeps_its_rows_and_the_room_around_them()
    {
        var placed = GridLayout.Place([new GridItem(1, 1, 12, 2, false, Around: 20), new GridItem(1, 3, 12, 1, false)], 12, 600, 400);
        Assert.Equal(2 * 36 + 8 + 20, placed.Boxes[0].Height);
        // The part below starts under it.
        Assert.Equal(2 * 36 + 8 + 20 + 8, placed.Boxes[1].Y);
    }

    [Fact]
    public void Rows_with_more_room_keep_it_and_the_others_share_the_rest()
    {
        // Two growing parts side by side in rows 1-2: the one with room around makes row 2 higher.
        var placed = GridLayout.Place([new GridItem(1, 1, 6, 2, true, Around: 100), new GridItem(7, 1, 6, 2, true)], 12, 600, 120);
        Assert.Equal(2 * 36 + 8 + 100, placed.Boxes[0].Height);
        Assert.Equal(2 * 36 + 8 + 100, placed.Content);
    }

    sealed class Holder(double around) : Element
    {
        public override double Around => around;
    }

    [Fact]
    public void A_grid_panel_asks_each_part_for_the_room_around_it()
    {
        var panel = new GridPanel(12);
        var holder = new Holder(30);
        panel.Add(holder, new GridItem(1, 1, 12, 2, true));
        panel.Arrange(new Box(0, 0, 600, 50));
        Assert.Equal(2 * 36 + 8 + 30, holder.Bounds.Height);
        Assert.Equal(2 * 36 + 8 + 30, panel.Content);
    }
}
