// A window that names its size and opens in place in the desktop window
// grows that window to its size, as far as the screen has room, and never
// makes it smaller than the person left it. A sidebar that opens grows it by
// the room it takes, so the window's own parts keep theirs.

using Clarity.Runtime;
using Clarity.Ui.Desktop;

namespace Clarity.Ui.Tests;

public class DesktopGrowTests
{
    [Fact]
    public void A_larger_window_grows_the_desktop_window()
    {
        Assert.Equal(new Extent(1100, 680), DesktopWindow.Grown(new Extent(900, 560), new Extent(1100, 680), new Extent(1920, 1080)));
    }

    [Fact]
    public void It_never_shrinks_and_stays_on_the_screen()
    {
        Assert.Equal(new Extent(1000, 900), DesktopWindow.Grown(new Extent(1000, 900), new Extent(800, 600), new Extent(1920, 1080)));
        Assert.Equal(new Extent(1280, 720), DesktopWindow.Grown(new Extent(900, 560), new Extent(1600, 1000), new Extent(1280, 720)));
        // Without a size of its own, or with a screen of unknown size, nothing changes beyond what it asks.
        Assert.Equal(new Extent(900, 560), DesktopWindow.Grown(new Extent(900, 560), null, new Extent(1920, 1080)));
        Assert.Equal(new Extent(1100, 680), DesktopWindow.Grown(new Extent(900, 560), new Extent(1100, 680), null));
    }

    [Fact]
    public void A_sidebar_that_opens_grows_the_window_by_its_room()
    {
        // Noughts: 405 wide with its sidebar folded to its strip (28), opened to 320.
        Assert.Equal(new Extent(697, 510), DesktopWindow.SidebarGrown(new Extent(405, 510), 28, 320, new Extent(1920, 1080)));
        // Folding it again leaves the window as it is; so does the same width.
        Assert.Equal(new Extent(697, 510), DesktopWindow.SidebarGrown(new Extent(697, 510), 320, 28, new Extent(1920, 1080)));
        Assert.Equal(new Extent(697, 510), DesktopWindow.SidebarGrown(new Extent(697, 510), 320, 320, new Extent(1920, 1080)));
        // At most as wide as the screen.
        Assert.Equal(new Extent(1000, 510), DesktopWindow.SidebarGrown(new Extent(900, 510), 28, 320, new Extent(1000, 1080)));
    }

    [Fact]
    public void A_window_view_says_how_wide_its_sidebars_are()
    {
        var spec = WindowMarkup.Read("""<Window name="Game"><Sidebar side="left" name="help" width="320" collapsible="true" collapsed="true"><Text name="rules" at="1,1" span="12" /></Sidebar><Button name="go" at="1,1" span="12" /></Window>""");
        var host = new TestHost();
        // As the generated view model starts it: folded.
        host.States["help"] = new PartState("help") { IsCollapsed = true };
        var view = new WindowView(spec, host);
        var screen = new Screen(view, ThemeLook.Clarity(dark: false));
        screen.Resize(405, 510, 1);
        Assert.Equal(FrameRules.StripWidth, view.SidebarsWidth);
        view.Frame.Left!.Fold(false);
        Assert.Equal(320, view.SidebarsWidth);
    }
}
