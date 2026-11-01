// What the person does reaches the right element: the pointer hovers, a press
// and a release on the same element click it, Tab walks the focus, and typed
// text goes to the element with the focus. Each change asks for a new drawing.

namespace Clarity.Ui.Tests;

sealed class Probe : Element
{
    public int Clicks;
    public string Typed = "";
    public List<Key> Keys = [];
    public override bool Focusable => true;
    public override void OnClick() => Clicks++;
    public override void OnText(string text) => Typed += text;
    public override bool OnKey(KeyInput key)
    {
        Keys.Add(key.Key);
        return true;
    }
}

public class InputTests
{
    static (Screen screen, Probe left, Probe right) TwoProbes()
    {
        var grid = new GridPanel(columns: 2);
        var left = new Probe();
        var right = new Probe();
        grid.Add(left, new GridItem(1, 1, 1, 1, false));
        grid.Add(right, new GridItem(2, 1, 1, 1, false));
        var screen = new Screen(grid, ThemeLook.Clarity(dark: false));
        screen.Resize(400, 200, 1);
        return (screen, left, right);
    }

    [Fact]
    public void The_pointer_hovers_one_element_at_a_time()
    {
        var (screen, left, right) = TwoProbes();
        var drawings = 0;
        screen.Invalidated += () => drawings++;
        screen.PointerMoved(50, 10);
        Assert.True(left.Hovered);
        Assert.False(right.Hovered);
        screen.PointerMoved(300, 10);
        Assert.False(left.Hovered);
        Assert.True(right.Hovered);
        screen.PointerMoved(300, 150);
        Assert.False(right.Hovered);
        Assert.Equal(3, drawings);
    }

    [Fact]
    public void A_press_and_release_on_the_same_element_click_it()
    {
        var (screen, left, right) = TwoProbes();
        screen.PointerPressed(50, 10, PointerButton.Left);
        Assert.True(left.Pressed);
        screen.PointerReleased(60, 12, PointerButton.Left);
        Assert.False(left.Pressed);
        Assert.Equal(1, left.Clicks);
        // Pressed on one, released on the other: no click.
        screen.PointerPressed(50, 10, PointerButton.Left);
        screen.PointerMoved(300, 10);
        screen.PointerReleased(300, 10, PointerButton.Left);
        Assert.Equal(1, left.Clicks);
        Assert.Equal(0, right.Clicks);
        // Only the left button clicks.
        screen.PointerPressed(300, 10, PointerButton.Right);
        screen.PointerReleased(300, 10, PointerButton.Right);
        Assert.Equal(0, right.Clicks);
    }

    [Fact]
    public void Pressing_an_element_that_takes_the_focus_gives_it_the_focus()
    {
        var (screen, left, right) = TwoProbes();
        screen.PointerPressed(300, 10, PointerButton.Left);
        screen.PointerReleased(300, 10, PointerButton.Left);
        Assert.True(right.Focused);
        Assert.Same(right, screen.Focused);
    }

    [Fact]
    public void Tab_walks_the_focus_forward_and_Shift_Tab_back()
    {
        var (screen, left, right) = TwoProbes();
        screen.KeyPressed(new KeyInput(Key.Tab, Modifiers.None));
        Assert.Same(left, screen.Focused);
        screen.KeyPressed(new KeyInput(Key.Tab, Modifiers.None));
        Assert.Same(right, screen.Focused);
        screen.KeyPressed(new KeyInput(Key.Tab, Modifiers.None));
        Assert.Same(left, screen.Focused);
        screen.KeyPressed(new KeyInput(Key.Tab, Modifiers.Shift));
        Assert.Same(right, screen.Focused);
        Assert.False(left.Focused);
    }

    [Fact]
    public void Keys_and_text_go_to_the_element_with_the_focus()
    {
        var (screen, left, right) = TwoProbes();
        screen.TextTyped("lost");
        screen.KeyPressed(new KeyInput(Key.Tab, Modifiers.None));
        screen.TextTyped("Hi");
        screen.KeyPressed(new KeyInput(Key.Enter, Modifiers.None));
        Assert.Equal("Hi", left.Typed);
        Assert.Equal([Key.Enter], left.Keys);
        Assert.Equal("", right.Typed);
    }
}
