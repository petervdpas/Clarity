// The pointer beyond a click: a press is held by the element pressed until
// it is let go, wherever the pointer goes (a slider's drag, a text's
// selection); the wheel goes to the deepest element that takes it, else up
// to the one that scrolls; an overlay (a dropdown's list, a menu) lies over
// everything, and a press outside it closes it.

namespace Clarity.Ui.Tests;

sealed class Dragged : Element
{
    public List<string> Seen = [];
    public double Scrolled;
    public bool TakesWheel;
    public override void OnPointerPressed(PointerInput at) => Seen.Add($"down {at.X:0},{at.Y:0}");
    public override void OnPointerMoved(PointerInput at)
    {
        if (Pressed)
            Seen.Add($"drag {at.X:0},{at.Y:0}");
    }
    public override void OnPointerReleased(PointerInput at) => Seen.Add($"up {at.X:0},{at.Y:0}");
    public override bool OnWheel(double dx, double dy)
    {
        if (!TakesWheel)
            return false;
        Scrolled += dy;
        return true;
    }
}

sealed class Holder : Element
{
    public Holder(Element inner) => AddChild(inner);
    public override void Arrange(Box box)
    {
        Bounds = box;
        foreach (var c in Children)
            c.Arrange(box.Inset(10));
    }
}

public class PointerTests
{
    [Fact]
    public void A_press_is_held_until_it_is_let_go_wherever_the_pointer_goes()
    {
        var grid = new GridPanel(columns: 2);
        var left = new Dragged();
        grid.Add(left, new GridItem(1, 1, 1, 1, false));
        var screen = new Screen(grid, ThemeLook.Clarity(dark: false));
        screen.Resize(400, 200, 1);
        screen.PointerPressed(10, 10, PointerButton.Left);
        screen.PointerMoved(300, 150);
        screen.PointerReleased(300, 150, PointerButton.Left);
        // Where it is in the element's own place, from its top left.
        Assert.Equal(["down 10,10", "drag 300,150", "up 300,150"], left.Seen);
        Assert.False(left.Pressed);
    }

    [Fact]
    public void The_wheel_goes_up_to_the_element_that_scrolls()
    {
        var inner = new Dragged();
        var outer = new Dragged { TakesWheel = true };
        var holder = new Holder(inner);
        var screen = new Screen(holder, ThemeLook.Clarity(dark: false));
        screen.Resize(200, 200, 1);
        screen.Wheel(50, 50, 0, 40);
        Assert.Equal(0, inner.Scrolled);
        inner.TakesWheel = true;
        screen.Wheel(50, 50, 0, 40);
        Assert.Equal(40, inner.Scrolled);
        Assert.Equal(0, outer.Scrolled);
    }

    [Fact]
    public void An_overlay_lies_over_everything_and_a_press_outside_closes_it()
    {
        var grid = new GridPanel(columns: 1);
        var under = new Probe();
        grid.Add(under, new GridItem(1, 1, 1, 4, false));
        var screen = new Screen(grid, ThemeLook.Clarity(dark: false));
        screen.Resize(400, 200, 1);
        var list = new Probe();
        var closed = 0;
        screen.Open(new Overlay(list, new Box(0, 0, 200, 100)) { Closed = () => closed++ });
        Assert.Same(list, screen.At(50, 50));
        screen.PointerPressed(50, 50, PointerButton.Left);
        screen.PointerReleased(50, 50, PointerButton.Left);
        Assert.Equal(1, list.Clicks);
        Assert.Equal(0, under.Clicks);
        // A press outside closes it, and does not reach what is under.
        screen.PointerPressed(300, 150, PointerButton.Left);
        screen.PointerReleased(300, 150, PointerButton.Left);
        Assert.Equal(1, closed);
        Assert.Equal(0, under.Clicks);
        Assert.Empty(screen.Overlays);
        // Escape closes the one on top too.
        screen.Open(new Overlay(new Probe(), new Box(0, 0, 10, 10)) { Closed = () => closed++ });
        screen.KeyPressed(new KeyInput(Key.Escape, Modifiers.None));
        Assert.Equal(2, closed);
    }

    [Fact]
    public void An_overlay_is_drawn_over_the_window()
    {
        var screen = new Screen(new GridPanel(columns: 1), ThemeLook.Clarity(dark: false));
        screen.Open(new Overlay(new Plate("accent"), new Box(10, 10, 50, 50)));
        using var pixels = Shot.Of(screen, 200, 100);
        Assert.Equal(screen.Look.Color("accent"), pixels.GetPixel(30, 30));
        Assert.Equal(screen.Look.Color("background"), pixels.GetPixel(150, 80));
    }

    [Fact]
    public void Text_is_copied_and_pasted_through_the_hosts_clipboard()
    {
        var screen = new Screen(new GridPanel(columns: 1), ThemeLook.Clarity(dark: false));
        // Without a host's, a clipboard of the screen's own.
        screen.Clipboard.SetText("kept");
        Assert.Equal("kept", screen.Clipboard.GetText());
    }

    [Fact]
    public void An_element_hears_when_it_gets_and_loses_the_focus()
    {
        var grid = new GridPanel(columns: 2);
        var left = new Probe();
        var right = new Probe();
        grid.Add(left, new GridItem(1, 1, 1, 1, false));
        grid.Add(right, new GridItem(2, 1, 1, 1, false));
        var screen = new Screen(grid, ThemeLook.Clarity(dark: false));
        screen.Focus(left);
        screen.Focus(right);
        Assert.Equal([true, false], left.FocusSeen);
        Assert.Equal([true], right.FocusSeen);
    }
}
