// The checkbox and the switch: the person's tick is a change, bound or kept
// in the part's state; what code sets is not; a read-only or disabled one
// cannot be ticked; Space ticks the focused one; a ticked box is drawn in
// the accent color.

namespace Clarity.Ui.Tests;

public class CheckPartTests
{
    static void Click(Screen screen, Element part)
    {
        var (x, y) = (part.Bounds.X + 8, part.Bounds.Y + part.Bounds.Height / 2);
        screen.PointerPressed(x, y, PointerButton.Left);
        screen.PointerReleased(x, y, PointerButton.Left);
    }

    static bool Near(SKColor a, SKColor b) => Math.Abs(a.Red - b.Red) <= 10 && Math.Abs(a.Green - b.Green) <= 10 && Math.Abs(a.Blue - b.Blue) <= 10;

    [Fact]
    public void Ticking_a_bound_checkbox_changes_it()
    {
        var host = new TestHost();
        host.Values["task.done"] = false;
        var (screen, _, parts) = Made.Window("""<Checkbox at="1,1" span="3" name="done" bind="task.done" caption="Done" onchange="doneChanged" />""", host);
        var check = Assert.IsType<CheckPart>(parts[0]);
        Click(screen, check);
        Assert.True(check.IsChecked);
        Assert.Equal(["write done task.done True", "run done change doneChanged"], host.Log);
    }

    [Fact]
    public void A_switch_flips_like_a_checkbox()
    {
        var host = new TestHost();
        host.Values["task.urgent"] = false;
        var (screen, _, parts) = Made.Window("""<Switch at="1,1" span="3" name="urgent" bind="task.urgent" onchange="urgentChanged" />""", host);
        var flip = Assert.IsType<CheckPart>(parts[0]);
        Assert.True(flip.IsSwitch);
        Click(screen, flip);
        Click(screen, flip);
        Assert.False(flip.IsChecked);
        Assert.Equal(2, host.Log.Count(l => l.StartsWith("run urgent change")));
    }

    [Fact]
    public void What_code_sets_is_no_change()
    {
        var host = new TestHost();
        host.Values["task.done"] = false;
        var (_, _, parts) = Made.Window("""<Checkbox at="1,1" span="3" name="done" bind="task.done" onchange="doneChanged" />""", host);
        host.Values["task.done"] = true;
        host.Change();
        Assert.True(((CheckPart)parts[0]).IsChecked);
        Assert.Empty(host.Log);
    }

    [Fact]
    public void An_unbound_checkbox_keeps_its_value_in_its_state()
    {
        var host = new TestHost();
        var (screen, _, parts) = Made.Window("""<Checkbox at="1,1" span="3" name="remember" onchange="remembered" />""", host);
        Click(screen, parts[0]);
        Assert.True(host.State("remember")!.IsChecked);
        Assert.Equal(["run remember change remembered"], host.Log);
    }

    [Fact]
    public void A_read_only_or_disabled_checkbox_cannot_be_ticked()
    {
        var host = new TestHost();
        host.Values["a"] = false;
        host.Values["b"] = false;
        var (screen, _, parts) = Made.Window("""
            <Checkbox at="1,1" span="3" name="first" bind="a" readonly="true" />
            <Checkbox at="1,2" span="3" name="second" bind="b" />
            """, host);
        host.State("second")!.IsEnabled = false;
        Click(screen, parts[0]);
        Click(screen, parts[1]);
        Assert.Empty(host.Log);
    }

    [Fact]
    public void Space_ticks_the_focused_checkbox()
    {
        var host = new TestHost();
        host.Values["a"] = false;
        var (screen, _, _) = Made.Window("""<Checkbox at="1,1" span="3" name="first" bind="a" />""", host);
        screen.KeyPressed(new KeyInput(Key.Tab, Modifiers.None));
        screen.KeyPressed(new KeyInput(Key.Space, Modifiers.None));
        Assert.Equal(true, host.Values["a"]);
    }

    [Fact]
    public void A_ticked_box_is_drawn_in_the_accent_color()
    {
        var host = new TestHost();
        host.Values["a"] = false;
        var (screen, _, _) = Made.Window("""<Checkbox at="1,1" span="3" name="first" bind="a" />""", host);
        using (var off = Shot.Of(screen, 800, 400))
            Assert.True(Near(screen.Look.Color("input"), off.PixelAt(5, 14)));
        host.Values["a"] = true;
        using var on = Shot.Of(screen, 800, 400);
        Assert.True(Near(screen.Look.Color("accent"), on.PixelAt(5, 14)));
    }
}
