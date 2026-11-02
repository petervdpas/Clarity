// What the screen tells its elements beyond the pointer: keys let go, the
// keys held while the wheel turns, joining and leaving the screen, a timed
// wake-up, and a tooltip after the pointer rests. A dialog on top hears keys
// before the part with the focus, and an overlay that covers the screen
// follows its size.

namespace Clarity.Ui.Tests;

sealed class Hooked : Element
{
    public readonly List<string> Seen = [];
    public string? Tip;
    public bool TakesFocus = true;

    public override bool Focusable => TakesFocus;

    public void Add(Element child) => AddChild(child);

    public void Remove(Element child) => RemoveChild(child);

    public override void Arrange(Box box)
    {
        Bounds = box;
        foreach (var c in Children)
            c.Arrange(box.Inset(10));
    }

    protected override void OnAttached() => Seen.Add("joined");

    protected override void OnDetached() => Seen.Add("left");

    public override void OnKeyUp(KeyInput key) => Seen.Add($"up {key.Key}");

    public override bool OnKey(KeyInput key)
    {
        Seen.Add($"down {key.Key}");
        return true;
    }

    public override bool OnWheel(double dx, double dy, Modifiers held)
    {
        Seen.Add($"wheel {dy} {held}");
        return true;
    }

    public override void OnWake() => Seen.Add("woke");

    public override string? TooltipAt(double x, double y) => Tip;
}

sealed class KeyDialog : Element, IDialogKeys
{
    public readonly List<Key> Heard = [];

    public bool KeyPressed(KeyInput key)
    {
        Heard.Add(key.Key);
        return key.Key == Key.Escape;
    }
}

public class ScreenHooksTests
{
    static (Screen Screen, Hooked Root) Shown()
    {
        var root = new Hooked();
        var screen = new Screen(root, ThemeLook.Clarity(dark: false));
        screen.Resize(400, 300, 1);
        return (screen, root);
    }

    [Fact]
    public void A_key_let_go_reaches_the_focused_element_and_modifier_keys_are_keys()
    {
        var (screen, root) = Shown();
        screen.Focus(root);
        screen.KeyPressed(new KeyInput(Key.Shift, Modifiers.Shift));
        screen.KeyReleased(new KeyInput(Key.Shift, Modifiers.None));
        screen.KeyReleased(new KeyInput(Key.Control, Modifiers.None));
        screen.KeyReleased(new KeyInput(Key.Alt, Modifiers.None));
        Assert.Equal(["joined", "down Shift", "up Shift", "up Control", "up Alt"], root.Seen);
    }

    [Fact]
    public void The_wheel_carries_the_keys_held()
    {
        var (screen, root) = Shown();
        screen.Wheel(50, 50, 0, 40, Modifiers.Control);
        Assert.Equal("wheel 40 Control", root.Seen[^1]);
    }

    [Fact]
    public void An_element_hears_when_it_joins_and_leaves_its_screen()
    {
        var (screen, root) = Shown();
        var inner = new Hooked();
        var deeper = new Hooked();
        inner.Add(deeper);
        Assert.Empty(inner.Seen);
        root.Add(inner);
        Assert.Equal(["joined"], inner.Seen);
        Assert.Equal(["joined"], deeper.Seen);
        Assert.True(deeper.Attached);
        root.Remove(inner);
        Assert.Equal(["joined", "left"], deeper.Seen);
        Assert.False(deeper.Attached);
        // An overlay's content joins as it opens and leaves as it closes.
        var over = new Hooked();
        var overlay = new Overlay(over, new Box(0, 0, 50, 50));
        screen.Open(overlay);
        screen.Close(overlay);
        Assert.Equal(["joined", "left"], over.Seen);
    }

    [Fact]
    public void A_screen_disposed_closes_its_overlays_and_its_elements_leave_it()
    {
        var (screen, root) = Shown();
        var over = new Hooked();
        screen.Open(new Overlay(over, new Box(0, 0, 10, 10)));
        screen.Dispose();
        Assert.Empty(screen.Overlays);
        Assert.Equal(["joined", "left"], root.Seen);
        Assert.Equal(["joined", "left"], over.Seen);
    }

    [Fact]
    public void An_element_wakes_once_at_its_time_and_not_after_it_left()
    {
        var (screen, root) = Shown();
        var now = new DateTime(2026, 11, 2, 12, 0, 0);
        screen.Now = () => now;
        screen.Wake(root, now.AddSeconds(1));
        Assert.Equal(now.AddSeconds(1), screen.NextWake);
        screen.WakeDue(now);
        Assert.DoesNotContain("woke", root.Seen);
        screen.WakeDue(now.AddSeconds(1));
        screen.WakeDue(now.AddSeconds(2));
        Assert.Single(root.Seen, s => s == "woke");
        Assert.Null(screen.NextWake);
        var inner = new Hooked();
        root.Add(inner);
        screen.Wake(inner, now);
        root.Remove(inner);
        screen.WakeDue(now);
        Assert.DoesNotContain("woke", inner.Seen);
    }

    [Fact]
    public void A_tooltip_shows_after_the_pointer_rests_and_goes_when_it_moves_off_or_presses()
    {
        var (screen, root) = Shown();
        var now = new DateTime(2026, 11, 2, 12, 0, 0);
        screen.Now = () => now;
        root.Tip = "Save (Ctrl+S)";
        screen.PointerMoved(100, 100);
        Assert.Null(screen.Tip);
        Assert.Equal(now + Screen.TipPause, screen.NextWake);
        screen.WakeDue(now + Screen.TipPause);
        Assert.Equal("Save (Ctrl+S)", screen.Tip?.Text);
        Assert.True(screen.Tip!.Place.Y > 100);
        // Moving over the same place keeps it; pressing hides it.
        screen.PointerMoved(102, 100);
        Assert.NotNull(screen.Tip);
        screen.PointerPressed(102, 100, PointerButton.Left);
        Assert.Null(screen.Tip);
        screen.PointerReleased(102, 100, PointerButton.Left);
        // Over nothing that says something, none waits.
        root.Tip = null;
        screen.PointerMoved(150, 100);
        Assert.Null(screen.NextWake);
        // It draws in the theme without failing.
        root.Tip = "Zoom in";
        screen.PointerMoved(200, 100);
        screen.WakeDue(now + Screen.TipPause);
        Shot.Of(screen, 400, 300);
        screen.PointerLeft();
        Assert.Null(screen.Tip);
    }

    [Fact]
    public void A_dialog_on_top_hears_keys_before_the_focused_element()
    {
        var (screen, root) = Shown();
        var dialog = new KeyDialog();
        var inside = new Hooked();
        screen.Open(new Overlay(dialog, screen.Whole) { Modal = true, Covers = true });
        screen.Focus(root);
        screen.KeyPressed(new KeyInput(Key.Escape, Modifiers.None));
        screen.KeyPressed(new KeyInput(Key.A, Modifiers.None));
        Assert.Equal([Key.Escape, Key.A], dialog.Heard);
        // The dialog took Escape; A went on to the element with the focus.
        Assert.Equal(["joined", "down A"], root.Seen);
        Assert.Empty(inside.Seen);
    }

    [Fact]
    public void An_overlay_that_covers_the_screen_follows_its_size()
    {
        var (screen, _) = Shown();
        var overlay = new Overlay(new Hooked(), new Box(0, 0, 10, 10)) { Covers = true };
        screen.Open(overlay);
        Assert.Equal(new Box(0, 0, 400, 300), overlay.Place);
        screen.Resize(640, 480, 1);
        Assert.Equal(new Box(0, 0, 640, 480), overlay.Place);
    }
}
