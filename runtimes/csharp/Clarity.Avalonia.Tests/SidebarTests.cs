// A sidebar docked to a window's edge (conformance/sidebars.json): the
// person drags its inner edge between its min and max, folds it to a narrow
// strip with a button, and lets it fill the window; its part state carries
// whether it is folded and whether it fills the window, for the code.

using System.Text.Json;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Headless.XUnit;
using Clarity.Avalonia;
using Xunit;

namespace Clarity.Avalonia.Tests;

public class SidebarTests
{
    [Fact]
    public void ASidebarIsDraggedWithinItsLimitsAsEveryRendererDoes()
    {
        var file = JsonDocument.Parse(File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "conformance", "sidebars.json")));
        var cases = file.RootElement.GetProperty("widths").EnumerateArray().ToList();
        Assert.NotEmpty(cases);
        double? Limit(JsonElement c, string key) => c.GetProperty(key).ValueKind == JsonValueKind.Null ? null : c.GetProperty(key).GetDouble();
        foreach (var c in cases)
            Assert.Equal(c.GetProperty("width").GetDouble(), SidebarView.WidthFor(c.GetProperty("dragged").GetDouble(), Limit(c, "min"), Limit(c, "max")));
    }

    static (Window, SidebarView, Border) Shown(SidebarView sidebar)
    {
        var main = new Border();
        DockPanel.SetDock(sidebar, Dock.Left);
        var dock = new DockPanel { Children = { sidebar, main } };
        var window = new Window { Width = 1000, Height = 600, Content = dock };
        window.Show();
        return (window, sidebar, main);
    }

    [AvaloniaFact]
    public void ASidebarIsItsWidthFoldsToAStripAndFillsTheWindow()
    {
        var (_, sidebar, main) = Shown(new SidebarView { SidebarWidth = 280, Collapsible = true, Maximizable = true, Content = new Border() });
        Assert.Equal(280, sidebar.Bounds.Width);
        sidebar.IsCollapsed = true;
        sidebar.UpdateLayout();
        Assert.Equal(SidebarView.StripWidth, sidebar.Bounds.Width);
        sidebar.IsCollapsed = false;
        sidebar.IsMaximized = true;
        sidebar.UpdateLayout();
        // It takes the whole width; the window's own parts get none.
        Assert.Equal(1000, sidebar.Bounds.Width);
        Assert.Equal(0, main.Bounds.Width);
    }

    [Fact]
    public void ASidebarsPartStateCarriesWhetherItIsFoldedAndFillsTheWindow()
    {
        var nav = new PartState("nav");
        var changed = new List<string?>();
        nav.PropertyChanged += (_, e) => changed.Add(e.PropertyName);
        nav.IsCollapsed = true;
        nav.IsMaximized = true;
        Assert.Equal(["IsCollapsed", "IsMaximized"], changed);
    }
}
