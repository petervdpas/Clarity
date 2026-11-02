// A display draws as Avalonia's segment display did: no background or
// frame of its own, its segments in the text color over what is behind it.

namespace Clarity.Ui.Tests;

public class DisplayLookTests
{
    [Fact]
    public void A_display_has_no_background_or_frame_of_its_own()
    {
        var host = new TestHost();
        host.Values["shown"] = "8";
        var (screen, _, parts) = Made.Window("""<Display name="lcd" at="1,1" span="6" rows="3" bind="shown" />""", host);
        using var shot = Shot.Of(screen, 800, 400);
        var box = parts[0].Bounds;
        // Its corner and edge show what is behind it, as the window beside it does.
        var behind = shot.PixelAt((int)box.Right + 20, (int)box.Y + 2);
        Assert.Equal(behind, shot.PixelAt((int)box.X + 1, (int)box.Y + 1));
        Assert.Equal(behind, shot.PixelAt((int)box.X + 3, (int)(box.Y + box.Height / 2)));
    }
}
