// The markdown part: Markdown read into blocks and spans against the cases
// every renderer shares (conformance/markdown.json), the file of the
// language shown, and the view that shows the blocks and opens links.

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

    /// A block as the conformance file writes it: only what holds is listed.
    static object Written(MarkdownBlock block) => block.Kind switch
    {
        "heading" => new Dictionary<string, object?> { ["kind"] = "heading", ["level"] = block.Level, ["spans"] = block.Spans.Select(Written).ToList() },
        "paragraph" => new Dictionary<string, object?> { ["kind"] = "paragraph", ["spans"] = block.Spans.Select(Written).ToList() },
        "item" when block.Number is int number => new Dictionary<string, object?> { ["kind"] = "item", ["depth"] = block.Depth, ["ordered"] = true, ["number"] = number, ["spans"] = block.Spans.Select(Written).ToList() },
        "item" => new Dictionary<string, object?> { ["kind"] = "item", ["depth"] = block.Depth, ["ordered"] = false, ["spans"] = block.Spans.Select(Written).ToList() },
        _ => new Dictionary<string, object?> { ["kind"] = "code", ["text"] = block.Text },
    };

    static Dictionary<string, object?> Written(MarkdownSpan span)
    {
        var written = new Dictionary<string, object?> { ["text"] = span.Text };
        if (span.Bold) written["bold"] = true;
        if (span.Italic) written["italic"] = true;
        if (span.Code) written["code"] = true;
        if (span.Link is not null) written["link"] = span.Link;
        return written;
    }

    static string Canonical(JsonElement element) => JsonSerializer.Serialize(JsonSerializer.Deserialize<JsonElement>(element.GetRawText()));

    [Fact]
    public void MarkdownReadsAsEveryRendererReadsIt()
    {
        foreach (var c in File.GetProperty("cases").EnumerateArray())
        {
            var blocks = MarkdownBlocks.Read(c.GetProperty("markdown").GetString()!).Select(Written).ToList();
            Assert.True(Canonical(c.GetProperty("blocks")) == JsonSerializer.Serialize(blocks), $"{c.GetProperty("name").GetString()}: {JsonSerializer.Serialize(blocks)}");
        }
    }

    [Fact]
    public void AFileShowsInTheLanguageShownElseTheDefaultOnes()
    {
        var have = new HashSet<string> { "rules.md", "rules.nl.md" };
        Assert.Equal("rules.nl.md", MarkdownView.FileFor("rules.md", "nl", have.Contains));
        Assert.Equal("rules.md", MarkdownView.FileFor("rules.md", "de", have.Contains));
        Assert.Equal("rules.md", MarkdownView.FileFor("rules.md", "en", have.Contains));
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
