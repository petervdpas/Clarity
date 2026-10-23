// Containers that line up (U1b): a panel's frame, and a foldable panel's
// (an Expander), sit 3 pixels out into the space between columns on each
// side, with the padding the generated theme gives them (ClarityFramePadding:
// 3 less the border, each side), so the grid inside has the window's column
// lines. Laid out by Avalonia's own Grid and Fluent's Expander.

using Avalonia;
using Avalonia.Controls;
using Avalonia.Headless.XUnit;
using Avalonia.Threading;
using Avalonia.VisualTree;
using Xunit;

namespace Clarity.Avalonia.Tests;

public class GridLineTests
{
    const double Gap = 8;
    const double Border = 1;

    static Grid Columns(int count, params Control[] children)
    {
        var grid = new Grid { ColumnDefinitions = new ColumnDefinitions(string.Join(",", Enumerable.Repeat("*", count))), ColumnSpacing = Gap };
        foreach (var child in children)
            grid.Children.Add(child);
        return grid;
    }

    /// The left edges of a grid's columns, in the window.
    static List<double> Lines(Grid grid, Window window, int count) =>
        Enumerable.Range(0, count).Select(i =>
        {
            var probe = grid.Children.OfType<Border>().First(b => Grid.GetColumn(b) == i && b.Tag as string == "probe");
            return Math.Round(probe.TranslatePoint(new Point(0, 0), window)!.Value.X, 1);
        }).ToList();

    static Border Probe(int column) => new() { Tag = "probe", Height = 4, [Grid.ColumnProperty] = column };

    void LinesUp(Func<Control, Control> frame)
    {
        var inner = Columns(6, Enumerable.Range(0, 6).Select(Probe).ToArray());
        var framed = frame(inner);
        Grid.SetColumnSpan(framed, 6);
        Grid.SetRow(framed, 1);
        var outer = Columns(12, Enumerable.Range(0, 12).Select(Probe).Append(framed).ToArray());
        outer.RowDefinitions = new RowDefinitions("Auto,Auto");
        var window = new Window { Width = 960 + 11 * Gap, Height = 400, Content = outer };
        window.Show();
        Dispatcher.UIThread.RunJobs();
        Assert.Equal(Lines(outer, window, 12).Take(6).ToList(), Lines(inner, window, 6));
        window.Close();
    }

    [AvaloniaFact]
    public void A_panels_grid_has_the_windows_column_lines()
    {
        LinesUp(inner => new global::Avalonia.Controls.Border
        {
            Margin = new Thickness(-3, 0),
            BorderThickness = new Thickness(Border),
            Padding = new Thickness(3 - Border, 8),
            Child = inner,
        });
    }

    [AvaloniaFact]
    public void A_foldable_panels_grid_has_the_windows_column_lines()
    {
        LinesUp(inner => new Expander
        {
            Margin = new Thickness(-3, 0),
            Padding = new Thickness(3 - Border, 8),
            IsExpanded = true,
            Header = "Address",
            HorizontalAlignment = global::Avalonia.Layout.HorizontalAlignment.Stretch,
            Content = inner,
        });
    }

    [AvaloniaFact]
    public void A_tab_pages_grid_has_the_windows_column_lines()
    {
        // As the generated style TabControl.tabs sets it: no padding around the page.
        LinesUp(inner => new TabControl { Padding = new Thickness(0), Items = { new TabItem { Header = "Orders", Content = inner } } });
    }
}
