// A popover lies over the window while its part's state is visible, under
// the part named in its `for`; a press outside it or Escape closes it, and
// its state hears so. In the window's grid it takes nothing.

namespace Clarity.Ui.Tests.Containers;

public class PopoverPartTests
{
    const string Window = """
        <Label name="save" at="1,2" span="3" />
        <Popover name="tip" for="save" at="7,6" span="4" rows="2"><Label name="inside" at="1,1" span="4" /></Popover>
        """;

    static (Screen Screen, PopoverPart Popover, TestHost Host) Made_(bool visible = false)
    {
        var host = new TestHost();
        host.State("tip")!.IsVisible = visible;
        var (screen, _, parts) = Made.Window(Window, host, width: 808);
        return (screen, Assert.IsType<PopoverPart>(parts[1]), host);
    }

    [Fact]
    public void It_opens_under_its_part_while_its_state_is_visible()
    {
        var (screen, popover, host) = Made_();
        Assert.Empty(screen.Overlays);
        host.State("tip")!.IsVisible = true;
        var overlay = Assert.Single(screen.Overlays);
        Assert.Same(popover.Opened, overlay);
        // Under the save part, at its left, kept off the window's edge.
        Assert.Equal(4, overlay.Place.X);
        Assert.Equal(44 + 36 + 4, overlay.Place.Y);
        var (width, height) = popover.Size(screen.Look);
        Assert.Equal((width, height), (overlay.Place.Width, overlay.Place.Height));
        Assert.Equal(4 * PopoverPart.Column + 3 * 8 + 2 * (2 * screen.Look.Padding + screen.Look.Border), width);
        // Its parts are inside it.
        var inside = popover.Inner.Children[0];
        Assert.Same(inside, screen.At(inside.Bounds.X + 5, inside.Bounds.Y + 5));
        Assert.True(ContainerPart.IsInside(inside));
        host.State("tip")!.IsVisible = false;
        Assert.Empty(screen.Overlays);
    }

    [Fact]
    public void A_press_outside_or_escape_closes_it_and_its_state_hears()
    {
        var (screen, _, host) = Made_(visible: true);
        screen.Resize(808, 400, 1);
        Assert.Single(screen.Overlays);
        screen.PointerPressed(700, 380, PointerButton.Left);
        screen.PointerReleased(700, 380, PointerButton.Left);
        Assert.Empty(screen.Overlays);
        Assert.False(host.State("tip")!.IsVisible);
        host.State("tip")!.IsVisible = true;
        Assert.Single(screen.Overlays);
        screen.KeyPressed(new KeyInput(Key.Escape, Modifiers.None));
        Assert.Empty(screen.Overlays);
        Assert.False(host.State("tip")!.IsVisible);
    }

    [Fact]
    public void In_the_windows_grid_it_takes_no_room_and_no_pointer()
    {
        var (screen, popover, _) = Made_();
        Assert.False(popover.IsShown);
        Assert.False(popover.Grows);
        Assert.NotSame(popover, screen.At(popover.Bounds.X + 5, popover.Bounds.Y + 5));
    }

    [Fact]
    public void Above_its_part_when_there_is_no_room_below()
    {
        var host = new TestHost();
        var (screen, _, _) = Made.Window(Window.Replace("at=\"1,2\"", "at=\"1,9\""), host, width: 808, height: 400);
        var overlay = Assert.Single(screen.Overlays);
        // The save part is at 352 to 388; the popover above it.
        Assert.Equal(352 - 4 - overlay.Place.Height, overlay.Place.Y);
    }

    [Fact]
    public void One_without_a_name_never_opens()
    {
        var (screen, _, _) = Made.Window(Window.Replace("name=\"tip\" ", ""), new TestHost(), width: 808);
        screen.Resize(808, 400, 1);
        Assert.Empty(screen.Overlays);
    }
}
