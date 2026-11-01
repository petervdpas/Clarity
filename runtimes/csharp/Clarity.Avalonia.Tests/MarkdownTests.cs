// The markdown part: Markdown read into blocks and spans against the cases
// every renderer shares (conformance/markdown.json), the resource a source
// names, and the view that shows the blocks and opens links.

using System.Text.Json;
using Avalonia.Controls;
using Avalonia.Controls.Documents;
using Avalonia.Headless.XUnit;
using Avalonia.Media;
using Clarity.Avalonia;
using Xunit;

namespace Clarity.Avalonia.Tests;

public class MarkdownTests
{
    static readonly JsonElement File = JsonDocument.Parse(System.IO.File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "markdown.json"))).RootElement;

    // Avalonia registers the avares scheme, which keeps the assembly name's case.
    [AvaloniaFact]
    public void ASourceIsTheResourceItNamesWhateverTheLanguage()
    {
        // Which file a language shows is up to its key, not the file's name.
        Assert.Equal("avares://Shop/resources/rules.md", MarkdownView.ResourceUri("Shop", "rules.md").ToString());
        Assert.Equal("avares://Shop/resources/rules.nl.md", MarkdownView.ResourceUri("Shop", "rules.nl.md").ToString());
    }

    [AvaloniaFact]
    public void TheViewShowsABlockEachAndKnowsWhereItsLinksAre()
    {
        var view = new MarkdownView { Markdown = "# Rules\n\nSee [the site](https://example.com) **now**.\n\n- one\n\n```\ncode\n```" };
        var window = new Window { Width = 400, Height = 300, Content = view };
        window.Show();
        var blocks = view.Blocks.Children;
        Assert.Equal(4, blocks.Count);
        var heading = Assert.IsType<TextBlock>(blocks[0]);
        Assert.Equal(FontWeight.SemiBold, heading.FontWeight);
        var paragraph = Assert.IsType<TextBlock>(blocks[1]);
        Assert.Equal("See the site now.", string.Concat(paragraph.Inlines!.OfType<Run>().Select(r => r.Text)));
        Assert.True(paragraph.Inlines!.OfType<Run>().Single(r => r.Text == "now").FontWeight == FontWeight.SemiBold);
        // "See " is 4 characters: the link runs from 4 to 12.
        Assert.Null(MarkdownView.LinkAt(paragraph, 3));
        Assert.Equal("https://example.com", MarkdownView.LinkAt(paragraph, 4));
        Assert.Equal("https://example.com", MarkdownView.LinkAt(paragraph, 11));
        Assert.Null(MarkdownView.LinkAt(paragraph, 12));
        Assert.IsType<Grid>(blocks[2]);
        Assert.IsType<Border>(blocks[3]);
    }
}
