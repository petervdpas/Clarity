// A window's footer: its status text at the left, in the language shown,
// and its parts on one row of its columns beside it, at the bottom of the
// window under the window's own parts.

namespace Clarity.Ui.Tests;

public class FrameFooterTests
{
    [Fact]
    public void Its_status_shows_in_the_language_shown()
    {
        var host = new TestHost();
        host.Texts["ready"] = "Klaar";
        var (screen, view, _) = Framed.Window("""<Footer status="@ready" columns="12" />""", host: host);
        Framed.Draw(screen);
        var footer = view.Frame.Footer!;
        Assert.Equal("Klaar", footer.Status);
        Assert.Equal(new Box(0, 600 - FrameFooter.StatusHeight, 1000, FrameFooter.StatusHeight), footer.Bounds);
        Assert.Equal(600 - FrameFooter.StatusHeight, view.Frame.Body.Bottom);
    }

    [Fact]
    public void Its_parts_sit_on_one_row_beside_the_status()
    {
        var (screen, view, host) = Framed.Window("""
            <Footer status="Ready" columns="6">
              <Label at="1,1" span="2" caption="12 items" />
              <Button name="sync" at="5,1" span="2" caption="Sync" onclick="sync" />
            </Footer>
            """);
        Framed.Draw(screen);
        var footer = view.Frame.Footer!;
        Assert.Equal(FrameFooter.PartsHeight, footer.Bounds.Height);
        var (label, sync) = (footer.Parts[0], footer.Parts[1]);
        Assert.Equal(label.Bounds.Y, sync.Bounds.Y);
        Assert.True(label.Bounds.X >= footer.StatusBox.Right);
        Assert.True(sync.Bounds.Right <= 1000);
        Framed.Click(screen, sync.Bounds);
        Assert.Equal(["run sync click sync"], host.Log);
    }
}
