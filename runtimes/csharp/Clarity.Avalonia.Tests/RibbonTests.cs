// A ribbon (conformance/ribbons.json): a tab control whose open tab, counted
// from 1, and folding follow its part state. Folded, no tab is open and only
// the tab strip shows; opening a tab unfolds it.

using System.Text.Json;
using Avalonia.Controls;
using Avalonia.Headless.XUnit;
using Clarity.Avalonia;
using Xunit;

namespace Clarity.Avalonia.Tests;

public class RibbonTests
{
    [Fact]
    public void ARibbonShowsTheTabAsEveryRendererDoes()
    {
        var file = JsonDocument.Parse(File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "ribbons.json")));
        var cases = file.RootElement.GetProperty("shown").EnumerateArray().ToList();
        Assert.NotEmpty(cases);
        foreach (var c in cases)
        {
            var visible = c.GetProperty("visible").EnumerateArray().Select(v => v.GetBoolean()).ToArray();
            var shown = c.GetProperty("shown");
            Assert.Equal(shown.ValueKind == JsonValueKind.Null ? null : shown.GetInt64(), RibbonView.ShownTab(visible, c.GetProperty("tab").GetInt64()));
        }
    }

    [AvaloniaFact]
    public void ARibbonOpensItsTabFoldsAndUnfolds()
    {
        var ribbon = new RibbonView { Collapsible = true, Items = { new TabItem { Header = "Home" }, new TabItem { Header = "View" } } };
        var window = new Window { Content = ribbon };
        window.Show();
        ribbon.Tab = 2;
        Assert.Equal(1, ribbon.SelectedIndex);
        ribbon.IsCollapsed = true;
        Assert.Equal(-1, ribbon.SelectedIndex);
        // The person opens a tab of the folded ribbon: it unfolds there.
        ribbon.SelectedIndex = 0;
        Assert.Equal((false, 1L), (ribbon.IsCollapsed, ribbon.Tab));
    }

    [Fact]
    public void ARibbonsPartStateCarriesItsOpenTab()
    {
        var ribbon = new PartState("mainRibbon");
        Assert.Equal(1, ribbon.Tab);
        var changed = new List<string?>();
        ribbon.PropertyChanged += (_, e) => changed.Add(e.PropertyName);
        ribbon.Tab = 2;
        Assert.Equal(["Tab"], changed);
    }
}
