// A host that gives only presses (the desktop's window) counts the clicks
// itself: a press soon after and near the last one of the same button is the
// next click of a double or triple click.

namespace Clarity.Ui.Tests;

public class PressCounterTests
{
    [Fact]
    public void Presses_close_in_time_and_place_count_up()
    {
        var counter = new PressCounter();
        Assert.Equal(1, counter.Press(10, 10, PointerButton.Left, 1000));
        Assert.Equal(2, counter.Press(11, 12, PointerButton.Left, 1200));
        Assert.Equal(3, counter.Press(11, 12, PointerButton.Left, 1400));
    }

    [Fact]
    public void A_late_far_or_other_press_starts_again()
    {
        var counter = new PressCounter();
        Assert.Equal(1, counter.Press(10, 10, PointerButton.Left, 1000));
        Assert.Equal(1, counter.Press(10, 10, PointerButton.Left, 1700));
        Assert.Equal(1, counter.Press(30, 10, PointerButton.Left, 1800));
        Assert.Equal(1, counter.Press(30, 10, PointerButton.Right, 1900));
    }
}
