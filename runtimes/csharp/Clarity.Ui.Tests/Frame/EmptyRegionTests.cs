// A region of the frame with nothing to show takes no room, as on the web
// screen: a menu bar, toolbar or ribbon whose items (or tabs) code hid all,
// and one written without any. Its room goes to the window's parts.

namespace Clarity.Ui.Tests;

public class EmptyRegionTests
{
    [Theory]
    [InlineData("""<Menu name="m"><Item caption="File" name="file" /></Menu>""", "file")]
    [InlineData("""<Toolbar name="t"><Item caption="Print" name="print" onclick="print" /><Item kind="separator" /></Toolbar>""", "print")]
    [InlineData("""<Ribbon name="r"><Tab title="Home" name="home"><Group title="G"><Item caption="Paste" onclick="paste" /></Group></Tab></Ribbon>""", "home")]
    public void A_region_whose_items_are_all_hidden_takes_no_room(string region, string item)
    {
        var (screen, view, host) = Framed.Window("""<Button at="1,1" span="2" caption="Go" />""", region);
        Framed.Draw(screen);
        Assert.True(view.Frame.Body.Y > 0);
        host.State(item)!.IsVisible = false;
        Framed.Draw(screen);
        Assert.Equal(0, view.Frame.Body.Y);
    }

    [Theory]
    [InlineData("""<Menu name="m" />""")]
    [InlineData("""<Toolbar name="t" />""")]
    [InlineData("""<Ribbon name="r" />""")]
    public void A_region_written_empty_takes_no_room(string region)
    {
        var (screen, view, _) = Framed.Window("""<Button at="1,1" span="2" caption="Go" />""", region);
        Framed.Draw(screen);
        Assert.Equal(0, view.Frame.Body.Y);
    }
}
