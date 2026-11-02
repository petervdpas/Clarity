// The markdown part: Markdown read into blocks the same way on every stack
// (conformance/markdown.json, tested with MarkdownBlocks in
// Clarity.Runtime.Tests), shown as headings, paragraphs, list items and
// code, wrapped to its width and scrolling when long; its text a resource
// file by name, the file a key names, or the text it binds; a link opening
// through the host.

using System.Text;

namespace Clarity.Ui.Tests;

public class MarkdownPartTests
{
    static (Screen Screen, MarkdownPart Part, TestHost Host) Markdown(string attributes, Action<TestHost>? given = null, double width = 800, double height = 400)
    {
        var host = new TestHost();
        given?.Invoke(host);
        var (screen, _, parts) = Made.Window($"""<Markdown name="rules" at="1,1" span="6" rows="4" {attributes} />""", host, width, height);
        Shot.Of(screen, (int)width, (int)height);
        return (screen, Assert.IsType<MarkdownPart>(parts[0]), host);
    }

    [Fact]
    public void Its_text_is_a_resource_file_a_key_s_text_or_what_it_binds()
    {
        var (_, fromFile, _) = Markdown("""source="rules.md" """, h => h.Resources["rules.md"] = Encoding.UTF8.GetBytes("# From the file"));
        Assert.Equal("# From the file", fromFile.Markdown);
        var (_, missing, _) = Markdown("""source="gone.md" """);
        Assert.Equal("", missing.Markdown);
        var (_, fromKey, _) = Markdown("""caption="@rulesText" """, h => h.Texts["rulesText"] = "Play *fair*.");
        Assert.Equal("Play *fair*.", fromKey.Markdown);
        var (screen, bound, host) = Markdown("""bind="game.notes" """, h => h.Values["game.notes"] = "One");
        Assert.Equal("One", bound.Markdown);
        host.Values["game.notes"] = "Two";
        host.Change();
        Assert.Equal("Two", bound.Markdown);
        Shot.Of(screen, 800, 400);
        Assert.Equal("Two", Assert.Single(bound.Page.Runs).Text.Trim());
    }

    [Fact]
    public void It_shows_headings_paragraphs_items_and_code()
    {
        var (_, part, _) = Markdown("""bind="notes" """, h => h.Values["notes"] = "# Rules\n\nSee [the site](https://example.com) **now** and *then*.\n\n- one\n1. two\n\n```\nlet x = 1\n```");
        var runs = part.Page.Runs;
        var heading = runs.First(r => r.Text.StartsWith("Rules"));
        var plain = runs.First(r => r.Text.StartsWith("See"));
        Assert.True(heading.Bold);
        Assert.True(heading.Size > plain.Size);
        Assert.Equal("See the site now and then.", string.Concat(runs.Where(r => r.Box.Y == plain.Box.Y).Select(r => r.Text)).Trim());
        Assert.Equal("https://example.com", runs.Single(r => r.Text == "the ").Link);
        Assert.Equal("https://example.com", runs.Single(r => r.Text.StartsWith("site")).Link);
        Assert.True(runs.Single(r => r.Text.StartsWith("now")).Bold);
        Assert.True(runs.Single(r => r.Text.StartsWith("then")).Italic);
        Assert.Contains(runs, r => r.Text == "•");
        Assert.Contains(runs, r => r.Text == "1.");
        var code = runs.Single(r => r.Text.StartsWith("let"));
        Assert.True(code.Code);
        Assert.Contains(part.Page.CodeBoxes, b => b.Contains(code.Box.X + 1, code.Box.Y + 1));
        Assert.True(runs.Single(r => r.Text.StartsWith("one")).Box.X > plain.Box.X);
    }

    [Fact]
    public void A_long_paragraph_wraps_to_the_width_and_a_long_text_scrolls()
    {
        var words = string.Join(" ", Enumerable.Repeat("squares", 200));
        var (screen, part, _) = Markdown("""bind="notes" """, h => h.Values["notes"] = words, width: 400, height: 300);
        var runs = part.Page.Runs;
        Assert.True(runs.Select(r => r.Box.Y).Distinct().Count() > 3);
        Assert.All(runs, r => Assert.True(r.Box.Right <= part.Bounds.Width + 0.5, $"{r.Box} beyond {part.Bounds.Width}"));
        Assert.True(part.Scroll.MaxScrollY > 0);
        screen.Wheel(part.Bounds.X + 10, part.Bounds.Y + 10, 0, 40);
        Assert.Equal(40, part.Scroll.ScrollY);
    }

    [Fact]
    public void A_link_opens_through_the_host_and_plain_text_does_not()
    {
        var (screen, part, host) = Markdown("""bind="notes" """, h => h.Values["notes"] = "See [the site](https://example.com) now.");
        void Click(MarkdownRun run)
        {
            var (x, y) = (part.Page.Bounds.X + run.Box.X + 2, part.Page.Bounds.Y + run.Box.Y + run.Box.Height / 2);
            screen.PointerPressed(x, y, PointerButton.Left);
            screen.PointerReleased(x, y, PointerButton.Left);
        }
        Click(part.Page.Runs.First(r => r.Text.StartsWith("See")));
        Assert.Empty(host.Opened);
        Click(part.Page.Runs.First(r => r.Link is not null));
        Assert.Equal(["https://example.com"], host.Opened);
    }

    [Fact]
    public void A_link_is_drawn_in_the_accent()
    {
        var (screen, part, _) = Markdown("""bind="notes" """, h => h.Values["notes"] = "[Rules](https://example.com)");
        var shot = Shot.Of(screen, 800, 400);
        var run = part.Page.Runs.Single();
        var accent = ThemeLook.Clarity(dark: false).Color("accent");
        var found = false;
        for (var x = (int)(part.Page.Bounds.X + run.Box.X); x < part.Page.Bounds.X + run.Box.Right; x++)
            for (var y = (int)(part.Page.Bounds.Y + run.Box.Y); y < part.Page.Bounds.Y + run.Box.Bottom; y++)
                found |= shot.GetPixel(x, y) == accent;
        Assert.True(found);
    }
}
