// The pointer over a Markdown link is the hand, as Avalonia's Markdown view
// showed it; over plain text it is the arrow.

namespace Clarity.Ui.Tests;

public class MarkdownLinkCursorTests
{
    [Fact]
    public void The_hand_shows_over_a_link_and_not_over_its_text()
    {
        var host = new TestHost();
        host.Values["notes"] = "See [the site](https://example.com) now.";
        var (screen, _, parts) = Made.Window("""<Markdown name="rules" at="1,1" span="6" rows="4" bind="notes" />""", host);
        using (Shot.Of(screen, 800, 400)) { }
        var part = Assert.IsType<MarkdownPart>(parts[0]);
        (double X, double Y) On(MarkdownRun run) => (part.Page.Bounds.X + run.Box.X + 2, part.Page.Bounds.Y + run.Box.Y + run.Box.Height / 2);
        var (lx, ly) = On(part.Page.Runs.First(r => r.Link is not null));
        screen.PointerMoved(lx, ly);
        Assert.Equal(Cursor.Hand, screen.Cursor);
        var (tx, ty) = On(part.Page.Runs.First(r => r.Text.StartsWith("See")));
        screen.PointerMoved(tx, ty);
        Assert.Equal(Cursor.Default, screen.Cursor);
    }
}
