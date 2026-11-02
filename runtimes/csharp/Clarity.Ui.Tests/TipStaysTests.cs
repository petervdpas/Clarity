// A tooltip shows only for what is under the pointer when its pause is over:
// a press starts no new pause, and a window shown in the meantime (another
// element under the pointer) shows no tip of the one before.

namespace Clarity.Ui.Tests;

public class TipStaysTests
{
    static (Screen, Hooked) Shown()
    {
        var root = new Hooked();
        var screen = new Screen(root, ThemeLook.Clarity(dark: false));
        screen.Resize(400, 300, 1);
        return (screen, root);
    }

    [Fact]
    public void A_press_starts_no_new_pause()
    {
        var (screen, root) = Shown();
        var now = new DateTime(2026, 11, 2, 12, 0, 0);
        screen.Now = () => now;
        root.Tip = "Users";
        screen.PointerMoved(100, 100);
        screen.PointerPressed(100, 100, PointerButton.Left);
        screen.PointerReleased(100, 100, PointerButton.Left);
        screen.WakeDue(now + Screen.TipPause + Screen.TipPause);
        Assert.Null(screen.Tip);
        // Resting on the same place after it shows no tip again until the pointer goes elsewhere.
        screen.PointerMoved(101, 100);
        Assert.Null(screen.NextWake);
    }

    [Fact]
    public void What_is_under_the_pointer_no_more_shows_no_tip()
    {
        var (screen, root) = Shown();
        var now = new DateTime(2026, 11, 2, 12, 0, 0);
        screen.Now = () => now;
        root.Tip = "Users";
        screen.PointerMoved(100, 100);
        // Another window shows under the pointer before the pause is over.
        root.Tip = null;
        screen.WakeDue(now + Screen.TipPause);
        Assert.Null(screen.Tip);
    }
}
