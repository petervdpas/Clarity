// A window's grid in its client area (WindowClient): as high as the window,
// so rows that grow share its height and a long list scrolls in its own
// rows; a window lower than the rows at their least scrolls by that much.

using Avalonia;
using Avalonia.Controls;
using Avalonia.Controls.Primitives;
using Avalonia.Headless.XUnit;
using Avalonia.Layout;
using Clarity.Avalonia;
using Xunit;

namespace Clarity.Avalonia.Tests;

public class WindowClientTests
{
    /// A grid as a window has it: a list in a row that grows, and a row of buttons below.
    static (Window window, ScrollViewer scroll, Grid grid, ListBox list, Button button) Shown(double height)
    {
        var list = new ListBox { ItemsSource = Enumerable.Range(1, 200).Select(i => $"Item {i}").ToList() };
        var button = new Button { Content = "Save", VerticalAlignment = VerticalAlignment.Stretch };
        var grid = new Grid { RowDefinitions = new RowDefinitions("*,36"), RowSpacing = 8, Margin = new Thickness(12) };
        grid.RowDefinitions[0].MinHeight = 36 * 4;
        Grid.SetRow(button, 1);
        grid.Children.Add(list);
        grid.Children.Add(button);
        var scroll = new ScrollViewer { HorizontalScrollBarVisibility = ScrollBarVisibility.Disabled, VerticalScrollBarVisibility = ScrollBarVisibility.Auto, Content = new WindowClient { Child = grid } };
        var window = new Window { Width = 600, Height = height, Content = scroll };
        window.Show();
        window.UpdateLayout();
        return (window, scroll, grid, list, button);
    }

    [AvaloniaFact]
    public void The_grid_is_as_high_as_the_window_and_a_long_list_scrolls_in_its_rows()
    {
        var (window, scroll, grid, list, button) = Shown(500);
        Assert.Equal(500, scroll.Extent.Height, 1);
        Assert.Equal(500 - 24 - 36 - 8, list.Bounds.Height, 1);
        Assert.Equal(36, button.Bounds.Height, 1);
        window.Close();
    }

    [AvaloniaFact]
    public void A_window_lower_than_the_rows_at_their_least_scrolls_by_that_much()
    {
        var (window, scroll, _, list, button) = Shown(120);
        var least = 12 + 36 * 4 + 8 + 36 + 12;
        Assert.Equal(least, scroll.Extent.Height, 1);
        Assert.Equal(36 * 4, list.Bounds.Height, 1);
        Assert.Equal(36, button.Bounds.Height, 1);
        window.Close();
    }
}
